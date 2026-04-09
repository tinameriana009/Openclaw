from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = REPO_ROOT / 'rust'
DEFAULT_POLICY = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-trust-policy.json'
DEFAULT_PROVENANCE = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-provenance.json'
DEFAULT_SIGNATURE = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-provenance.sig'
DEFAULT_PUBLIC_KEY = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-provenance.pub.pem'
DEFAULT_MANIFEST = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-manifest.json'
DEFAULT_ATTESTATION = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-attestation.json'


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fail(message: str) -> int:
    print(message)
    return 1


def verify_signature(provenance_path: Path, signature_path: Path, public_key_path: Path) -> None:
    subprocess.run(
        [
            'openssl',
            'dgst',
            '-sha256',
            '-verify',
            str(public_key_path),
            '-signature',
            str(signature_path),
            str(provenance_path),
        ],
        check=True,
        capture_output=True,
        text=True,
    )


def verify_public_key_matches_certificate(public_key_path: Path, certificate_path: Path) -> None:
    cert_pub = subprocess.check_output(
        ['openssl', 'x509', '-in', str(certificate_path), '-pubkey', '-noout'],
        text=False,
    )
    cert_der = subprocess.check_output(
        ['openssl', 'pkey', '-pubin', '-outform', 'DER'],
        input=cert_pub,
        text=False,
    )
    key_der = subprocess.check_output(
        ['openssl', 'pkey', '-pubin', '-in', str(public_key_path), '-outform', 'DER'],
        text=False,
    )
    if cert_der != key_der:
        raise ValueError('signing certificate public key does not match the pinned public key')


def verify_certificate_chain(certificate_path: Path, trust_root_path: Path, chain_path: Path | None) -> None:
    argv = ['openssl', 'verify', '-CAfile', str(trust_root_path)]
    if chain_path is not None:
        argv.extend(['-untrusted', str(chain_path)])
    argv.append(str(certificate_path))
    subprocess.run(argv, check=True, capture_output=True, text=True)


