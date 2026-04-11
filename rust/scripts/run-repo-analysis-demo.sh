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
REVIEW_STATUS_JSON="$RUN_DIR/review-status.json"
REVIEW_LOG_MD="$RUN_DIR/review-log.md"
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

## Strong grounded findings
- 

## Weak or missing claims
- 

## Files/tests manually spot-checked
- 

## Trace review notes
- Trace file(s):
- Missing evidence:

## Replay / resume continuity
- Exact resume command used next:
- Exact trace summary command used next:
- If re-running, what changed between passes?
- Which prior claim should be challenged first?

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

## Evidence ledger
- Files spot-checked:
- Trace files reviewed:
- Mismatches or surprises:
- Follow-up prompt queued:

## Handoff note
- Recommended next operator action:
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
7. Re-run the demo validator if you changed docs/assets:
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
next-steps.txt
bundle-summary.json
operator-handoff.json
operator-dashboard.html
bundle-checksums.txt
EOF

python3 - <<PY
from __future__ import annotations

import html
import json
from pathlib import Path

run_dir = Path(${RUN_DIR@Q})
artifact_root = Path(${ARTIFACT_ROOT@Q})
summary_path = Path(${SUMMARY_JSON@Q})
handoff_path = Path(${HANDOFF_JSON@Q})
dashboard_path = Path(${DASHBOARD_HTML@Q})
review_status_path = Path(${REVIEW_STATUS_JSON@Q})
index_json_path = Path(${INDEX_JSON@Q})
index_html_path = Path(${INDEX_HTML@Q})
manifest_entries = [
    line.strip()
    for line in (run_dir / 'bundle-manifest.txt').read_text().splitlines()
    if line.strip()
]
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
        'next-prompt-template.md',
        'next-steps.txt',
        'operator-dashboard.html',
    ],
    'continuityCommands': {
        'resumeSession': './target/debug/claw --resume latest',
        'traceSummary': './target/debug/claw --resume latest /trace summary .claw/trace/<trace-file>',
        'traceReplay': '/trace replay <trace-file|approval-packet>',
        'traceResume': '/trace resume <trace-file|approval-packet>',
        'traceHandoff': '/trace handoff [target]',
    },
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
    'notes': 'Update this file by hand when review starts/completes so the shared index reflects the bounded operator state honestly.',
}
review_status_path.write_text(json.dumps(review_status, indent=2) + '\n')

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
    'nextPromptTemplate': 'next-prompt-template.md',
    'operatorDashboard': 'operator-dashboard.html',
    'crossRunIndex': {
        'json': str(index_json_path),
        'html': str(index_html_path),
    },
    'operatorNextStep': 'Review the two responses against expected-findings.md, inspect any surprising trace claims, and continue the same session with a narrower evidence-backed prompt.',
    'automationStatus': 'staged-review-and-resume-only',
    'manualValidationRequired': True,
}
handoff_path.write_text(json.dumps(handoff, indent=2) + '\n')

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
      <li><strong>Initial response:</strong> <code>01-brief-response.txt</code></li>
      <li><strong>Follow-up response:</strong> <code>02-followup-response.txt</code></li>
      <li><strong>Cross-run index:</strong> <code>{esc(str(index_html_path))}</code></li>
    </ul>
  </div>

  <div class="card">
    <h2>Review flow</h2>
    <ol>
      <li>Compare both responses against <code>docs/examples/repo-analysis-demo/expected-findings.md</code>.</li>
      <li>Use <code>docs/examples/repo-analysis-demo/manual-validation-checklist.md</code> for file spot-checks.</li>
      <li>Capture evidence and weak claims in <code>operator-session-template.md</code>, <code>operator-findings-template.md</code>, and <code>review-log.md</code>.</li>
      <li>Update <code>review-status.json</code> so the shared multi-run index reflects reality.</li>
      <li>Inspect trace evidence if the model made a jump that the files do not justify.</li>
      <li>Continue the same session with <code>next-prompt-template.md</code> instead of starting over.</li>
    </ol>
  </div>

  <div class="card">
    <h2>Continuity commands</h2>
    <pre>./target/debug/claw --resume latest
