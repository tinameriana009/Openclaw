from __future__ import annotations

import math
import re
from collections import Counter
from dataclasses import dataclass
from functools import lru_cache

from .commands import PORTED_COMMANDS
from .context import PortContext, build_port_context, render_context
from .execution_registry import build_execution_registry
from .history import HistoryLog
from .models import PermissionDenial, PortingModule
from .query_engine import QueryEngineConfig, QueryEnginePort, TurnResult
from .setup import SetupReport, WorkspaceSetup, run_setup
from .system_init import build_system_init_message
from .tools import PORTED_TOOLS


SEMANTIC_SYNONYMS: dict[str, tuple[str, ...]] = {
    'review': ('audit', 'inspect', 'security', 'check'),
    'audit': ('review', 'inspect', 'security', 'check'),
    'search': ('find', 'lookup', 'query', 'discover'),
    'find': ('search', 'lookup', 'query', 'discover'),
    'tool': ('tools', 'utility'),
    'tools': ('tool', 'utilities'),
    'agent': ('agents', 'assistant'),
    'agents': ('agent', 'assistants'),
    'session': ('sessions', 'history', 'transcript'),
    'sessions': ('session', 'history', 'transcript'),
    'mcp': ('resource', 'resources', 'server', 'protocol'),
}
STOPWORDS = frozenset({'a', 'an', 'and', 'for', 'from', 'how', 'i', 'in', 'of', 'on', 'the', 'to', 'with'})
TOKEN_PATTERN = re.compile(r'[A-Z]+(?=[A-Z][a-z]|\b)|[A-Z]?[a-z]+|\d+')


@dataclass(frozen=True)
class RoutedMatch:
    kind: str
    name: str
    source_hint: str
    score: int


@dataclass(frozen=True)
class NextStepAction:
    title: str
    rationale: str
    command: str
    bucket: str = 'next'


@dataclass(frozen=True)
class OperatorFlowPlan:
    headline: str
    actions: tuple[NextStepAction, ...]

    def as_markdown(self) -> str:
        if not self.actions:
            return 'No concrete next-step actions available.'
        lines = [self.headline, '']
        current_bucket: str | None = None
        for action in self.actions:
            if action.bucket != current_bucket:
                current_bucket = action.bucket
                lines.extend([f'### {current_bucket}', ''])
            lines.append(f'- **{action.title}** — {action.rationale}')
            lines.append(f'  - `{action.command}`')
        return '\n'.join(lines)


@dataclass
class RuntimeSession:
    prompt: str
    context: PortContext
    setup: WorkspaceSetup
    setup_report: SetupReport
    system_init_message: str
    history: HistoryLog
    routed_matches: list[RoutedMatch]
    turn_result: TurnResult
    command_execution_messages: tuple[str, ...]
    tool_execution_messages: tuple[str, ...]
    stream_events: tuple[dict[str, object], ...]
    persisted_session_path: str
    operator_flow: OperatorFlowPlan

    def as_markdown(self) -> str:
        lines = [
            '# Runtime Session',
            '',
            f'Prompt: {self.prompt}',
            '',
            '## Context',
            render_context(self.context),
            '',
            '## Setup',
            f'- Python: {self.setup.python_version} ({self.setup.implementation})',
            f'- Platform: {self.setup.platform_name}',
            f'- Test command: {self.setup.test_command}',
            '',
            '## Startup Steps',
            *(f'- {step}' for step in self.setup.startup_steps()),
            '',
            '## System Init',
            self.system_init_message,
            '',
            '## Routed Matches',
        ]
        if self.routed_matches:
            lines.extend(
                f'- [{match.kind}] {match.name} ({match.score}) — {match.source_hint}'
                for match in self.routed_matches
            )
        else:
            lines.append('- none')
        lines.extend([
            '',
            '## Command Execution',
            *(self.command_execution_messages or ('none',)),
            '',
            '## Tool Execution',
            *(self.tool_execution_messages or ('none',)),
            '',
            '## Stream Events',
            *(f"- {event['type']}: {event}" for event in self.stream_events),
            '',
            '## Turn Result',
            self.turn_result.output,
            '',
            '## Operator Flow',
            self.operator_flow.as_markdown(),
            '',
            f'Persisted session path: {self.persisted_session_path}',
            '',
            self.history.as_markdown(),
        ])
        return '\n'.join(lines)


