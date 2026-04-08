from __future__ import annotations

import re
from collections import Counter
from dataclasses import dataclass

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
        for kind in ('command', 'tool'):
            if by_kind[kind]:
                selected.append(by_kind[kind].pop(0))

        leftovers = sorted(
            [match for matches in by_kind.values() for match in matches],
            key=lambda item: (-item.score, item.kind, item.name),
        )
        selected.extend(leftovers[: max(0, limit - len(selected))])
        return selected[:limit]

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
        history.add('routing', f'matches={len(matches)} for prompt={prompt!r}')
        history.add('execution', f'command_execs={len(command_execs)} tool_execs={len(tool_execs)}')
        history.add('turn', f'commands={len(turn_result.matched_commands)} tools={len(turn_result.matched_tools)} denials={len(turn_result.permission_denials)} stop={turn_result.stop_reason}')
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
        evidence_sets = (name_tokens, path_tokens, responsibility_tokens)
        score = 0
        matched_sets = 0

        for evidence in evidence_sets:
            evidence_score = 0
            for token, weight in query_tokens.items():
                if token in evidence:
                    evidence_score += 5 * min(weight, evidence[token])
                    continue
                if any(token in candidate or candidate in token for candidate in evidence):
                    evidence_score += 2
            if evidence_score:
                matched_sets += 1
                score += evidence_score

        overlap = set(query_tokens) & set(name_tokens)
        score += len(overlap) * 4

        query_bigrams = cls._bigrams(query_tokens)
        if query_bigrams:
            score += 3 * len(query_bigrams & cls._bigrams(name_tokens))
            score += 2 * len(query_bigrams & cls._bigrams(path_tokens))

        if matched_sets >= 2:
            score += 4
        if matched_sets == 3:
            score += 2

        source_parts = [part for part in module.source_hint.split('/') if part]
        if len(source_parts) >= 2:
            parent = cls._normalize_text(source_parts[-2])
            if set(query_tokens) & set(parent):
                score += 4

        if name_tokens and any(token == module.name.lower() for token in query_tokens):
            score += 6

        if ordered_query_tokens:
            lead_token = ordered_query_tokens[0]
            if lead_token in name_tokens:
                score += 18 if kind == 'command' else 8
            elif lead_token in path_tokens:
                score += 8 if kind == 'command' else 4

        return score

    @staticmethod
    def _bigrams(tokens: Counter[str]) -> set[str]:
        ordered = list(tokens)
        return {f'{ordered[index]}::{ordered[index + 1]}' for index in range(len(ordered) - 1)}
