#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
RUST_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd -- "$RUST_ROOT/.." && pwd)
OUTPUT_DIR=${OUTPUT_DIR:-"$RUST_ROOT/.claw/release-artifacts"}
OUTPUT_FILE=${OUTPUT_FILE:-"$OUTPUT_DIR/release-manifest.json"}
CLAW_BIN=${CLAW_BIN:-"$RUST_ROOT/target/debug/claw"}

mkdir -p "$OUTPUT_DIR"

if [[ ! -f "$CLAW_BIN" ]]; then
  cat <<EOF
ERROR: expected built claw binary at:
  $CLAW_BIN

Build it first:
  cd $RUST_ROOT
  cargo build --workspace --locked
EOF
  exit 2
fi

python3 - "$RUST_ROOT" "$REPO_ROOT" "$CLAW_BIN" "$OUTPUT_FILE" <<'PY'
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
import tomllib

rust_root = Path(sys.argv[1]).resolve()
repo_root = Path(sys.argv[2]).resolve()
claw_bin = Path(sys.argv[3]).resolve()
out_file = Path(sys.argv[4]).resolve()

cargo_toml = tomllib.loads((rust_root / 'Cargo.toml').read_text())
workspace_version = cargo_toml['workspace']['package']['version']
required_toolchain = tomllib.loads((rust_root / 'rust-toolchain.toml').read_text())['toolchain']['channel']

def git(args: list[str]) -> str:
    return subprocess.check_output(['git', *args], cwd=repo_root, text=True).strip()

commit = git(['rev-parse', 'HEAD'])
branch = git(['rev-parse', '--abbrev-ref', 'HEAD'])
status_short = subprocess.check_output(['git', 'status', '--short'], cwd=repo_root, text=True)

def digest(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {
        'path': path.relative_to(rust_root).as_posix(),
        'bytes': len(data),
        'sha256': hashlib.sha256(data).hexdigest(),
    }

artifacts = [
    digest(claw_bin),
    digest(rust_root / 'README.md'),
    digest(rust_root / 'RELEASE.md'),
    digest(rust_root / 'CHANGELOG.md'),
    digest(rust_root / 'docs' / 'ARTIFACTS.md'),
    digest(rust_root / 'docs' / 'PRIVACY.md'),
    digest(rust_root / 'docs' / 'RELEASE_CANDIDATE.md'),
    digest(rust_root / 'scripts' / 'release-verify.sh'),
]

manifest = {
    'artifactKind': 'claw.release-manifest',
    'schemaVersion': 1,
    'compatVersion': '0.1',
    'generatedAt': datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace('+00:00', 'Z'),
    'workspaceVersion': workspace_version,
    'requiredToolchain': required_toolchain,
    'git': {
        'commit': commit,
        'branch': branch,
        'dirty': bool(status_short.strip()),
    },
    'artifacts': artifacts,
}

out_file.write_text(json.dumps(manifest, indent=2) + '\n')
print(out_file)
PY
