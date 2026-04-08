#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
RUST_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)

cd "$RUST_ROOT"

required_toolchain=$(python3 - <<'PY'
from pathlib import Path
import tomllib
cfg = tomllib.loads(Path('rust-toolchain.toml').read_text())
print(cfg['toolchain']['channel'])
PY
)

workspace_version=$(python3 - <<'PY'
from pathlib import Path
import tomllib
cfg = tomllib.loads(Path('Cargo.toml').read_text())
print(cfg['workspace']['package']['version'])
PY
)

cargo_version=$(cargo --version 2>/dev/null || true)
rustc_version=$(rustc --version 2>/dev/null || true)
release_candidate=${RELEASE_CANDIDATE:-0}

echo "== Release verification preflight =="
echo "rust root: $RUST_ROOT"
echo "workspace version: $workspace_version"
echo "required toolchain: $required_toolchain"
echo "cargo: ${cargo_version:-missing}"
echo "rustc: ${rustc_version:-missing}"
echo "release candidate discipline: $release_candidate"

toolchain_mismatch=0
if [[ -z "$cargo_version" || "$cargo_version" != cargo\ "$required_toolchain"* ]]; then
  toolchain_mismatch=1
fi
if [[ -z "$rustc_version" || "$rustc_version" != rustc\ "$required_toolchain"* ]]; then
  toolchain_mismatch=1
fi

if [[ $toolchain_mismatch -ne 0 ]]; then
  cat <<EOF
ERROR: active Rust toolchain does not match rust-toolchain.toml.
This workspace currently expects Rust $required_toolchain.

Recommended fix:
  rustup toolchain install $required_toolchain
  rustup override set $required_toolchain

Then rerun:
  ./scripts/release-verify.sh
EOF
  exit 2
fi

echo
printf '== Repository posture ==\n'
git status --short
current_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)
echo "current branch: $current_branch"
git branch -vv || true
git remote -v || true

if [[ "$release_candidate" == "1" ]]; then
  if [[ -n "$(git status --short)" ]]; then
    echo
    echo "ERROR: release-candidate verification requires a clean working tree."
    echo "Commit/stash local changes or rerun without RELEASE_CANDIDATE=1 for a non-RC smoke pass."
    exit 3
  fi
  if [[ "$current_branch" != release/* && "$current_branch" != hotfix/* && "$current_branch" != main ]]; then
    echo
    echo "WARN: RC verification is usually run from main, release/*, or hotfix/* branches."
    echo "Current branch: $current_branch"
  fi
fi

echo
printf '== Locked verification ==\n'
cargo build --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked
cargo test --workspace --locked
./target/debug/claw --help
./target/debug/claw status

echo
printf '== Operator readiness + demo validation ==\n'
python3 ../tests/validate_operator_readiness.py
python3 ../tests/validate_blender_demo.py
python3 ../tests/validate_unreal_demo.py
python3 ../tests/validate_repo_analysis_demo.py

if [[ "$release_candidate" == "1" ]]; then
  echo
  printf '== Release-candidate documentation gate ==\n'
  python3 ../tests/validate_release_candidate_readiness.py
fi

echo
printf '== Artifact contract spot-check ==\n'
python3 - <<'PY'
from pathlib import Path
text = Path('docs/ARTIFACTS.md').read_text()
required = ['schemaVersion', 'compatVersion', 'artifactKind']
missing = [item for item in required if item not in text]
if missing:
    raise SystemExit(f"docs/ARTIFACTS.md is missing required artifact contract markers: {', '.join(missing)}")
print('docs/ARTIFACTS.md mentions artifactKind/schemaVersion/compatVersion')
PY

manifest_path=$(./scripts/generate-release-artifact-manifest.sh)
attestation_path=.claw/release-artifacts/release-attestation.json
echo "release manifest: $manifest_path"
echo "release attestation: $attestation_path"
python3 ../tests/validate_release_artifact_manifest.py "$manifest_path"
python3 ../tests/validate_release_attestation.py "$attestation_path" "$manifest_path"

if [[ -n "${PROVENANCE_SIGNING_KEY:-}" ]]; then
  echo
  printf '== Optional signed provenance ==\n'
  provenance_path=$(./scripts/sign-release-provenance.sh)
  signature_path=.claw/release-artifacts/release-provenance.sig
  public_key_path=.claw/release-artifacts/release-provenance.pub.pem
  echo "signed provenance: $provenance_path"
  echo "signed provenance signature: $signature_path"
  echo "signed provenance public key: $public_key_path"
  python3 ../tests/validate_signed_release_provenance.py "$provenance_path" "$signature_path" "$public_key_path"
else
  echo "optional signed provenance: skipped (set PROVENANCE_SIGNING_KEY to emit release-provenance.json + .sig)"
fi

if [[ "$release_candidate" == "1" ]]; then
  echo
  printf '== RC reminders ==\n'
  echo "- confirm CHANGELOG.md and RELEASE.md match actual operator behavior"
  echo "- call out artifact compatibility notes in the release draft"
  echo "- if local .claw state from older runs exists, test with a fresh workspace too"
fi
