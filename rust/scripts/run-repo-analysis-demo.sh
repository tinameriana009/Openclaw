#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
RUST_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd -- "$RUST_ROOT/.." && pwd)
ARTIFACT_ROOT=${ARTIFACT_ROOT:-"$REPO_ROOT/.demo-artifacts/repo-analysis-demo"}
PROFILE=${PROFILE:-balanced}
CLAW_BIN=${CLAW_BIN:-"$RUST_ROOT/target/debug/claw"}
TIMESTAMP=$(date -u +"%Y%m%dT%H%M%SZ")
RUN_DIR="$ARTIFACT_ROOT/$TIMESTAMP"
STEP1_OUT="$RUN_DIR/01-brief-response.txt"
STEP2_OUT="$RUN_DIR/02-followup-response.txt"
RUN_META="$RUN_DIR/run-metadata.txt"
TRACE_HINT="$RUN_DIR/next-steps.txt"
SESSION_TEMPLATE="$RUN_DIR/operator-session-template.md"
NEXT_PROMPT_TEMPLATE="$RUN_DIR/next-prompt-template.md"
REPORT_TEMPLATE="$RUN_DIR/operator-findings-template.md"
SUMMARY_JSON="$RUN_DIR/bundle-summary.json"
CHECKSUMS="$RUN_DIR/bundle-checksums.txt"
HANDOFF_JSON="$RUN_DIR/operator-handoff.json"
DASHBOARD_HTML="$RUN_DIR/operator-dashboard.html"
RUNTIME_BRIDGE_JSON="$RUN_DIR/runtime-bridge.json"
REVIEW_STATUS_JSON="$RUN_DIR/review-status.json"
REVIEW_LOG_MD="$RUN_DIR/review-log.md"
TRANSITION_MD="$RUN_DIR/operator-transition-brief.md"
QUEUE_STATE_JSON="$RUN_DIR/queue-state.json"
# queue-state.json remains the operator-facing queue ledger name in the docs/tests; queue-state.json is the current script artifact.
INDEX_JSON="$ARTIFACT_ROOT/index.json"
INDEX_HTML="$ARTIFACT_ROOT/index.html"

mkdir -p "$RUN_DIR"

if [[ ! -x "$CLAW_BIN" ]]; then
  cat <<EOF
ERROR: expected built claw binary at:
  $CLAW_BIN

Build it first:
  cd $RUST_ROOT
  cargo build --workspace --locked
EOF
  exit 2
fi

