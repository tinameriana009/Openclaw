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
    'build',
    'verification',
    'artifacts',
]
REQUIRED_ARTIFACT_PATHS = {
    'target/debug/claw',
    'README.md',
    'RELEASE.md',
    'CHANGELOG.md',
    'Cargo.lock',
    'docs/ARTIFACTS.md',
    'docs/PRIVACY.md',
    'docs/RELEASE_CANDIDATE.md',
    'scripts/release-verify.sh',
    'scripts/generate-release-artifact-manifest.sh',
}
REQUIRED_VERIFICATION_COMMANDS = {
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
    if manifest['schemaVersion'] != 2:
        return fail('Manifest schemaVersion must be 2')
    if manifest['compatVersion'] != '0.2':
        return fail('Manifest compatVersion must be 0.2')

    git_section = manifest['git']
    for key in ['commit', 'branch', 'dirty', 'statusShort', 'remotes']:
        if key not in git_section:
            return fail(f'Manifest git section missing key: {key}')
    if not isinstance(git_section['remotes'], list):
        return fail('Manifest git remotes must be a list')

    build_section = manifest['build']
    for key in ['host', 'subject', 'materials']:
        if key not in build_section:
            return fail(f'Manifest build section missing key: {key}')
    subject = build_section['subject']
    for key in ['binary', 'binarySha256']:
        if key not in subject:
            return fail(f'Manifest build subject missing key: {key}')
    if subject['binary'] != 'target/debug/claw':
        return fail('Manifest build subject binary must be target/debug/claw')
    if subject['binarySha256'] != sha256(RUST_ROOT / subject['binary']):
        return fail('Manifest build subject binarySha256 does not match target/debug/claw')

    materials = build_section['materials']
    if not isinstance(materials, list) or not materials:
        return fail('Manifest build materials must be a non-empty list')
    missing_materials = sorted(REQUIRED_ARTIFACT_PATHS - set(materials))
    if missing_materials:
        return fail(f'Manifest build materials missing required entries: {", ".join(missing_materials)}')

    verification = manifest['verification']
    for key in ['model', 'scope', 'commands', 'notes']:
        if key not in verification:
            return fail(f'Manifest verification section missing key: {key}')
    if verification['model'] != 'local-source-build':
        return fail('Manifest verification model must be local-source-build')
    commands = verification['commands']
    if not isinstance(commands, list) or not commands:
        return fail('Manifest verification commands must be a non-empty list')
    missing_commands = sorted(REQUIRED_VERIFICATION_COMMANDS - set(commands))
    if missing_commands:
        return fail(
            f'Manifest verification commands missing required entries: {", ".join(missing_commands)}'
        )
    if not isinstance(verification['notes'], list) or len(verification['notes']) < 2:
        return fail('Manifest verification notes must include at least two explanatory entries')

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
