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

cargo_version=$(cargo --version 2>/dev/null || true)
rustc_version=$(rustc --version 2>/dev/null || true)

echo "== Release verification preflight =="
echo "rust root: $RUST_ROOT"
echo "required toolchain: $required_toolchain"
echo "cargo: ${cargo_version:-missing}"
echo "rustc: ${rustc_version:-missing}"

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
git branch -vv || true
git remote -v || true

echo
printf '== Locked verification ==\n'
cargo build --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked
cargo test --workspace --locked
./target/debug/claw --help
./target/debug/claw status