./target/debug/claw --resume latest /trace summary .claw/trace/&lt;trace-file&gt;
/trace replay &lt;trace-file|approval-packet&gt;
/trace resume &lt;trace-file|approval-packet&gt;</pre>
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
    if not bundle_summary_path.exists() or not handoff_candidate_path.exists():
        continue
    bundle_summary = json.loads(bundle_summary_path.read_text())
    handoff_candidate = json.loads(handoff_candidate_path.read_text())
    review_candidate = json.loads(review_candidate_path.read_text()) if review_candidate_path.exists() else {}
    runs.append({
        'runId': candidate.name,
        'generatedAtUtc': bundle_summary.get('generatedAtUtc', candidate.name),
        'profile': bundle_summary.get('profile'),
        'runDir': str(candidate),
        'status': review_candidate.get('status', 'status-unknown'),
        'evidenceStatus': review_candidate.get('evidenceStatus', 'not-recorded'),
        'traceStatus': review_candidate.get('traceStatus', 'not-recorded'),
        'followupPromptQueued': review_candidate.get('followupPromptQueued', False),
        'bundleSummary': str(bundle_summary_path),
        'operatorHandoff': str(handoff_candidate_path),
        'reviewStatus': str(review_candidate_path) if review_candidate_path.exists() else None,
        'reviewLog': str(candidate / 'review-log.md') if (candidate / 'review-log.md').exists() else None,
        'dashboard': str(candidate / 'operator-dashboard.html') if (candidate / 'operator-dashboard.html').exists() else None,
        'resumeSessionCommand': review_candidate.get('resumeSessionCommand', bundle_summary.get('continuityCommands', {}).get('resumeSession')),
        'traceSummaryCommand': review_candidate.get('traceSummaryCommand', bundle_summary.get('continuityCommands', {}).get('traceSummary')),
        'traceReplayCommand': review_candidate.get('traceReplayCommand', bundle_summary.get('continuityCommands', {}).get('traceReplay')),
        'traceResumeCommand': review_candidate.get('traceResumeCommand', bundle_summary.get('continuityCommands', {}).get('traceResume')),
        'operatorNextStep': handoff_candidate.get('operatorNextStep'),
    })

index_payload = {
    'workflow': 'repo-analysis-demo',
    'generatedAtUtc': ${TIMESTAMP@Q},
    'artifactRoot': str(artifact_root),
    'reviewModel': 'bounded-static-review-index',
    'notes': [
        'This index aggregates staged run bundles for cross-run review and resume continuity.',
        'Operators should update each run\'s review-status.json and review-log.md by hand; this is not a live web service.',
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
      <li>Check <code>review-status.json</code> before trusting a prior pass.</li>
      <li>Use <code>review-log.md</code> to record what changed between runs.</li>
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
          <th>Follow-up queued</th>
        </tr>
      </thead>
      <tbody>
        {''.join(rows) if rows else '<tr><td colspan="6">No staged runs yet.</td></tr>'}
      </tbody>
    </table>
  </div>

  <div class="card">
    <h2>Continuity commands</h2>
    <pre>./target/debug/claw --resume latest
./target/debug/claw --resume latest /trace summary .claw/trace/&lt;trace-file&gt;
/trace replay &lt;trace-file|approval-packet&gt;
/trace resume &lt;trace-file|approval-packet&gt;</pre>
    <p>These commands preserve bounded CLI review/resume continuity only. They do <strong>not</strong> imply browser automation, a live dashboard backend, or a cross-run verification service.</p>
  </div>
</body>
</html>
'''
index_html_path.write_text(index_html)
PY

(
  cd "$RUN_DIR"
  find . -type f ! -name "$(basename "$CHECKSUMS")" -print0 \
    | sort -z \
    | xargs -0 sha256sum >"$CHECKSUMS"
)

echo
cat "$TRACE_HINT"
