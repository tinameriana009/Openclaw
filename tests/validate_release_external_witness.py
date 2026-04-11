from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = REPO_ROOT / 'rust'
DEFAULT_WITNESS = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-external-witness.json'
DEFAULT_MANIFEST = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-manifest.json'
DEFAULT_ATTESTATION = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-attestation.json'
DEFAULT_PROVENANCE = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-provenance.json'
DEFAULT_SIGNATURE = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-provenance.sig'
DEFAULT_TRUST_POLICY = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-trust-policy.json'


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fail(message: str) -> int:
    print(message)
    return 1


def git(args: list[str]) -> str:
    return subprocess.check_output(['git', *args], cwd=REPO_ROOT, text=True).strip()


def main() -> int:
    witness_path = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT_WITNESS
    manifest_path = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else DEFAULT_MANIFEST
    attestation_path = Path(sys.argv[3]).resolve() if len(sys.argv) > 3 else DEFAULT_ATTESTATION
    provenance_path = Path(sys.argv[4]).resolve() if len(sys.argv) > 4 else DEFAULT_PROVENANCE
    signature_path = Path(sys.argv[5]).resolve() if len(sys.argv) > 5 else DEFAULT_SIGNATURE
    trust_policy_path = Path(sys.argv[6]).resolve() if len(sys.argv) > 6 else DEFAULT_TRUST_POLICY

    for path, label in [
        (witness_path, 'Release external witness'),
        (manifest_path, 'Release manifest'),
        (attestation_path, 'Release attestation'),
        (provenance_path, 'Signed provenance'),
        (signature_path, 'Signed provenance signature'),
        (trust_policy_path, 'Release trust policy'),
    ]:
        if not path.exists():
            return fail(f'{label} does not exist: {path}')

    witness = json.loads(witness_path.read_text())
    manifest = json.loads(manifest_path.read_text())
    attestation = json.loads(attestation_path.read_text())
    provenance = json.loads(provenance_path.read_text())
    trust_policy = json.loads(trust_policy_path.read_text())

    if witness.get('artifactKind') != 'claw.release-external-witness':
        return fail('Release external witness artifactKind must be claw.release-external-witness')
    if witness.get('schemaVersion') != 1:
        return fail('Release external witness schemaVersion must be 1')
    if witness.get('compatVersion') != '0.1':
        return fail('Release external witness compatVersion must be 0.1')

    subject = witness.get('subject')
    if not isinstance(subject, list) or len(subject) != 5:
        return fail('Release external witness subject must include manifest, attestation, provenance, signature, and trust policy')

    expected = {
        manifest_path.relative_to(RUST_ROOT).as_posix(): manifest_path,
        attestation_path.relative_to(RUST_ROOT).as_posix(): attestation_path,
        provenance_path.relative_to(RUST_ROOT).as_posix(): provenance_path,
        signature_path.relative_to(RUST_ROOT).as_posix(): signature_path,
        trust_policy_path.relative_to(RUST_ROOT).as_posix(): trust_policy_path,
    }
    by_name = {item.get('name'): item for item in subject if isinstance(item, dict)}
    for name, path in expected.items():
        item = by_name.get(name)
        if item is None:
            return fail(f'Release external witness subject missing {name}')
        if item.get('digest', {}).get('sha256') != sha256_bytes(path.read_bytes()):
            return fail(f'Release external witness digest mismatch for {name}')
        if item.get('bytes') != path.stat().st_size:
            return fail(f'Release external witness byte count mismatch for {name}')

    predicate = witness.get('predicate')
    if not isinstance(predicate, dict):
        return fail('Release external witness predicate must be an object')

    source = predicate.get('source')
    if not isinstance(source, dict):
        return fail('Release external witness predicate.source must be an object')
    manifest_git = manifest.get('git', {})
    head_commit = git(['rev-parse', 'HEAD'])
    if source.get('commit') != manifest_git.get('commit') or source.get('commit') != head_commit:
        return fail('Release external witness source.commit must match manifest git.commit and current HEAD')
    tree = git(['rev-parse', 'HEAD^{tree}'])
    if source.get('tree') != tree:
        return fail('Release external witness source.tree must match current HEAD tree')

    anchors = predicate.get('anchors')
    if not isinstance(anchors, list) or not anchors:
        return fail('Release external witness predicate.anchors must be a non-empty list')
    anchor_kinds = {anchor.get('kind') for anchor in anchors if isinstance(anchor, dict)}
    if 'git-commit' not in anchor_kinds:
        return fail('Release external witness anchors must include a git-commit anchor')
    if not ({'repository', 'git-tag'} & anchor_kinds or predicate.get('publications')):
        return fail('Release external witness must include a repository/tag anchor or explicit publication URLs')

    for anchor in anchors:
        if not isinstance(anchor, dict):
            return fail('Release external witness anchors must contain only objects')
        kind = anchor.get('kind')
        if kind == 'git-commit':
            if anchor.get('commit') != head_commit:
                return fail('Release external witness git-commit anchor commit must match HEAD')
            if anchor.get('tree') != tree:
                return fail('Release external witness git-commit anchor tree must match HEAD^{tree}')
            if anchor.get('remotes') != manifest_git.get('remotes'):
                return fail('Release external witness git-commit anchor remotes must match manifest git.remotes')
        elif kind == 'repository':
            for key in ['repositoryUrl', 'commitUrl', 'treeUrl']:
                value = anchor.get(key)
                if not isinstance(value, str) or not value.startswith('http'):
                    return fail(f'Release external witness repository anchor {key} must be an http(s) URL')
        elif kind == 'git-tag':
            tag = anchor.get('tag')
            if not isinstance(tag, str) or not tag:
                return fail('Release external witness git-tag anchor tag must be a non-empty string')
            tag_commit = git(['rev-list', '-n', '1', tag])
            if tag_commit != head_commit or anchor.get('commit') != head_commit:
                return fail('Release external witness git-tag anchor must resolve to current HEAD')
        else:
            return fail(f'Release external witness anchor kind is not recognized: {kind}')

    publications = predicate.get('publications')
    if publications is not None and not isinstance(publications, dict):
        return fail('Release external witness predicate.publications must be an object when present')
    if isinstance(publications, dict):
        for key, value in publications.items():
            if not isinstance(value, str) or not value:
                return fail(f'Release external witness publication {key} must be a non-empty string')
            if key.endswith('Url') and not value.startswith('http'):
                return fail(f'Release external witness publication {key} must be an http(s) URL')

    material_binding = predicate.get('materialBinding')
    if not isinstance(material_binding, dict):
        return fail('Release external witness predicate.materialBinding must be an object')
    if material_binding.get('manifestArtifactKind') != manifest.get('artifactKind'):
        return fail('Release external witness materialBinding.manifestArtifactKind must match the manifest')
    if material_binding.get('attestationArtifactKind') != attestation.get('artifactKind'):
        return fail('Release external witness materialBinding.attestationArtifactKind must match the attestation')
    if material_binding.get('provenanceArtifactKind') != provenance.get('artifactKind'):
        return fail('Release external witness materialBinding.provenanceArtifactKind must match the signed provenance')
    if material_binding.get('trustPolicyArtifactKind') != trust_policy.get('artifactKind'):
        return fail('Release external witness materialBinding.trustPolicyArtifactKind must match the trust policy')
    if material_binding.get('provenanceSha256') != sha256_bytes(provenance_path.read_bytes()):
        return fail('Release external witness materialBinding.provenanceSha256 must match the signed provenance')
    if material_binding.get('signatureSha256') != sha256_bytes(signature_path.read_bytes()):
        return fail('Release external witness materialBinding.signatureSha256 must match the provenance signature')
    if material_binding.get('trustPolicySha256') != sha256_bytes(trust_policy_path.read_bytes()):
        return fail('Release external witness materialBinding.trustPolicySha256 must match the trust policy')

    verification = predicate.get('verification')
    if not isinstance(verification, dict):
        return fail('Release external witness predicate.verification must be an object')
    commands = verification.get('commands')
    required_command = (
        'python3 ../tests/validate_release_external_witness.py '
        '<witness-path> <manifest-path> <attestation-path> <provenance-path> <signature-path> <trust-policy-path>'
    )
    if not isinstance(commands, list) or required_command not in commands:
        return fail('Release external witness verification.commands must include the external witness validator command')
    notes = verification.get('notes')
    if not isinstance(notes, list) or len(notes) < 4:
        return fail('Release external witness verification.notes must include at least four trust-boundary reminders')

    print(f'Release external witness validation passed: {witness_path}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
