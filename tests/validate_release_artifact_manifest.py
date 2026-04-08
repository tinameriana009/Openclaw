from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = REPO_ROOT / 'rust'
DEFAULT_MANIFEST = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-manifest.json'
REQUIRED_TOP_LEVEL = [
    'artifactKind',
    'schemaVersion',
    'compatVersion',
    'generatedAt',
    'workspaceVersion',
    'requiredToolchain',
    'git',
    'artifacts',
]
REQUIRED_ARTIFACT_PATHS = {
    'target/debug/claw',
    'README.md',
    'RELEASE.md',
    'CHANGELOG.md',
    'docs/ARTIFACTS.md',
    'docs/PRIVACY.md',
    'docs/RELEASE_CANDIDATE.md',
    'scripts/release-verify.sh',
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def fail(message: str) -> int:
    print(message)
    return 1


def main() -> int:
    manifest_path = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT_MANIFEST
    if not manifest_path.exists():
        return fail(f'Manifest does not exist: {manifest_path}')

    manifest = json.loads(manifest_path.read_text())

    for key in REQUIRED_TOP_LEVEL:
        if key not in manifest:
            return fail(f'Manifest missing top-level key: {key}')

    if manifest['artifactKind'] != 'claw.release-manifest':
        return fail('Manifest artifactKind must be claw.release-manifest')
    if manifest['schemaVersion'] != 1:
        return fail('Manifest schemaVersion must be 1')
    if manifest['compatVersion'] != '0.1':
        return fail('Manifest compatVersion must be 0.1')

    git_section = manifest['git']
    for key in ['commit', 'branch', 'dirty']:
        if key not in git_section:
            return fail(f'Manifest git section missing key: {key}')

    artifacts = manifest['artifacts']
    if not isinstance(artifacts, list) or not artifacts:
        return fail('Manifest artifacts must be a non-empty list')

    seen_paths: set[str] = set()
    for item in artifacts:
        for key in ['path', 'bytes', 'sha256']:
            if key not in item:
                return fail(f'Artifact entry missing key: {key}')
        rel_path = item['path']
        seen_paths.add(rel_path)
        candidate = (RUST_ROOT / rel_path).resolve()
        if not candidate.exists():
            return fail(f'Manifest references missing artifact: {rel_path}')
        actual_bytes = candidate.stat().st_size
        actual_sha256 = sha256(candidate)
        if item['bytes'] != actual_bytes:
            return fail(
                f'Artifact byte mismatch for {rel_path}: manifest={item["bytes"]} actual={actual_bytes}'
            )
        if item['sha256'] != actual_sha256:
            return fail(
                f'Artifact sha256 mismatch for {rel_path}: manifest={item["sha256"]} actual={actual_sha256}'
            )

    missing_paths = sorted(REQUIRED_ARTIFACT_PATHS - seen_paths)
    if missing_paths:
        return fail(f'Manifest missing required artifact paths: {", ".join(missing_paths)}')

    print(f'Release artifact manifest validation passed: {manifest_path}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
