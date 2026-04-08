from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = REPO_ROOT / 'rust'
DEFAULT_ATTESTATION = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-attestation.json'
DEFAULT_MANIFEST = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-manifest.json'
REQUIRED_COMMAND = 'python3 ../tests/validate_release_attestation.py <attestation-path> <manifest-path>'


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fail(message: str) -> int:
    print(message)
    return 1


def main() -> int:
    attestation_path = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT_ATTESTATION
    manifest_path = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else DEFAULT_MANIFEST

    if not attestation_path.exists():
        return fail(f'Attestation does not exist: {attestation_path}')
    if not manifest_path.exists():
        return fail(f'Manifest does not exist: {manifest_path}')

    attestation = json.loads(attestation_path.read_text())
    manifest = json.loads(manifest_path.read_text())

    if attestation.get('artifactKind') != 'claw.release-attestation':
        return fail('Attestation artifactKind must be claw.release-attestation')
    if attestation.get('schemaVersion') != 1:
        return fail('Attestation schemaVersion must be 1')
    if attestation.get('compatVersion') != '0.1':
        return fail('Attestation compatVersion must be 0.1')
    if attestation.get('_type') != 'https://in-toto.io/Statement/v1':
        return fail('Attestation _type must be https://in-toto.io/Statement/v1')
    if attestation.get('predicateType') != 'https://claw.dev/attestation/local-source-build/v1':
        return fail('Attestation predicateType must be https://claw.dev/attestation/local-source-build/v1')

    subject = attestation.get('subject')
    if not isinstance(subject, list) or len(subject) < 2:
        return fail('Attestation subject must contain at least binary and manifest subjects')

    subject_by_name = {item.get('name'): item for item in subject if isinstance(item, dict)}
    binary_subject = subject_by_name.get('target/debug/claw')
    manifest_name = manifest_path.relative_to(RUST_ROOT).as_posix()
    manifest_subject = subject_by_name.get(manifest_name)
    if binary_subject is None:
        return fail('Attestation subject is missing target/debug/claw')
    if manifest_subject is None:
        return fail(f'Attestation subject is missing {manifest_name}')

    binary_path = RUST_ROOT / 'target' / 'debug' / 'claw'
    if binary_subject.get('digest', {}).get('sha256') != sha256_bytes(binary_path.read_bytes()):
        return fail('Attestation binary subject sha256 does not match target/debug/claw')
    if manifest_subject.get('digest', {}).get('sha256') != sha256_bytes(manifest_path.read_bytes()):
        return fail('Attestation manifest subject sha256 does not match release-manifest.json')

    predicate = attestation.get('predicate')
    if not isinstance(predicate, dict):
        return fail('Attestation predicate must be an object')
    build_definition = predicate.get('buildDefinition')
    run_details = predicate.get('runDetails')
    if not isinstance(build_definition, dict):
        return fail('Attestation predicate.buildDefinition must be an object')
    if not isinstance(run_details, dict):
        return fail('Attestation predicate.runDetails must be an object')

    if build_definition.get('buildType') != 'local-source-build':
        return fail('Attestation buildDefinition.buildType must be local-source-build')
    external_parameters = build_definition.get('externalParameters')
    if not isinstance(external_parameters, dict):
        return fail('Attestation externalParameters must be an object')
    commands = external_parameters.get('verificationCommands')
    if not isinstance(commands, list) or REQUIRED_COMMAND not in commands:
        return fail('Attestation verificationCommands must include the attestation validator command')

    if external_parameters.get('workspaceVersion') != manifest.get('workspaceVersion'):
        return fail('Attestation workspaceVersion must match the manifest workspaceVersion')
    if external_parameters.get('requiredToolchain') != manifest.get('requiredToolchain'):
        return fail('Attestation requiredToolchain must match the manifest requiredToolchain')

    dependencies = build_definition.get('resolvedDependencies')
    if not isinstance(dependencies, list) or len(dependencies) < 3:
        return fail('Attestation resolvedDependencies must include repo, Cargo.lock, and generator script')
    dependency_uris = {item.get('uri') for item in dependencies if isinstance(item, dict)}
    for required_uri in {
        'file://Cargo.lock',
        'file://scripts/generate-release-artifact-manifest.sh',
    }:
        if required_uri not in dependency_uris:
            return fail(f'Attestation resolvedDependencies missing {required_uri}')

    builder = run_details.get('builder')
    metadata = run_details.get('metadata')
    byproducts = run_details.get('byproducts')
    if not isinstance(builder, dict) or builder.get('id') != 'claw.local.release-verify':
        return fail('Attestation builder.id must be claw.local.release-verify')
    if not isinstance(metadata, dict):
        return fail('Attestation metadata must be an object')
    if metadata.get('startedOn') != attestation.get('generatedAt'):
        return fail('Attestation metadata.startedOn must match generatedAt')
    if metadata.get('finishedOn') != attestation.get('generatedAt'):
        return fail('Attestation metadata.finishedOn must match generatedAt')
    if not isinstance(byproducts, list) or not byproducts:
        return fail('Attestation byproducts must be a non-empty list')

    byproduct_names = {item.get('name') for item in byproducts if isinstance(item, dict)}
    if 'release-manifest' not in byproduct_names:
        return fail('Attestation byproducts must include release-manifest')

    notes = predicate.get('notes')
    if not isinstance(notes, list) or len(notes) < 2:
        return fail('Attestation notes must include at least two trust-boundary reminders')

    print(f'Release attestation validation passed: {attestation_path}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