class PortRuntime:
    def route_prompt(self, prompt: str, limit: int = 5) -> list[RoutedMatch]:
        query_tokens = self._normalize_text(prompt)
        ordered_query_tokens = tuple(query_tokens)
        by_kind = {
            'command': self._collect_matches(query_tokens, PORTED_COMMANDS, 'command', ordered_query_tokens),
            'tool': self._collect_matches(query_tokens, PORTED_TOOLS, 'tool', ordered_query_tokens),
        }

        selected: list[RoutedMatch] = []
        family_counts: Counter[tuple[str, str]] = Counter()
        for kind in ('command', 'tool'):
            if by_kind[kind]:
                match = by_kind[kind].pop(0)
                selected.append(match)
                family_counts[(kind, self._family_key(match.source_hint))] += 1

        leftovers = sorted(
            [match for matches in by_kind.values() for match in matches],
            key=lambda item: (-item.score, item.kind, item.name),
        )
        for match in leftovers:
            if len(selected) >= limit:
                break
            family_key = (match.kind, self._family_key(match.source_hint))
            if family_counts[family_key] >= 2:
                continue
            selected.append(match)
            family_counts[family_key] += 1
        return selected[:limit]

    def plan_operator_flow(
        self,
        prompt: str,
        matches: list[RoutedMatch],
        *,
        persisted_session_path: str | None = None,
        session_id: str | None = None,
        transcript_entries: tuple[str, ...] = (),
        include_handoff: bool = True,
    ) -> OperatorFlowPlan:
        matched_commands = [match.name for match in matches if match.kind == 'command']
        matched_tools = [match.name for match in matches if match.kind == 'tool']
        actions: list[NextStepAction] = []
        session_hint = session_id or 'latest'
        prompt_tokens = set(self._normalize_text(prompt))

        if matches:
            top_matches = ', '.join(match.name for match in matches[:3])
            actions.append(NextStepAction(
                title='Review the surfaced command/tool path',
                rationale=f'The router currently thinks the strongest path is: {top_matches}. Confirm that before going deeper.',
                command=f'python -m src.main route {prompt!r} --limit 8',
                bucket='Now',
            ))
        else:
            actions.append(NextStepAction(
                title='Refine the ask before iterating',
                rationale='No mirrored command or tool stood out. Narrow the prompt to a subsystem, workflow, or artifact first.',
                command='python -m src.main commands --query "<narrower topic>" --limit 8',
                bucket='Now',
            ))

        if matched_commands:
            actions.append(NextStepAction(
                title='Inspect the top command surface',
                rationale='This keeps the operator grounded in a concrete command entry instead of jumping straight into a vague follow-up.',
                command=f'python -m src.main show-command {matched_commands[0]}',
                bucket='Inspect',
            ))
        if matched_tools:
            actions.append(NextStepAction(
                title='Inspect the top tool surface',
                rationale='Check whether the likely tool match actually corresponds to the workflow you want before treating it as evidence.',
                command=f'python -m src.main show-tool {matched_tools[0]}',
                bucket='Inspect',
            ))

        if persisted_session_path or session_id:
            actions.append(NextStepAction(
                title='Resume the same operator thread',
                rationale='Stay on the same saved session so replay/review/resume remain one continuous trail.',
                command=f'python -m src.main load-session {session_hint}',
                bucket='Continue',
            ))

        if transcript_entries:
            actions.append(NextStepAction(
                title='Replay the saved operator context',
                rationale=f'{len(transcript_entries)} prompt(s) are already in the saved transcript. Replaying them is the honest way to re-enter context.',
                command=f'python -m src.main next-steps --session-id {session_hint}',
                bucket='Continue',
            ))

        if prompt_tokens & {'review', 'audit', 'handoff', 'resume', 'replay', 'session', 'trace'}:
            actions.append(NextStepAction(
                title='Capture the next verification move explicitly',
                rationale='This workflow is strongest when the operator names the next manual validation or evidence-gathering step instead of assuming completion.',
                command=f'python -m src.main bootstrap {prompt!r} --limit 5',
                bucket='Validate',
            ))

        if include_handoff:
            actions.append(NextStepAction(
                title='Prepare a bounded handoff',
                rationale='If another operator takes over, pass the saved session id and routed surfaces instead of a loose summary.',
                command=f'python -m src.main next-steps --session-id {session_hint}',
                bucket='Handoff',
            ))

        headline = 'Recommended next operator moves for this saved run' if persisted_session_path or session_id else 'Recommended next operator moves for this prompt'
        return OperatorFlowPlan(headline=headline, actions=tuple(actions))

    def bootstrap_session(self, prompt: str, limit: int = 5) -> RuntimeSession:
        context = build_port_context()
        setup_report = run_setup(trusted=True)
        setup = setup_report.setup
        history = HistoryLog()
        engine = QueryEnginePort.from_workspace()
        history.add('context', f'python_files={context.python_file_count}, archive_available={context.archive_available}')
        history.add('registry', f'commands={len(PORTED_COMMANDS)}, tools={len(PORTED_TOOLS)}')
        matches = self.route_prompt(prompt, limit=limit)
        registry = build_execution_registry()
        command_execs = tuple(registry.command(match.name).execute(prompt) for match in matches if match.kind == 'command' and registry.command(match.name))
        tool_execs = tuple(registry.tool(match.name).execute(prompt) for match in matches if match.kind == 'tool' and registry.tool(match.name))
        denials = tuple(self._infer_permission_denials(matches))
        stream_events = tuple(engine.stream_submit_message(
            prompt,
            matched_commands=tuple(match.name for match in matches if match.kind == 'command'),
            matched_tools=tuple(match.name for match in matches if match.kind == 'tool'),
            denied_tools=denials,
        ))
        turn_result = engine.submit_message(
            prompt,
            matched_commands=tuple(match.name for match in matches if match.kind == 'command'),
            matched_tools=tuple(match.name for match in matches if match.kind == 'tool'),
            denied_tools=denials,
        )
        persisted_session_path = engine.persist_session()
        operator_flow = self.plan_operator_flow(
            prompt,
            matches,
            persisted_session_path=persisted_session_path,
            session_id=engine.session_id,
            transcript_entries=engine.replay_user_messages(),
        )
        history.add('routing', f'matches={len(matches)} for prompt={prompt!r}')
        history.add('execution', f'command_execs={len(command_execs)} tool_execs={len(tool_execs)}')
        history.add('turn', f'commands={len(turn_result.matched_commands)} tools={len(turn_result.matched_tools)} denials={len(turn_result.permission_denials)} stop={turn_result.stop_reason}')
        history.add('operator_flow', f'actions={len(operator_flow.actions)}')
        history.add('session_store', persisted_session_path)
        return RuntimeSession(
            prompt=prompt,
            context=context,
            setup=setup,
            setup_report=setup_report,
            system_init_message=build_system_init_message(trusted=True),
            history=history,
            routed_matches=matches,
            turn_result=turn_result,
            command_execution_messages=command_execs,
            tool_execution_messages=tool_execs,
            stream_events=stream_events,
            persisted_session_path=persisted_session_path,
            operator_flow=operator_flow,
        )

    def run_turn_loop(self, prompt: str, limit: int = 5, max_turns: int = 3, structured_output: bool = False) -> list[TurnResult]:
        engine = QueryEnginePort.from_workspace()
        engine.config = QueryEngineConfig(max_turns=max_turns, structured_output=structured_output)
        matches = self.route_prompt(prompt, limit=limit)
        command_names = tuple(match.name for match in matches if match.kind == 'command')
        tool_names = tuple(match.name for match in matches if match.kind == 'tool')
        results: list[TurnResult] = []
        for turn in range(max_turns):
            turn_prompt = prompt if turn == 0 else f'{prompt} [turn {turn + 1}]'
            result = engine.submit_message(turn_prompt, command_names, tool_names, ())
            results.append(result)
            if result.stop_reason != 'completed':
                break
        return results

    def _infer_permission_denials(self, matches: list[RoutedMatch]) -> list[PermissionDenial]:
        denials: list[PermissionDenial] = []
        for match in matches:
            if match.kind == 'tool' and 'bash' in match.name.lower():
                denials.append(PermissionDenial(tool_name=match.name, reason='destructive shell execution remains gated in the Python port'))
        return denials

    def _collect_matches(
        self,
        query_tokens: Counter[str],
        modules: tuple[PortingModule, ...],
        kind: str,
        ordered_query_tokens: tuple[str, ...],
    ) -> list[RoutedMatch]:
        matches: list[RoutedMatch] = []
        for module in modules:
            score = self._score(query_tokens, module, ordered_query_tokens, kind)
            if score > 0:
                matches.append(RoutedMatch(kind=kind, name=module.name, source_hint=module.source_hint, score=score))
        matches.sort(key=lambda item: (-item.score, item.name))
        return matches

    @classmethod
    def _normalize_text(cls, text: str) -> Counter[str]:
        normalized: Counter[str] = Counter()
        lowered = text.replace('/', ' ').replace('\\', ' ').replace('-', ' ').replace('_', ' ')
        for chunk in lowered.split():
            for token in cls._split_token(chunk):
                token = token.lower().strip()
                if not token or token in STOPWORDS:
                    continue
                normalized[token] += 1
                singular = cls._singularize(token)
                if singular != token and singular not in STOPWORDS:
                    normalized[singular] += 1
                for synonym in SEMANTIC_SYNONYMS.get(token, ()):  # lightweight semantic expansion
                    if synonym not in STOPWORDS:
                        normalized[synonym] += 1
        return normalized

    @staticmethod
    def _split_token(token: str) -> tuple[str, ...]:
        parts = TOKEN_PATTERN.findall(token)
        return tuple(parts) if parts else (token,)

    @staticmethod
    def _singularize(token: str) -> str:
        if len(token) <= 3:
            return token
        if token.endswith('ies'):
            return f'{token[:-3]}y'
        if token.endswith('ses'):
            return token[:-2]
        if token.endswith('s') and not token.endswith('ss'):
            return token[:-1]
        return token

    @classmethod
    def _score(
        cls,
        query_tokens: Counter[str],
        module: PortingModule,
        ordered_query_tokens: tuple[str, ...],
        kind: str,
    ) -> int:
        name_tokens = cls._normalize_text(module.name)
        path_tokens = cls._normalize_text(module.source_hint)
        responsibility_tokens = cls._normalize_text(module.responsibility)
        basename_tokens = cls._normalize_text(cls._basename_without_extension(module.source_hint))
        source_parts = [part for part in module.source_hint.split('/') if part]
        parent_tokens = cls._normalize_text(source_parts[-2]) if len(source_parts) >= 2 else Counter()
        evidence_sets = (name_tokens, basename_tokens, parent_tokens, path_tokens, responsibility_tokens)
        corpus_stats = cls._corpus_stats()
        score = 0
        matched_sets = 0

        for evidence in evidence_sets:
            evidence_score = 0
            for token, weight in query_tokens.items():
                token_weight = cls._token_rarity_weight(token, corpus_stats)
                if token in evidence:
                    evidence_score += math.ceil(5 * token_weight) * min(weight, evidence[token])
                    continue
                if any(token in candidate or candidate in token for candidate in evidence):
                    evidence_score += max(1, math.ceil(2 * token_weight))
            if evidence_score:
                matched_sets += 1
                score += evidence_score

        overlap = set(query_tokens) & set(name_tokens)
        score += sum(math.ceil(4 * cls._token_rarity_weight(token, corpus_stats)) for token in overlap)

        query_bigrams = cls._bigrams(query_tokens)
        if query_bigrams:
            score += 3 * len(query_bigrams & cls._bigrams(name_tokens))
            score += 3 * len(query_bigrams & cls._bigrams(basename_tokens))
            score += 2 * len(query_bigrams & cls._bigrams(path_tokens))

        if matched_sets >= 2:
            score += 4
        if matched_sets >= 4:
            score += 4

        if set(query_tokens) & set(parent_tokens):
            score += 6
        if set(query_tokens) & set(basename_tokens):
            score += 10

        if name_tokens and any(token == module.name.lower() for token in query_tokens):
            score += 6

        if ordered_query_tokens:
            lead_token = ordered_query_tokens[0]
            if lead_token in name_tokens:
                score += 18 if kind == 'command' else 8
            elif lead_token in basename_tokens:
                score += 12 if kind == 'command' else 8
            elif lead_token in path_tokens:
                score += 8 if kind == 'command' else 4

        return score

    @staticmethod
    def _bigrams(tokens: Counter[str]) -> set[str]:
        ordered = list(tokens)
        return {f'{ordered[index]}::{ordered[index + 1]}' for index in range(len(ordered) - 1)}

    @staticmethod
    def _basename_without_extension(source_hint: str) -> str:
        basename = source_hint.rsplit('/', 1)[-1]
        return basename.split('.', 1)[0]

    @staticmethod
    def _family_key(source_hint: str) -> str:
        parts = [part for part in source_hint.split('/') if part]
        if len(parts) >= 2:
            return '/'.join(parts[:2])
        return source_hint

    @classmethod
    @lru_cache(maxsize=1)
    def _corpus_stats(cls) -> Counter[str]:
        stats: Counter[str] = Counter()
        for module in (*PORTED_COMMANDS, *PORTED_TOOLS):
            tokens = set(cls._normalize_text(module.name))
            tokens |= set(cls._normalize_text(module.source_hint))
            tokens |= set(cls._normalize_text(module.responsibility))
            for token in tokens:
                stats[token] += 1
        return stats

    @staticmethod
    def _token_rarity_weight(token: str, corpus_stats: Counter[str]) -> float:
        doc_freq = corpus_stats.get(token, 0)
        if doc_freq <= 1:
            return 2.4
        if doc_freq <= 3:
            return 1.9
        if doc_freq <= 8:
            return 1.5
        if doc_freq <= 20:
            return 1.2
        return 1.0