if [[ $# -gt 0 ]]; then
  cat <<EOF
This helper does not take positional arguments today.
Optional environment overrides:
  PROFILE=deep|research
  ARTIFACT_ROOT=/custom/output/path
  CLAW_BIN=/custom/claw/path

Example:
  cd $RUST_ROOT
  PROFILE=deep ./scripts/run-repo-analysis-demo.sh
EOF
  exit 2
fi

BRIEF_PROMPT=$(cat "$REPO_ROOT/docs/examples/repo-analysis-demo/brief.md")
FOLLOWUP_PROMPT=$(cat <<'EOF'
Trace the path from the CLI entrypoint through query routing, runtime/bootstrap state, and execution registry selection. Name the files involved and explain what each contributes. Distinguish facts from inferences.
EOF
)

cat >"$RUN_META" <<EOF
repo_root=$REPO_ROOT
rust_root=$RUST_ROOT
claw_bin=$CLAW_BIN
profile=$PROFILE
run_dir=$RUN_DIR
step1_output=$STEP1_OUT
step2_output=$STEP2_OUT
started_at_utc=$TIMESTAMP
resume_command=./target/debug/claw --resume latest
trace_summary_command=./target/debug/claw --resume latest /trace summary .claw/trace/<trace-file>
replay_command=/trace replay <trace-file|approval-packet>
resume_trace_command=/trace resume <trace-file|approval-packet>
trace_handoff_command=/trace handoff [target]
review_index_json=$INDEX_JSON
review_index_html=$INDEX_HTML
queue_state=$QUEUE_STATE_JSON
transition_brief=$TRANSITION_MD
EOF

echo "== Repo analysis demo =="
echo "profile: $PROFILE"
echo "artifacts: $RUN_DIR"
echo

echo "[1/2] Running onboarding brief..."
(
  cd "$RUST_ROOT"
  "$CLAW_BIN" --profile "$PROFILE" \
    --corpus ../src \
    --corpus ../tests \
    prompt "$BRIEF_PROMPT"
) | tee "$STEP1_OUT"

echo

echo "[2/2] Running file-path follow-up on the resumed session..."
(
  cd "$RUST_ROOT"
  "$CLAW_BIN" --resume latest prompt "$FOLLOWUP_PROMPT"
) | tee "$STEP2_OUT"

cp "$REPO_ROOT/docs/examples/repo-analysis-demo/operator-session-template.md" "$SESSION_TEMPLATE"
cp "$REPO_ROOT/docs/examples/repo-analysis-demo/next-prompt-template.md" "$NEXT_PROMPT_TEMPLATE"

cat >"$REPORT_TEMPLATE" <<'EOF'
# Repo analysis operator findings

## Run reviewed
- Run artifact directory:
- Step 1 response reviewed? yes / no
- Step 2 response reviewed? yes / no
- Inherited from prior run bundle:

## Strong grounded findings
- 

## Weak or missing claims
- 

## Files/tests manually spot-checked
- 

## Trace review notes
- Trace file(s):
- Missing evidence:
- Continuity surprises between passes:

## Replay / resume continuity
- Exact resume command used next:
- Exact trace summary command used next:
- If re-running, what changed between passes?
- Which prior claim should be challenged first?
- What should the next operator trust vs re-check?

## Recommended next prompt
- 
EOF

cat >"$REVIEW_LOG_MD" <<EOF
# Repo analysis review log

## Run identity
- Timestamp: $TIMESTAMP
- Profile: $PROFILE
- Run dir: $RUN_DIR

## Review status
- Status: pending-review
- Reviewed by:
- Review started at:
- Review completed at:

## Cross-run continuity
- Prior run compared against:
- What changed from the previous reviewed run?
- Which claims still need manual verification?
- Which resume command should the next operator use?
- Which trace or approval packet should the next operator inspect first?

## Evidence ledger
- Files spot-checked:
- Trace files reviewed:
- Mismatches or surprises:
- Follow-up prompt queued:

## Handoff note
- Recommended next operator action:
- Smallest useful next question:
- What should *not* be assumed from this bundle:
EOF

cat >"$TRACE_HINT" <<EOF
Review steps for this run:
1. Compare the answers against:
   - docs/examples/repo-analysis-demo/expected-findings.md
   - docs/examples/repo-analysis-demo/manual-validation-checklist.md
2. Capture exact evidence, weak spots, and handoff notes in:
   $SESSION_TEMPLATE
   $REPORT_TEMPLATE
   $REVIEW_LOG_MD
   $TRANSITION_MD
3. If the answers sound overconfident, inspect traces from rust/.claw/trace/ using:
   ./target/debug/claw --resume latest /trace summary .claw/trace/<trace-file>
4. Use the staged next-prompt template for the next narrowed ask:
   $NEXT_PROMPT_TEMPLATE
5. If you intentionally want to re-run the approved trace path, use the current bounded commands:
   /trace replay <trace-file|approval-packet>
   /trace resume <trace-file|approval-packet>
   /trace handoff [target]
6. Check the cross-run review index for older bundles:
   $INDEX_HTML
   $INDEX_JSON
7. Update the queue state before handing off:
   $QUEUE_STATE_JSON
   $TRANSITION_MD
8. Re-run the demo validator if you changed docs/assets:
   python3 tests/validate_repo_analysis_demo.py

This helper only runs the documented prompt flow and captures outputs.
It does not certify answer quality, drive a browser, or verify the repository automatically.
EOF

cat >"$RUN_DIR/bundle-manifest.txt" <<EOF
01-brief-response.txt
02-followup-response.txt
run-metadata.txt
operator-session-template.md
next-prompt-template.md
operator-findings-template.md
review-log.md
review-status.json
queue-state.json
operator-transition-brief.md
next-steps.txt
bundle-summary.json
operator-handoff.json
operator-dashboard.html
runtime-bridge.json
bundle-checksums.txt
EOF

python3 - <<PY
from __future__ import annotations

import datetime as dt
import html
import json
from pathlib import Path

run_dir = Path(${RUN_DIR@Q})
artifact_root = Path(${ARTIFACT_ROOT@Q})
rust_root = Path(${RUST_ROOT@Q})
summary_path = Path(${SUMMARY_JSON@Q})
handoff_path = Path(${HANDOFF_JSON@Q})
dashboard_path = Path(${DASHBOARD_HTML@Q})
runtime_bridge_path = Path(${RUNTIME_BRIDGE_JSON@Q})
review_status_path = Path(${REVIEW_STATUS_JSON@Q})
continuity_path = Path(${QUEUE_STATE_JSON@Q})
transition_path = Path(${TRANSITION_MD@Q})
index_json_path = Path(${INDEX_JSON@Q})
index_html_path = Path(${INDEX_HTML@Q})
manifest_entries = [
    line.strip()
    for line in (run_dir / 'bundle-manifest.txt').read_text().splitlines()
    if line.strip()
]

existing_runs: list[dict] = []
if index_json_path.exists():
    try:
        existing_runs = json.loads(index_json_path.read_text()).get('runs', [])
    except json.JSONDecodeError:
        existing_runs = []

prior_runs_sorted = sorted(
    [item for item in existing_runs if isinstance(item, dict)],
    key=lambda item: str(item.get('runId') or ''),
    reverse=True,
)
prior_run = prior_runs_sorted[0] if prior_runs_sorted else None
prior_reviewed_run = next(
    (item for item in prior_runs_sorted if str(item.get('status')) == 'review-complete'),
    None,
)

def compact_run_pointer(run: dict | None) -> dict | None:
    if not run:
        return None
    return {
        'runId': run.get('runId'),
        'profile': run.get('profile'),
        'status': run.get('status'),
        'evidenceStatus': run.get('evidenceStatus'),
        'traceStatus': run.get('traceStatus'),
        'runDir': run.get('runDir'),
        'dashboard': run.get('dashboard'),
        'reviewStatus': run.get('reviewStatus'),
        'reviewLog': run.get('reviewLog'),
        'resumeSessionCommand': run.get('resumeSessionCommand'),
        'traceSummaryCommand': run.get('traceSummaryCommand'),
        'operatorNextStep': run.get('operatorNextStep'),
    }


def iso_utc(epoch_seconds: float) -> str:
    return dt.datetime.fromtimestamp(epoch_seconds, tz=dt.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')


def relative_to_rust(path: Path) -> str:
    try:
        return path.relative_to(rust_root).as_posix()
    except ValueError:
        return str(path)


def load_runtime_bridge() -> dict[str, object]:
    sessions_dir = rust_root / '.claw' / 'sessions'
    trace_dir = rust_root / '.claw' / 'trace'
    session_entries = sorted([p for p in sessions_dir.glob('*.jsonl') if p.is_file()], key=lambda p: p.stat().st_mtime, reverse=True) if sessions_dir.exists() else []
    trace_entries = sorted([p for p in trace_dir.glob('*.json') if p.is_file()], key=lambda p: p.stat().st_mtime, reverse=True) if trace_dir.exists() else []

    latest_session = None
    if session_entries:
        latest = session_entries[0]
        latest_session = {
            'sessionId': latest.stem,
            'path': relative_to_rust(latest),
            'modifiedAtUtc': iso_utc(latest.stat().st_mtime),
            'sizeBytes': latest.stat().st_size,
            'messageCountEstimate': sum(1 for line in latest.read_text().splitlines() if line.strip()),
            'resumeCommand': './target/debug/claw --resume latest',
        }

    recent_traces = [
        {
            'traceId': path.stem,
            'path': relative_to_rust(path),
            'modifiedAtUtc': iso_utc(path.stat().st_mtime),
            'sizeBytes': path.stat().st_size,
            'summaryCommand': f'./target/debug/claw --resume latest /trace summary {relative_to_rust(path)}',
        }
        for path in trace_entries[:5]
    ]

    return {
        'schemaVersion': 1,
        'generatedAtUtc': ${TIMESTAMP@Q},
        'runtimeRoot': str(rust_root / '.claw'),
        'latestSession': latest_session,
        'recentTraces': recent_traces,
        'webOperatorBridge': {
            'bundleDashboard': 'operator-dashboard.html',
            'reviewStatus': 'review-status.json',
            'queueState': 'queue-state.json',
            'transitionBrief': 'operator-transition-brief.md',
            'honestyNote': 'Static snapshot only: this bundle reflects the runtime/session/trace state seen when the helper finished. It is not a live sync channel.',
        },
    }

runtime_bridge = load_runtime_bridge()
runtime_bridge_path.write_text(json.dumps(runtime_bridge, indent=2) + '\n')

summary = {
    'workflow': 'repo-analysis-demo',
    'generatedAtUtc': ${TIMESTAMP@Q},
    'runDir': str(run_dir),
    'profile': ${PROFILE@Q},
    'validatorCommand': 'python3 tests/validate_repo_analysis_demo.py',
    'runCommands': [
        './target/debug/claw --profile ' + ${PROFILE@Q} + ' --corpus ../src --corpus ../tests prompt <brief>',
        './target/debug/claw --resume latest prompt <follow-up>',
    ],
    'manualValidationRequired': True,
    'bundleEntries': manifest_entries,
    'operatorHandoffFiles': [
        'operator-session-template.md',
        'operator-findings-template.md',
        'review-log.md',
        'review-status.json',
        'queue-state.json',
        'operator-transition-brief.md',
        'next-prompt-template.md',
        'next-steps.txt',
        'operator-dashboard.html',
        'runtime-bridge.json',
    ],
    'continuityCommands': {
        'resumeSession': './target/debug/claw --resume latest',
        'traceSummary': './target/debug/claw --resume latest /trace summary .claw/trace/<trace-file>',
        'traceReplay': '/trace replay <trace-file|approval-packet>',
        'traceResume': '/trace resume <trace-file|approval-packet>',
        'traceHandoff': '/trace handoff [target]',
    },
    'continuityArtifacts': {
        'transitionBrief': 'operator-transition-brief.md',
        'queueState': 'queue-state.json',
        'reviewStatus': 'review-status.json',
        'reviewLog': 'review-log.md',
        'runtimeBridge': 'runtime-bridge.json',
    },
    'runtimeBridge': {
        'path': 'runtime-bridge.json',
        'latestSessionId': (runtime_bridge.get('latestSession') or {}).get('sessionId'),
        'recentTraceIds': [trace.get('traceId') for trace in runtime_bridge.get('recentTraces', [])],
    },
    'priorRun': compact_run_pointer(prior_run),
    'priorReviewedRun': compact_run_pointer(prior_reviewed_run),
    'crossRunIndex': {
        'json': str(index_json_path),
        'html': str(index_html_path),
    },
    'caveats': [
        'This helper only runs the documented prompt flow and stages review artifacts.',
        'It does not certify answer quality, drive a browser, or verify the repository automatically.',
    ],
}
summary_path.write_text(json.dumps(summary, indent=2) + '\n')

review_status = {
    'workflow': 'repo-analysis-demo',
    'generatedAtUtc': ${TIMESTAMP@Q},
    'runDir': str(run_dir),
    'status': 'pending-review',
    'manualValidationRequired': True,
    'reviewedBy': None,
    'reviewStartedAtUtc': None,
    'reviewCompletedAtUtc': None,
    'evidenceStatus': 'not-reviewed',
    'traceStatus': 'optional-not-reviewed',
    'followupPromptQueued': False,
    'resumeSessionCommand': './target/debug/claw --resume latest',
    'traceSummaryCommand': './target/debug/claw --resume latest /trace summary .claw/trace/<trace-file>',
    'traceReplayCommand': '/trace replay <trace-file|approval-packet>',
    'traceResumeCommand': '/trace resume <trace-file|approval-packet>',
    'traceHandoffCommand': '/trace handoff [target]',
    'reviewLog': 'review-log.md',
    'queueState': 'queue-state.json',
    'transitionBrief': 'operator-transition-brief.md',
    'notes': 'Update this file by hand when review starts/completes so the shared index reflects the bounded operator state honestly.',
}
review_status_path.write_text(json.dumps(review_status, indent=2) + '\n')

continuity_status = {
    'workflow': 'repo-analysis-demo',
    'generatedAtUtc': ${TIMESTAMP@Q},
    'runDir': str(run_dir),
    'sessionState': 'ready-for-review',
    'reviewState': 'not-started',
    'handoffState': 'awaiting-first-operator',
    'currentOwner': None,
    'nextOwner': None,
    'resumeSessionCommand': './target/debug/claw --resume latest',
    'traceSummaryCommand': './target/debug/claw --resume latest /trace summary .claw/trace/<trace-file>',
    'traceReplayCommand': '/trace replay <trace-file|approval-packet>',
    'traceResumeCommand': '/trace resume <trace-file|approval-packet>',
    'traceHandoffCommand': '/trace handoff [target]',
    'latestOperatorNote': 'Fill operator-transition-brief.md before handoff so the next operator inherits exact review context, not just raw artifacts.',
    'priorRun': compact_run_pointer(prior_run),
    'priorReviewedRun': compact_run_pointer(prior_reviewed_run),
}
continuity_path.write_text(json.dumps(continuity_status, indent=2) + '\n')

prior_run_label = prior_run.get('runId') if prior_run else 'none'
prior_reviewed_label = prior_reviewed_run.get('runId') if prior_reviewed_run else 'none'
transition_lines = [
    '# Operator transition brief',
    '',
    'This is the baton-pass note for the next operator. Keep it short, exact, and file-backed.',
    '',
    '## Run identity',
    f'- Current run: {${TIMESTAMP@Q}}',
    f'- Profile: {${PROFILE@Q}}',
    f'- Run dir: {run_dir}',
    f'- Prior run in index: {prior_run_label}',
    f'- Prior fully reviewed run: {prior_reviewed_label}',
    '',
    '## What is already true',
    '- The two prompt responses are staged locally in this run bundle.',
    '- No review has been completed yet unless review-status.json says otherwise.',
    '- This bundle supports bounded CLI continuity only; it is not a live dashboard backend.',
    '',
    '## What the next operator should do first',
    '- Open operator-dashboard.html for the local run summary.',
    '- Read review-status.json and queue-state.json before trusting older notes.',
    '- Compare 01-brief-response.txt and 02-followup-response.txt against expected-findings.md.',
    '- Update review-log.md with exact files spot-checked and any claims that still need re-checking.',
    '',
    '## Exact continuity commands',
    '- Resume session: `./target/debug/claw --resume latest`',
    '- Trace summary: `./target/debug/claw --resume latest /trace summary .claw/trace/<trace-file>`',
    '- Trace replay: `/trace replay <trace-file|approval-packet>`',
    '- Trace resume: `/trace resume <trace-file|approval-packet>`',
    '- Trace handoff: `/trace handoff [target]`',
    '',
    '## Fill before handoff',
    '- Current operator:',
    '- Next operator:',
    '- Facts manually verified this pass:',
    '- Claims that still need re-checking:',
    '- Smallest high-value next prompt:',
    '- Files/tests the next operator should inspect first:',
    '- What changed versus the prior reviewed run:',
]
transition_path.write_text('\n'.join(transition_lines) + '\n')

def esc(value: str) -> str:
    return html.escape(value, quote=True)

dashboard_html = f'''<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Repo analysis operator dashboard</title>
  <style>
    body {{ font-family: system-ui, sans-serif; margin: 2rem; line-height: 1.5; background: #0f172a; color: #e2e8f0; }}
    code, pre {{ font-family: ui-monospace, monospace; background: #111827; color: #e5e7eb; }}
    code {{ padding: 0.1rem 0.3rem; border-radius: 0.25rem; }}
    pre {{ padding: 0.9rem; border-radius: 0.5rem; overflow-x: auto; }}
    a {{ color: #93c5fd; }}
    .card {{ background: #111827; padding: 1rem 1.2rem; border-radius: 0.75rem; margin-bottom: 1rem; border: 1px solid #334155; }}
    ul {{ padding-left: 1.2rem; }}
  </style>
</head>
<body>
  <h1>Repo analysis operator dashboard</h1>
  <p>Static run bundle for review, replay, and resume continuity. This is an on-disk handoff artifact, not a live web app.</p>

  <div class="card">
    <h2>Run summary</h2>
    <ul>
      <li><strong>Generated at:</strong> {esc(${TIMESTAMP@Q})}</li>
      <li><strong>Profile:</strong> {esc(${PROFILE@Q})}</li>
      <li><strong>Run dir:</strong> <code>{esc(str(run_dir))}</code></li>
      <li><strong>Review status:</strong> <code>pending-review</code> (edit <code>review-status.json</code> as review progresses)</li>
      <li><strong>Queue state:</strong> <code>queue-state.json</code> tracks current/next operator and handoff state</li>
      <li><strong>Initial response:</strong> <code>01-brief-response.txt</code></li>
      <li><strong>Follow-up response:</strong> <code>02-followup-response.txt</code></li>
      <li><strong>Transition brief:</strong> <code>operator-transition-brief.md</code></li>
      <li><strong>Cross-run index:</strong> <code>{esc(str(index_html_path))}</code></li>
      <li><strong>Prior run in index:</strong> <code>{esc(prior_run_label)}</code></li>
      <li><strong>Prior fully reviewed run:</strong> <code>{esc(prior_reviewed_label)}</code></li>
    </ul>
  </div>

  <div class="card">
    <h2>Review flow</h2>
    <ol>
      <li>Compare both responses against <code>docs/examples/repo-analysis-demo/expected-findings.md</code>.</li>
      <li>Use <code>docs/examples/repo-analysis-demo/manual-validation-checklist.md</code> for file spot-checks.</li>
      <li>Capture evidence and weak claims in <code>operator-session-template.md</code>, <code>operator-findings-template.md</code>, and <code>review-log.md</code>.</li>
      <li>Update <code>review-status.json</code> and <code>queue-state.json</code> so the shared multi-run index reflects reality.</li>
      <li>Fill <code>operator-transition-brief.md</code> before handing the run to another operator.</li>
      <li>Inspect trace evidence if the model made a jump that the files do not justify.</li>
      <li>Continue the same session with <code>next-prompt-template.md</code> instead of starting over.</li>
    </ol>
  </div>

  <div class="card">
    <h2>Continuity commands</h2>
    <pre>./target/debug/claw --resume latest
./target/debug/claw --resume latest /trace summary .claw/trace/&lt;trace-file&gt;
/trace replay &lt;trace-file|approval-packet&gt;
/trace resume &lt;trace-file|approval-packet&gt;
/trace handoff [target]</pre>
    <p>Replay/resume here means bounded CLI continuity over saved traces and approval packets. It does <strong>not</strong> mean browser automation or a live operator UI.</p>
  </div>

  <div class="card">
    <h2>Bundle files</h2>
    <ul>
      {''.join(f'<li><code>{esc(entry)}</code></li>' for entry in manifest_entries)}
    </ul>
  </div>
</body>
</html>
'''
dashboard_path.write_text(dashboard_html)

runs = []
for candidate in sorted(
    [path for path in artifact_root.iterdir() if path.is_dir()],
    key=lambda item: item.name,
    reverse=True,
):
    bundle_summary_path = candidate / 'bundle-summary.json'
    handoff_candidate_path = candidate / 'operator-handoff.json'
    review_candidate_path = candidate / 'review-status.json'
    continuity_candidate_path = candidate / 'queue-state.json'
    if not bundle_summary_path.exists() or not handoff_candidate_path.exists():
        continue
    bundle_summary = json.loads(bundle_summary_path.read_text())
    handoff_candidate = json.loads(handoff_candidate_path.read_text())
    review_candidate = json.loads(review_candidate_path.read_text()) if review_candidate_path.exists() else {}
    continuity_candidate = json.loads(continuity_candidate_path.read_text()) if continuity_candidate_path.exists() else {}
    runtime_bridge_candidate_path = candidate / 'runtime-bridge.json'
    runtime_bridge_candidate = json.loads(runtime_bridge_candidate_path.read_text()) if runtime_bridge_candidate_path.exists() else {}
    runs.append({
        'runId': candidate.name,
        'generatedAtUtc': bundle_summary.get('generatedAtUtc', candidate.name),
        'profile': bundle_summary.get('profile'),
        'runDir': str(candidate),
        'status': review_candidate.get('status', 'status-unknown'),
        'evidenceStatus': review_candidate.get('evidenceStatus', 'not-recorded'),
        'traceStatus': review_candidate.get('traceStatus', 'not-recorded'),
        'followupPromptQueued': review_candidate.get('followupPromptQueued', False),
        'handoffState': continuity_candidate.get('handoffState', 'unknown'),
        'currentOwner': continuity_candidate.get('currentOwner'),
        'nextOwner': continuity_candidate.get('nextOwner'),
        'bundleSummary': str(bundle_summary_path),
        'operatorHandoff': str(handoff_candidate_path),
        'reviewStatus': str(review_candidate_path) if review_candidate_path.exists() else None,
        'queueState': str(continuity_candidate_path) if continuity_candidate_path.exists() else None,
        'runtimeBridge': str(runtime_bridge_candidate_path) if runtime_bridge_candidate_path.exists() else None,
        'latestSessionId': (runtime_bridge_candidate.get('latestSession') or {}).get('sessionId'),
        'recentTraceCount': len(runtime_bridge_candidate.get('recentTraces') or []),
        'transitionBrief': str(candidate / 'operator-transition-brief.md') if (candidate / 'operator-transition-brief.md').exists() else None,
        'reviewLog': str(candidate / 'review-log.md') if (candidate / 'review-log.md').exists() else None,
        'dashboard': str(candidate / 'operator-dashboard.html') if (candidate / 'operator-dashboard.html').exists() else None,
        'resumeSessionCommand': review_candidate.get('resumeSessionCommand', bundle_summary.get('continuityCommands', {}).get('resumeSession')),
        'traceSummaryCommand': review_candidate.get('traceSummaryCommand', bundle_summary.get('continuityCommands', {}).get('traceSummary')),
        'traceReplayCommand': review_candidate.get('traceReplayCommand', bundle_summary.get('continuityCommands', {}).get('traceReplay')),
        'traceResumeCommand': review_candidate.get('traceResumeCommand', bundle_summary.get('continuityCommands', {}).get('traceResume')),
        'operatorNextStep': handoff_candidate.get('operatorNextStep'),
        'previousReviewedRunId': (bundle_summary.get('priorReviewedRun') or {}).get('runId'),
    })

index_payload = {
    'workflow': 'repo-analysis-demo',
    'generatedAtUtc': ${TIMESTAMP@Q},
    'artifactRoot': str(artifact_root),
    'reviewModel': 'bounded-static-review-index',
    'notes': [
        'This index aggregates staged run bundles for cross-run review and resume continuity.',
        'Operators should update each run\'s review-status.json, queue-state.json, and review-log.md by hand; this is not a live web service.',
    ],
    'runs': runs,
}
index_json_path.write_text(json.dumps(index_payload, indent=2) + '\n')

rows = []
for run in runs:
    rows.append(
        '<tr>'
        f'<td><code>{esc(run["runId"])}</code></td>'
        f'<td>{esc(str(run.get("profile") or "unknown"))}</td>'
        f'<td><code>{esc(str(run.get("status") or "unknown"))}</code></td>'
        f'<td><code>{esc(str(run.get("evidenceStatus") or "unknown"))}</code></td>'
        f'<td><code>{esc(str(run.get("traceStatus") or "unknown"))}</code></td>'
        f'<td><code>{esc(str(run.get("handoffState") or "unknown"))}</code><br><small>session={esc(str(run.get("latestSessionId") or "n/a"))}</small><br><small>traces={esc(str(run.get("recentTraceCount") or 0))}</small></td>'
        f'<td>{"yes" if run.get("followupPromptQueued") else "no"}</td>'
        '</tr>'
    )

index_html = f'''<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Repo analysis review index</title>
  <style>
    body {{ font-family: system-ui, sans-serif; margin: 2rem; line-height: 1.5; background: #020617; color: #e2e8f0; }}
    code, pre {{ font-family: ui-monospace, monospace; background: #0f172a; color: #e5e7eb; }}
    code {{ padding: 0.1rem 0.3rem; border-radius: 0.25rem; }}
    pre {{ padding: 0.9rem; border-radius: 0.5rem; overflow-x: auto; }}
    table {{ width: 100%; border-collapse: collapse; margin-top: 1rem; }}
    th, td {{ border: 1px solid #334155; padding: 0.6rem; text-align: left; vertical-align: top; }}
    th {{ background: #111827; }}
    .card {{ background: #111827; padding: 1rem 1.2rem; border-radius: 0.75rem; margin-bottom: 1rem; border: 1px solid #334155; }}
    a {{ color: #93c5fd; }}
  </style>
</head>
<body>
  <h1>Repo analysis review index</h1>
  <p>Static multi-run index for staged repo-analysis demo bundles. This is an on-disk review surface, not a live web UI.</p>

  <div class="card">
    <h2>How to use this index</h2>
    <ol>
      <li>Open the newest run bundle first.</li>
      <li>Check <code>review-status.json</code> and <code>queue-state.json</code> before trusting a prior pass.</li>
      <li>Use <code>review-log.md</code> and <code>operator-transition-brief.md</code> to record what changed between runs.</li>
      <li>Resume the same session or use <code>/trace replay</code> / <code>/trace resume</code> only as bounded CLI continuity tools.</li>
    </ol>
  </div>

  <div class="card">
    <h2>Runs</h2>
    <table>
      <thead>
        <tr>
          <th>Run</th>
          <th>Profile</th>
          <th>Review status</th>
          <th>Evidence</th>
          <th>Trace</th>
          <th>Handoff</th>
          <th>Follow-up queued</th>
        </tr>
      </thead>
      <tbody>
        {''.join(rows) if rows else '<tr><td colspan="7">No staged runs yet.</td></tr>'}
      </tbody>
    </table>
  </div>

  <div class="card">
    <h2>Continuity commands</h2>
    <pre>./target/debug/claw --resume latest
./target/debug/claw --resume latest /trace summary .claw/trace/&lt;trace-file&gt;
/trace replay &lt;trace-file|approval-packet&gt;
/trace resume &lt;trace-file|approval-packet&gt;
/trace handoff [target]</pre>
    <p>These commands preserve bounded CLI review/resume continuity only. They do <strong>not</strong> imply browser automation, a live dashboard backend, or a cross-run verification service.</p>
  </div>
</body>
</html>
'''
index_html_path.write_text(index_html)

handoff = {
    'workflow': 'repo-analysis-demo',
    'generatedAtUtc': ${TIMESTAMP@Q},
    'runDir': str(run_dir),
    'profile': ${PROFILE@Q},
    'initialResponse': '01-brief-response.txt',
    'followupResponse': '02-followup-response.txt',
    'operatorChecklist': 'docs/examples/repo-analysis-demo/manual-validation-checklist.md',
    'operatorSessionTemplate': 'operator-session-template.md',
    'operatorFindingsTemplate': 'operator-findings-template.md',
    'reviewLog': 'review-log.md',
    'reviewStatus': 'review-status.json',
    'queueState': 'queue-state.json',
    'transitionBrief': 'operator-transition-brief.md',
    'nextPromptTemplate': 'next-prompt-template.md',
    'operatorDashboard': 'operator-dashboard.html',
    'crossRunIndex': {
        'json': str(index_json_path),
        'html': str(index_html_path),
    },
    'priorRun': compact_run_pointer(prior_run),
    'priorReviewedRun': compact_run_pointer(prior_reviewed_run),
    'operatorNextStep': 'Review the two responses against expected-findings.md, inspect any surprising trace claims, update queue-state.json plus operator-transition-brief.md, and continue the same session with a narrower evidence-backed prompt.',
    'automationStatus': 'staged-review-and-resume-only',
    'manualValidationRequired': True,
}
handoff_path.write_text(json.dumps(handoff, indent=2) + '\n')
PY

(
  cd "$RUN_DIR"
  find . -type f ! -name "$(basename "$CHECKSUMS")" -print0 \
    | sort -z \
    | xargs -0 sha256sum >"$CHECKSUMS"
)

echo
echo "Staged continuity artifacts:"
echo "  $QUEUE_STATE_JSON"
echo "  $TRANSITION_MD"
echo
cat "$TRACE_HINT"
