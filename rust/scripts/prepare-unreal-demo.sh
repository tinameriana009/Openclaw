#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
RUST_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd -- "$RUST_ROOT/.." && pwd)
ARTIFACT_ROOT=${ARTIFACT_ROOT:-"$REPO_ROOT/.demo-artifacts/unreal-demo"}
TIMESTAMP=$(date -u +"%Y%m%dT%H%M%SZ")
RUN_DIR="$ARTIFACT_ROOT/$TIMESTAMP"
NEXT_STEPS="$RUN_DIR/next-steps.txt"
REPORT_TEMPLATE="$RUN_DIR/operator-findings-template.md"
PLUGIN_BUNDLE_DIR="$RUN_DIR/RuntimeTelemetry"
SUMMARY_JSON="$RUN_DIR/bundle-summary.json"
CHECKSUMS="$RUN_DIR/bundle-checksums.txt"
HANDOFF_JSON="$RUN_DIR/operator-handoff.json"

if [[ $# -gt 0 ]]; then
  cat <<EOF
This helper does not take positional arguments today.
Optional environment overrides:
  ARTIFACT_ROOT=/custom/output/path

Example:
  cd $RUST_ROOT
  ./scripts/prepare-unreal-demo.sh
EOF
  exit 2
fi

mkdir -p "$RUN_DIR"

echo "== Unreal demo prep =="
echo "artifacts: $RUN_DIR"
echo

echo "[1/2] Validating Unreal demo assets..."
(
  cd "$REPO_ROOT"
  python3 tests/validate_unreal_demo.py
)

echo
echo "[2/2] Staging plugin and review docs..."
cp "$REPO_ROOT/docs/examples/unreal-runtime-telemetry-demo/brief.md" "$RUN_DIR/brief.md"
cp "$REPO_ROOT/docs/examples/unreal-runtime-telemetry-demo/expected-findings.md" "$RUN_DIR/expected-findings.md"
cp "$REPO_ROOT/docs/examples/unreal-runtime-telemetry-demo/manual-validation-checklist.md" "$RUN_DIR/manual-validation-checklist.md"
cp "$REPO_ROOT/docs/examples/unreal-runtime-telemetry-demo/error-feedback-playbook.md" "$RUN_DIR/error-feedback-playbook.md"
cp "$REPO_ROOT/docs/examples/unreal-runtime-telemetry-demo/operator-session-template.md" "$RUN_DIR/operator-session-template.md"
cp "$REPO_ROOT/docs/examples/unreal-runtime-telemetry-demo/next-prompt-template.md" "$RUN_DIR/next-prompt-template.md"
cp "$REPO_ROOT/docs/examples/unreal-runtime-telemetry-demo/trace-review-checklist.md" "$RUN_DIR/trace-review-checklist.md"
cp -R "$REPO_ROOT/docs/examples/unreal-runtime-telemetry-demo/plugin/RuntimeTelemetry" "$PLUGIN_BUNDLE_DIR"

cat >"$REPORT_TEMPLATE" <<'EOF'
# Unreal operator findings

## Environment
- Unreal version:
- OS:
- Validation path: IDE build / UBT / Editor compile
- Project path:

## Compile result
- [ ] Project/plugin built successfully
- Build command/path used:
- Exact compiler or UHT errors:

## Editor result
- [ ] Plugin enabled and loaded cleanly
- [ ] RuntimeTelemetry subsystem discoverable
- [ ] Blueprint library nodes visible if expected
- Exact relevant log lines:
- Notes:

## Runtime result
- Event recording behavior observed:
- Flush/log behavior observed:
- Divergence from expected-findings.md:

## Errors to feed back into the next prompt
- Exact logs/errors:
- Other observations:
EOF

cat >"$NEXT_STEPS" <<EOF
Next operator steps:
1. Copy this staged plugin into a disposable Unreal project:
   $PLUGIN_BUNDLE_DIR
2. Read expected-findings.md before asking for changes.
3. Fill in operator-session-template.md while you validate so version, logs, and runtime observations stay exact.
4. Run through manual-validation-checklist.md in your real build/editor loop.
5. Use error-feedback-playbook.md to turn any failure into the next grounded prompt.
6. Use next-prompt-template.md so the follow-up prompt preserves exact environment details, logs, and runtime observations.
7. Use trace-review-checklist.md if a model answer sounds overconfident.
8. Record final exact errors and observations in operator-findings-template.md.

This helper only validates and stages the demo kit.
It does not launch Unreal Editor, run UnrealBuildTool, or verify the plugin automatically.
EOF

cat >"$RUN_DIR/bundle-manifest.txt" <<EOF
brief.md
expected-findings.md
manual-validation-checklist.md
error-feedback-playbook.md
operator-session-template.md
next-prompt-template.md
trace-review-checklist.md
operator-findings-template.md
next-steps.txt
RuntimeTelemetry/
bundle-summary.json
operator-handoff.json
bundle-checksums.txt
EOF

python3 - <<PY
from __future__ import annotations

import json
from pathlib import Path

run_dir = Path(${RUN_DIR@Q})
summary_path = Path(${SUMMARY_JSON@Q})
handoff_path = Path(${HANDOFF_JSON@Q})
manifest_entries = [
    line.strip()
    for line in (run_dir / 'bundle-manifest.txt').read_text().splitlines()
    if line.strip()
]
summary = {
    'workflow': 'unreal-demo',
    'generatedAtUtc': ${TIMESTAMP@Q},
    'runDir': str(run_dir),
    'validatorCommand': 'python3 tests/validate_unreal_demo.py',
    'manualValidationRequired': True,
    'bundleEntries': manifest_entries,
    'externalToolRequired': 'Unreal Editor / UnrealBuildTool',
    'operatorHandoffFiles': [
        'manual-validation-checklist.md',
        'operator-findings-template.md',
        'operator-session-template.md',
        'next-prompt-template.md',
        'next-steps.txt',
    ],
    'caveats': [
        'This helper only validates static demo coherence and stages artifacts.',
        'It does not launch Unreal Editor, UnrealBuildTool, or verify plugin behavior automatically.',
    ],
}
summary_path.write_text(json.dumps(summary, indent=2) + '\n')

handoff = {
    'workflow': 'unreal-demo',
    'generatedAtUtc': ${TIMESTAMP@Q},
    'runDir': str(run_dir),
    'pluginBundle': 'RuntimeTelemetry',
    'operatorChecklist': 'manual-validation-checklist.md',
    'operatorFindingsTemplate': 'operator-findings-template.md',
    'operatorSessionTemplate': 'operator-session-template.md',
    'nextPromptTemplate': 'next-prompt-template.md',
    'operatorNextStep': 'Copy the staged plugin into a disposable Unreal project, run the real build/editor loop, and capture exact logs or compiler errors before asking for changes.',
    'automationStatus': 'staged-handoff-only',
    'manualValidationRequired': True,
    'externalToolRequired': 'Unreal Editor / UnrealBuildTool',
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
echo "Staged operator review bundle."
cat "$NEXT_STEPS"
