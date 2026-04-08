#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
RUST_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd -- "$RUST_ROOT/.." && pwd)
ARTIFACT_ROOT=${ARTIFACT_ROOT:-"$REPO_ROOT/.demo-artifacts/blender-demo"}
TIMESTAMP=$(date -u +"%Y%m%dT%H%M%SZ")
RUN_DIR="$ARTIFACT_ROOT/$TIMESTAMP"
ZIP_SOURCE="$REPO_ROOT/docs/examples/blender-scene-cleanup-demo/dist/scene_cleanup_helper_demo.zip"
ZIP_ARTIFACT="$RUN_DIR/scene_cleanup_helper_demo.zip"
NEXT_STEPS="$RUN_DIR/next-steps.txt"
REPORT_TEMPLATE="$RUN_DIR/operator-findings-template.md"

if [[ $# -gt 0 ]]; then
  cat <<EOF
This helper does not take positional arguments today.
Optional environment overrides:
  ARTIFACT_ROOT=/custom/output/path

Example:
  cd $RUST_ROOT
  ./scripts/prepare-blender-demo.sh
EOF
  exit 2
fi

mkdir -p "$RUN_DIR"

echo "== Blender demo prep =="
echo "artifacts: $RUN_DIR"
echo

echo "[1/3] Validating Blender demo assets..."
(
  cd "$REPO_ROOT"
  python3 tests/validate_blender_demo.py
)

echo
echo "[2/3] Building installable demo zip..."
(
  cd "$REPO_ROOT"
  python3 docs/examples/blender-scene-cleanup-demo/tools/package_demo_addon.py
)

cp "$ZIP_SOURCE" "$ZIP_ARTIFACT"
cp "$REPO_ROOT/docs/examples/blender-scene-cleanup-demo/brief.md" "$RUN_DIR/brief.md"
cp "$REPO_ROOT/docs/examples/blender-scene-cleanup-demo/validation-baseline.md" "$RUN_DIR/validation-baseline.md"
cp "$REPO_ROOT/docs/examples/blender-scene-cleanup-demo/manual-test-checklist.md" "$RUN_DIR/manual-test-checklist.md"

cat >"$REPORT_TEMPLATE" <<'EOF'
# Blender operator findings

## Environment
- Blender version:
- OS:
- Add-on install method: zip / copied package

## Registration result
- [ ] Add-on enabled without traceback
- Notes:

## Baseline scan result
- Duplicate materials count:
- Unapplied transforms count with hidden disabled:
- Unapplied transforms count with hidden enabled:
- Matches validation-baseline.md? yes / no

## UI notes
- Panel/tab visible? yes / no
- Confusing wording or layout:

## Errors to feed back into the next prompt
- Exact traceback:
- Other observations:
EOF

cat >"$NEXT_STEPS" <<EOF
Next operator steps:
1. Install or enable this artifact in Blender:
   $ZIP_ARTIFACT
2. Recreate the disposable scene from validation-baseline.md.
3. Run through manual-test-checklist.md.
4. Record observations in operator-findings-template.md.
5. Feed exact tracebacks, mismatched counts, or confusing UI wording back into the next prompt.

This helper only validates and stages the demo kit.
It does not launch Blender or verify behavior automatically.
EOF

echo
echo "[3/3] Staged operator review bundle."
cat "$NEXT_STEPS"