def main() -> int:
    policy_path = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT_POLICY
    provenance_path = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else DEFAULT_PROVENANCE
    signature_path = Path(sys.argv[3]).resolve() if len(sys.argv) > 3 else DEFAULT_SIGNATURE
    public_key_path = Path(sys.argv[4]).resolve() if len(sys.argv) > 4 else DEFAULT_PUBLIC_KEY
    manifest_path = Path(sys.argv[5]).resolve() if len(sys.argv) > 5 else DEFAULT_MANIFEST
    attestation_path = Path(sys.argv[6]).resolve() if len(sys.argv) > 6 else DEFAULT_ATTESTATION

    for path, label in [
        (policy_path, 'Release trust policy'),
        (provenance_path, 'Signed provenance'),
        (signature_path, 'Signed provenance signature'),
        (public_key_path, 'Signed provenance public key'),
        (manifest_path, 'Release manifest'),
        (attestation_path, 'Release attestation'),
    ]:
        if not path.exists():
            return fail(f'{label} does not exist: {path}')

    try:
        verify_signature(provenance_path, signature_path, public_key_path)
    except FileNotFoundError:
        return fail('openssl is required to verify the release trust policy chain')
    except subprocess.CalledProcessError as exc:
        return fail(f'Release trust policy signature verification failed: {exc.stderr.strip() or exc.stdout.strip()}')

    policy = json.loads(policy_path.read_text())
    provenance = json.loads(provenance_path.read_text())
    manifest = json.loads(manifest_path.read_text())
    attestation = json.loads(attestation_path.read_text())

    if policy.get('artifactKind') != 'claw.release-trust-policy':
        return fail('Release trust policy artifactKind must be claw.release-trust-policy')
    if policy.get('schemaVersion') != 1:
        return fail('Release trust policy schemaVersion must be 1')
    if policy.get('compatVersion') != '0.1':
        return fail('Release trust policy compatVersion must be 0.1')

    policy_type = policy.get('policyType')
    if policy_type not in {'local-pinned-public-key', 'x509-rooted-public-key'}:
        return fail('Release trust policy policyType must be local-pinned-public-key or x509-rooted-public-key')

    materials = policy.get('materials')
    if not isinstance(materials, dict):
        return fail('Release trust policy materials must be an object')

    expected_paths = {
        'manifestPath': manifest_path,
        'attestationPath': attestation_path,
        'provenancePath': provenance_path,
        'signaturePath': signature_path,
        'publicKeyPath': public_key_path,
    }
    for key, path in expected_paths.items():
        value = materials.get(key)
        if value != path.relative_to(RUST_ROOT).as_posix():
            return fail(f'Release trust policy {key} must match the provided path')

    if materials.get('manifestSha256') != sha256_bytes(manifest_path.read_bytes()):
        return fail('Release trust policy manifestSha256 does not match the provided manifest')
    if materials.get('attestationSha256') != sha256_bytes(attestation_path.read_bytes()):
        return fail('Release trust policy attestationSha256 does not match the provided attestation')
    if materials.get('provenanceSha256') != sha256_bytes(provenance_path.read_bytes()):
        return fail('Release trust policy provenanceSha256 does not match the provided signed provenance')
    if materials.get('signatureSha256') != sha256_bytes(signature_path.read_bytes()):
        return fail('Release trust policy signatureSha256 does not match the provided signature')
    if materials.get('publicKeySha256') != sha256_bytes(public_key_path.read_bytes()):
        return fail('Release trust policy publicKeySha256 does not match the provided public key')

    identity = policy.get('identity')
    if not isinstance(identity, dict):
        return fail('Release trust policy identity must be an object')
    signer = provenance.get('predicate', {}).get('signer', {})
    if identity.get('signerIdentity') != signer.get('identity'):
        return fail('Release trust policy signerIdentity must match signed provenance signer.identity')
    if identity.get('publicKeySha256') != signer.get('publicKeySha256'):
        return fail('Release trust policy publicKeySha256 must match signed provenance signer.publicKeySha256')

    source = policy.get('source')
    if not isinstance(source, dict):
        return fail('Release trust policy source must be an object')
    manifest_git = manifest.get('git', {})
    if source.get('commit') != manifest_git.get('commit'):
        return fail('Release trust policy source.commit must match manifest git.commit')
    if source.get('branch') != manifest_git.get('branch'):
        return fail('Release trust policy source.branch must match manifest git.branch')
    if source.get('dirty') != manifest_git.get('dirty'):
        return fail('Release trust policy source.dirty must match manifest git.dirty')
    if source.get('remotes') != manifest_git.get('remotes'):
        return fail('Release trust policy source.remotes must match manifest git.remotes')

    verification = policy.get('verification')
    if not isinstance(verification, dict):
        return fail('Release trust policy verification must be an object')
    commands = verification.get('commands')
    required_command = (
        'python3 ../tests/validate_release_trust_policy.py '
        '<policy-path> <provenance-path> <signature-path> <public-key-path> <manifest-path> <attestation-path>'
    )
    if not isinstance(commands, list) or required_command not in commands:
        return fail('Release trust policy verification.commands must include the trust policy validator command')
    notes = verification.get('notes')
    if not isinstance(notes, list) or len(notes) < 3:
        return fail('Release trust policy verification.notes must include at least three trust-boundary reminders')

    policy_attestation = policy.get('attestationBinding')
    if not isinstance(policy_attestation, dict):
        return fail('Release trust policy attestationBinding must be an object')
    if policy_attestation.get('manifestArtifactKind') != manifest.get('artifactKind'):
        return fail('Release trust policy attestationBinding.manifestArtifactKind must match the manifest')
    if policy_attestation.get('attestationArtifactKind') != attestation.get('artifactKind'):
        return fail('Release trust policy attestationBinding.attestationArtifactKind must match the attestation')
    if policy_attestation.get('provenancePredicateType') != provenance.get('predicateType'):
        return fail('Release trust policy attestationBinding.provenancePredicateType must match signed provenance predicateType')

    x509_policy = policy.get('x509Identity')
    signer_x509 = signer.get('x509Identity')
    if policy_type == 'x509-rooted-public-key':
        if not isinstance(x509_policy, dict):
            return fail('Release trust policy x509Identity must be present for x509-rooted-public-key policy')
        if not isinstance(signer_x509, dict):
            return fail('Signed provenance signer.x509Identity must be present for x509-rooted-public-key policy')
        certificate_path = RUST_ROOT / x509_policy.get('certificatePath', '')
        trust_root_path = RUST_ROOT / x509_policy.get('trustRootPath', '')
        chain_value = x509_policy.get('chainPath')
        chain_path = RUST_ROOT / chain_value if isinstance(chain_value, str) else None
        for path, label in [(certificate_path, 'X.509 certificate'), (trust_root_path, 'X.509 trust root')]:
            if not path.exists():
                return fail(f'{label} does not exist: {path}')
        if materials.get('certificatePath') != certificate_path.relative_to(RUST_ROOT).as_posix():
            return fail('Release trust policy materials.certificatePath must match x509Identity.certificatePath')
        if materials.get('certificateSha256') != sha256_bytes(certificate_path.read_bytes()):
            return fail('Release trust policy materials.certificateSha256 does not match the provided certificate')
        if materials.get('trustRootPath') != trust_root_path.relative_to(RUST_ROOT).as_posix():
            return fail('Release trust policy materials.trustRootPath must match x509Identity.trustRootPath')
        if materials.get('trustRootSha256') != sha256_bytes(trust_root_path.read_bytes()):
            return fail('Release trust policy materials.trustRootSha256 does not match the provided trust root')
        if x509_policy.get('certificateSha256') != sha256_bytes(certificate_path.read_bytes()):
            return fail('Release trust policy x509Identity.certificateSha256 does not match the provided certificate')
        if x509_policy.get('trustRootSha256') != sha256_bytes(trust_root_path.read_bytes()):
            return fail('Release trust policy x509Identity.trustRootSha256 does not match the provided trust root')
        if x509_policy.get('certificateSubject') != signer_x509.get('certificateSubject'):
            return fail('Release trust policy x509Identity.certificateSubject must match signed provenance signer.x509Identity.certificateSubject')
        if x509_policy.get('certificateIssuer') != signer_x509.get('certificateIssuer'):
            return fail('Release trust policy x509Identity.certificateIssuer must match signed provenance signer.x509Identity.certificateIssuer')
        if chain_path is not None:
            if not chain_path.exists():
                return fail(f'X.509 chain bundle does not exist: {chain_path}')
            if materials.get('chainPath') != chain_path.relative_to(RUST_ROOT).as_posix():
                return fail('Release trust policy materials.chainPath must match x509Identity.chainPath')
            if materials.get('chainSha256') != sha256_bytes(chain_path.read_bytes()):
                return fail('Release trust policy materials.chainSha256 does not match the provided chain bundle')
            if x509_policy.get('chainSha256') != sha256_bytes(chain_path.read_bytes()):
                return fail('Release trust policy x509Identity.chainSha256 does not match the provided chain bundle')
        elif 'chainPath' in materials or 'chainSha256' in materials:
            return fail('Release trust policy chain materials must only be present when x509Identity.chainPath is set')
        try:
            verify_public_key_matches_certificate(public_key_path, certificate_path)
            verify_certificate_chain(certificate_path, trust_root_path, chain_path)
        except FileNotFoundError:
            return fail('openssl is required to verify the release trust policy X.509 chain')
        except (subprocess.CalledProcessError, ValueError) as exc:
            detail = exc.stderr.strip() if isinstance(exc, subprocess.CalledProcessError) and exc.stderr else str(exc)
            return fail(f'Release trust policy X.509 verification failed: {detail}')
    elif x509_policy is not None or signer_x509 is not None:
        return fail('Release trust policy x509Identity must only be present for x509-rooted-public-key policy')

    print(f'Release trust policy validation passed: {policy_path}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
