#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
RUST_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd -- "$RUST_ROOT/.." && pwd)
ARTIFACT_ROOT=${ARTIFACT_ROOT:-"$REPO_ROOT/.demo-artifacts/domain-demos"}
TIMESTAMP=$(date -u +"%Y%m%dT%H%M%SZ")
RUN_DIR="$ARTIFACT_ROOT/$TIMESTAMP"
BLENDER_ROOT="$RUN_DIR/blender-demo"
UNREAL_ROOT="$RUN_DIR/unreal-demo"
SUMMARY_JSON="$RUN_DIR/bundle-summary.json"
HANDOFF_JSON="$RUN_DIR/operator-handoff.json"
NEXT_STEPS="$RUN_DIR/next-steps.txt"
CHECKSUMS="$RUN_DIR/bundle-checksums.txt"

if [[ $# -gt 0 ]]; then
  cat <<EOF
This helper does not take positional arguments today.
Optional environment overrides:
  ARTIFACT_ROOT=/custom/output/path

Example:
  cd $RUST_ROOT
  ./scripts/prepare-domain-demo-bundles.sh
EOF
  exit 2
fi

mkdir -p "$RUN_DIR"

echo "== Domain demo bundle prep =="
echo "artifacts: $RUN_DIR"
echo

echo "[1/3] Staging Blender demo bundle..."
ARTIFACT_ROOT="$BLENDER_ROOT" "$SCRIPT_DIR/prepare-blender-demo.sh"

echo
echo "[2/3] Staging Unreal demo bundle..."
ARTIFACT_ROOT="$UNREAL_ROOT" "$SCRIPT_DIR/prepare-unreal-demo.sh"

BLENDER_LATEST=$(find "$BLENDER_ROOT" -mindepth 1 -maxdepth 1 -type d | sort | tail -n 1)
UNREAL_LATEST=$(find "$UNREAL_ROOT" -mindepth 1 -maxdepth 1 -type d | sort | tail -n 1)

cat >"$NEXT_STEPS" <<EOF
Next operator steps:
1. Pick the domain you actually want to validate first:
   - Blender bundle: $BLENDER_LATEST
   - Unreal bundle:  $UNREAL_LATEST
2. Follow that bundle's checklist, findings template, and next-prompt template.
3. Treat both bundles as staged handoff kits, not app automation.
4. Record exact editor/runtime observations before asking for another patch.
5. Use the generated bundle-summary.json / operator-handoff.json files if you need to hand the bundle to another operator.

This helper improves repeatability by staging both domain-demo bundles together.
It does not launch Blender, launch Unreal Editor, run builds, or verify runtime behavior automatically.
EOF

python3 - <<PY
from __future__ import annotations

import json
from pathlib import Path

run_dir = Path(${RUN_DIR@Q})
summary_path = Path(${SUMMARY_JSON@Q})
handoff_path = Path(${HANDOFF_JSON@Q})
blender_latest = Path(${BLENDER_LATEST@Q})
unreal_latest = Path(${UNREAL_LATEST@Q})

summary = {
    'workflow': 'domain-demo-bundles',
    'generatedAtUtc': ${TIMESTAMP@Q},
    'runDir': str(run_dir),
    'bundles': [
        {
            'name': 'blender-demo',
            'bundleRoot': str(blender_latest),
            'automationStatus': 'staged-handoff-only',
            'manualValidationRequired': True,
            'externalToolRequired': 'Blender 4.x',
        },
        {
            'name': 'unreal-demo',
            'bundleRoot': str(unreal_latest),
            'automationStatus': 'staged-handoff-only',
            'manualValidationRequired': True,
            'externalToolRequired': 'Unreal Editor / UnrealBuildTool',
        },
    ],
    'caveats': [
        'This helper only stages both domain demo bundles in one run.',
        'It improves repeatability for operator handoff but does not automate either app.',
    ],
}
summary_path.write_text(json.dumps(summary, indent=2) + '\n')

handoff = {
    'workflow': 'domain-demo-bundles',
    'generatedAtUtc': ${TIMESTAMP@Q},
    'runDir': str(run_dir),
    'operatorNextStep': 'Choose one staged bundle, run the real app/editor validation loop, and capture exact observations before asking for changes.',
    'automationStatus': 'combined-staged-handoff-only',
    'manualValidationRequired': True,
    'bundleRoots': {
        'blender-demo': str(blender_latest),
        'unreal-demo': str(unreal_latest),
    },
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
echo "[3/3] Staged combined domain review bundle."
cat "$NEXT_STEPS"
