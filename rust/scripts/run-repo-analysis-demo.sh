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

## Recommended next prompt
- 
EOF

cat >"$TRACE_HINT" <<EOF
Review steps for this run:
1. Compare the answers against:
   - docs/examples/repo-analysis-demo/expected-findings.md
   - docs/examples/repo-analysis-demo/manual-validation-checklist.md
2. Capture exact evidence, weak spots, and handoff notes in:
   $SESSION_TEMPLATE
   $REPORT_TEMPLATE
3. If the answers sound overconfident, inspect traces from rust/.claw/trace/ using:
   ./target/debug/claw --resume latest /trace summary .claw/trace/<trace-file>
4. Use the staged next-prompt template for the next narrowed ask:
   $NEXT_PROMPT_TEMPLATE
5. Re-run the demo validator if you changed docs/assets:
   python3 tests/validate_repo_analysis_demo.py

This helper only runs the documented prompt flow and captures outputs.
It does not certify answer quality or verify the repository automatically.
EOF

cat >"$RUN_DIR/bundle-manifest.txt" <<EOF
01-brief-response.txt
02-followup-response.txt
run-metadata.txt
operator-session-template.md
next-prompt-template.md
operator-findings-template.md
next-steps.txt
EOF

echo
cat "$TRACE_HINT"
