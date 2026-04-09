#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
RUST_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd -- "$RUST_ROOT/.." && pwd)
OUTPUT_DIR=${OUTPUT_DIR:-"$RUST_ROOT/.claw/release-artifacts"}
OUTPUT_FILE=${OUTPUT_FILE:-"$OUTPUT_DIR/release-manifest.json"}
ATTESTATION_FILE=${ATTESTATION_FILE:-"$OUTPUT_DIR/release-attestation.json"}
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

python3 - "$RUST_ROOT" "$REPO_ROOT" "$CLAW_BIN" "$OUTPUT_FILE" "$ATTESTATION_FILE" <<'PY'
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
attestation_file = Path(sys.argv[5]).resolve()

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


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


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
    'python3 ../tests/validate_release_attestation.py <attestation-path> <manifest-path>',
]

optional_signing_commands = [
    './scripts/sign-release-provenance.sh',
    'python3 ../tests/validate_signed_release_provenance.py <provenance-path> <signature-path> <public-key-path> [trust-policy-path]',
    'python3 ../tests/validate_release_trust_policy.py <policy-path> <provenance-path> <signature-path> <public-key-path> <manifest-path> <attestation-path>',
]
optional_rooted_signing_hint = (
    'For a bounded external/rooted variant, also provide PROVENANCE_SIGNING_CERT and '
    'PROVENANCE_TRUST_ROOT (optionally PROVENANCE_SIGNING_CHAIN) so the signed bundle pins an X.509 chain back to the supplied root.'
)

rc_note = (
    'For RC discipline, also run RELEASE_CANDIDATE=1 ./scripts/release-verify.sh '
    'or at minimum python3 ../tests/validate_release_candidate_readiness.py.'
)


def digest(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {
        'path': path.relative_to(rust_root).as_posix(),
        'bytes': len(data),
        'sha256': sha256_bytes(data),
    }


binary_sha256 = sha256_file(claw_bin)
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
            'It is intentionally local-first and should not be treated as a hosted or transparency-backed attestation chain.',
            'A paired release-attestation.json binds this manifest hash into a more formal statement shape; optional signing can extend that into a signed local provenance bundle.',
            'If you set PROVENANCE_SIGNING_KEY, also run ' + optional_signing_commands[0] + ', ' + optional_signing_commands[1] + ', and ' + optional_signing_commands[2] + '.',
            optional_rooted_signing_hint,
            rc_note,
        ],
    },
    'artifacts': artifacts,
}

manifest_json = json.dumps(manifest, indent=2) + '\n'
out_file.write_text(manifest_json)
manifest_sha256 = sha256_bytes(manifest_json.encode())
attestation = {
    'artifactKind': 'claw.release-attestation',
    'schemaVersion': 1,
    'compatVersion': '0.1',
    '_type': 'https://in-toto.io/Statement/v1',
    'predicateType': 'https://claw.dev/attestation/local-source-build/v1',
    'generatedAt': manifest['generatedAt'],
    'subject': [
        {
            'name': 'target/debug/claw',
            'digest': {'sha256': binary_sha256},
        },
        {
            'name': out_file.relative_to(rust_root).as_posix(),
            'digest': {'sha256': manifest_sha256},
        },
    ],
    'predicate': {
        'buildDefinition': {
            'buildType': 'local-source-build',
            'externalParameters': {
                'workspaceVersion': workspace_version,
                'requiredToolchain': required_toolchain,
                'verificationCommands': verification_commands,
            },
            'internalParameters': {
                'releaseCandidateModeSupported': True,
                'sourceRoot': repo_root.as_posix(),
                'rustRoot': rust_root.as_posix(),
            },
            'resolvedDependencies': [
                {
                    'uri': f'git+file://{repo_root.as_posix()}',
                    'digest': {'gitCommit': commit},
                    'annotations': {
                        'branch': branch,
                        'dirty': str(bool(status_short.strip())).lower(),
                    },
                },
                {
                    'uri': 'file://Cargo.lock',
                    'digest': {'sha256': sha256_file(rust_root / 'Cargo.lock')},
                },
                {
                    'uri': 'file://scripts/generate-release-artifact-manifest.sh',
                    'digest': {'sha256': sha256_file(rust_root / 'scripts' / 'generate-release-artifact-manifest.sh')},
                },
            ],
        },
        'runDetails': {
            'builder': {
                'id': 'claw.local.release-verify',
                'version': {
                    'workspace': workspace_version,
                    'cargo': cargo_version,
                    'rustc': rustc_version,
                    'python': python_version,
                },
            },
            'metadata': {
                'invocationId': f'{commit[:12]}:{manifest["generatedAt"]}',
                'startedOn': manifest['generatedAt'],
                'finishedOn': manifest['generatedAt'],
            },
            'byproducts': [
                {
                    'name': 'release-manifest',
                    'path': out_file.relative_to(rust_root).as_posix(),
                    'sha256': manifest_sha256,
                },
                {
                    'name': 'release-materials',
                    'count': len(artifacts),
                },
            ],
        },
        'notes': [
            'This statement intentionally improves structure, not cryptographic trust.',
            'It is unsigned local provenance and must be paired with source review and local verification.',
        ],
    },
}
attestation_file.write_text(json.dumps(attestation, indent=2) + '\n')
print(out_file)
PY
