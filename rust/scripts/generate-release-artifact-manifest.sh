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
import platform
import shutil
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


def git_lines(args: list[str]) -> list[str]:
    output = subprocess.check_output(['git', *args], cwd=repo_root, text=True)
    return [line.strip() for line in output.splitlines() if line.strip()]


commit = git(['rev-parse', 'HEAD'])
branch = git(['rev-parse', '--abbrev-ref', 'HEAD'])
status_short = subprocess.check_output(['git', 'status', '--short'], cwd=repo_root, text=True)

remote_lines = git_lines(['remote', '-v'])
remotes = []
seen_remotes: set[tuple[str, str, str]] = set()
for line in remote_lines:
    parts = line.split()
    if len(parts) < 3:
        continue
    name, url, kind = parts[0], parts[1], parts[2].strip('()')
    key = (name, url, kind)
    if key not in seen_remotes:
        seen_remotes.add(key)
        remotes.append({'name': name, 'url': url, 'kind': kind})


def command_output(argv: list[str]) -> str | None:
    try:
        return subprocess.check_output(argv, text=True).strip()
    except (OSError, subprocess.CalledProcessError):
        return None


cargo_version = command_output(['cargo', '--version'])
rustc_version = command_output(['rustc', '--version'])
python_version = sys.version.split()[0]
sha256sum_available = shutil.which('sha256sum') is not None

verification_commands = [
    'cargo build --workspace --locked',
    'cargo fmt --all --check',
    'cargo clippy --workspace --all-targets --locked',
    'cargo test --workspace --locked',
    './target/debug/claw --help',
    './target/debug/claw status',
    'python3 ../tests/validate_operator_readiness.py',
    'python3 ../tests/validate_blender_demo.py',
    'python3 ../tests/validate_unreal_demo.py',
    'python3 ../tests/validate_repo_analysis_demo.py',
    'python3 ../tests/validate_release_artifact_manifest.py <manifest-path>',
]

rc_note = (
    'For RC discipline, also run RELEASE_CANDIDATE=1 ./scripts/release-verify.sh '
    'or at minimum python3 ../tests/validate_release_candidate_readiness.py.'
)


def digest(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {
        'path': path.relative_to(rust_root).as_posix(),
        'bytes': len(data),
        'sha256': hashlib.sha256(data).hexdigest(),
    }


binary_sha256 = hashlib.sha256(claw_bin.read_bytes()).hexdigest()
artifacts = [
    digest(claw_bin),
    digest(rust_root / 'README.md'),
    digest(rust_root / 'RELEASE.md'),
    digest(rust_root / 'CHANGELOG.md'),
    digest(rust_root / 'Cargo.lock'),
    digest(rust_root / 'docs' / 'ARTIFACTS.md'),
    digest(rust_root / 'docs' / 'PRIVACY.md'),
    digest(rust_root / 'docs' / 'RELEASE_CANDIDATE.md'),
    digest(rust_root / 'scripts' / 'release-verify.sh'),
    digest(rust_root / 'scripts' / 'generate-release-artifact-manifest.sh'),
]

manifest = {
    'artifactKind': 'claw.release-manifest',
    'schemaVersion': 2,
    'compatVersion': '0.2',
    'generatedAt': datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace('+00:00', 'Z'),
    'workspaceVersion': workspace_version,
    'requiredToolchain': required_toolchain,
    'git': {
        'commit': commit,
        'branch': branch,
        'dirty': bool(status_short.strip()),
        'statusShort': status_short.splitlines(),
        'remotes': remotes,
    },
    'build': {
        'host': {
            'os': platform.system(),
            'release': platform.release(),
            'machine': platform.machine(),
            'python': python_version,
            'cargo': cargo_version,
            'rustc': rustc_version,
            'sha256sumAvailable': sha256sum_available,
        },
        'subject': {
            'binary': claw_bin.relative_to(rust_root).as_posix(),
            'binarySha256': binary_sha256,
        },
        'materials': [
            'target/debug/claw',
            'Cargo.lock',
            'README.md',
            'RELEASE.md',
            'CHANGELOG.md',
            'docs/ARTIFACTS.md',
            'docs/PRIVACY.md',
            'docs/RELEASE_CANDIDATE.md',
            'scripts/release-verify.sh',
            'scripts/generate-release-artifact-manifest.sh',
        ],
    },
    'verification': {
        'model': 'local-source-build',
        'scope': 'workspace binary plus operator-facing release/trust docs',
        'commands': verification_commands,
        'notes': [
            'This manifest records local provenance cues and hashed release surfaces for the current workspace.',
            'It is intentionally unsigned and should not be treated as a portable attestation chain.',
            rc_note,
        ],
    },
    'artifacts': artifacts,
}

out_file.write_text(json.dumps(manifest, indent=2) + '\n')
print(out_file)
PY
