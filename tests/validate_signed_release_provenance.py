from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = REPO_ROOT / 'rust'
DEFAULT_PROVENANCE = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-provenance.json'
DEFAULT_SIGNATURE = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-provenance.sig'
DEFAULT_PUBLIC_KEY = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-provenance.pub.pem'
DEFAULT_TRUST_POLICY = RUST_ROOT / '.claw' / 'release-artifacts' / 'release-trust-policy.json'


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
        raise ValueError('signing certificate public key does not match the provided public key')


def main() -> int:
    provenance_path = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT_PROVENANCE
    signature_path = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else DEFAULT_SIGNATURE
    public_key_path = Path(sys.argv[3]).resolve() if len(sys.argv) > 3 else DEFAULT_PUBLIC_KEY
    trust_policy_path = Path(sys.argv[4]).resolve() if len(sys.argv) > 4 else DEFAULT_TRUST_POLICY

    for path, label in [
        (provenance_path, 'Signed provenance'),
        (signature_path, 'Signed provenance signature'),
        (public_key_path, 'Signed provenance public key'),
        (trust_policy_path, 'Release trust policy'),
    ]:
        if not path.exists():
            return fail(f'{label} does not exist: {path}')

    try:
        verify_signature(provenance_path, signature_path, public_key_path)
    except FileNotFoundError:
        return fail('openssl is required to verify signed release provenance')
    except subprocess.CalledProcessError as exc:
        return fail(f'Signed provenance signature verification failed: {exc.stderr.strip() or exc.stdout.strip()}')

    provenance = json.loads(provenance_path.read_text())

    if provenance.get('artifactKind') != 'claw.signed-release-provenance':
        return fail('Signed provenance artifactKind must be claw.signed-release-provenance')
    if provenance.get('schemaVersion') != 1:
        return fail('Signed provenance schemaVersion must be 1')
    if provenance.get('compatVersion') != '0.1':
        return fail('Signed provenance compatVersion must be 0.1')
    if provenance.get('_type') != 'https://in-toto.io/Statement/v1':
        return fail('Signed provenance _type must be https://in-toto.io/Statement/v1')
    if provenance.get('predicateType') != 'https://claw.dev/attestation/local-source-build-signed/v1':
        return fail('Signed provenance predicateType must be https://claw.dev/attestation/local-source-build-signed/v1')

    subject = provenance.get('subject')
    if not isinstance(subject, list) or len(subject) < 3:
        return fail('Signed provenance subject must include binary, manifest, and attestation entries')

    expected_subjects = {
        'target/debug/claw': RUST_ROOT / 'target' / 'debug' / 'claw',
        '.claw/release-artifacts/release-manifest.json': RUST_ROOT / '.claw' / 'release-artifacts' / 'release-manifest.json',
        '.claw/release-artifacts/release-attestation.json': RUST_ROOT / '.claw' / 'release-artifacts' / 'release-attestation.json',
    }
    by_name = {item.get('name'): item for item in subject if isinstance(item, dict)}
    for name, path in expected_subjects.items():
        item = by_name.get(name)
        if item is None:
            return fail(f'Signed provenance subject missing {name}')
        digest = item.get('digest', {}).get('sha256')
        actual_sha = sha256_bytes(path.read_bytes())
        if digest != actual_sha:
            return fail(f'Signed provenance digest mismatch for {name}')
        if item.get('bytes') != path.stat().st_size:
            return fail(f'Signed provenance byte count mismatch for {name}')

    predicate = provenance.get('predicate')
    if not isinstance(predicate, dict):
        return fail('Signed provenance predicate must be an object')
    signer = predicate.get('signer')
    verification = predicate.get('verification')
    if not isinstance(signer, dict):
        return fail('Signed provenance signer must be an object')
    if not isinstance(verification, dict):
        return fail('Signed provenance verification must be an object')

    if signer.get('publicKeyPath') != public_key_path.relative_to(RUST_ROOT).as_posix():
        return fail('Signed provenance signer.publicKeyPath must match the provided public key path')
    if signer.get('publicKeySha256') != sha256_bytes(public_key_path.read_bytes()):
        return fail('Signed provenance signer.publicKeySha256 does not match the provided public key')
    if signer.get('signaturePath') != signature_path.relative_to(RUST_ROOT).as_posix():
        return fail('Signed provenance signer.signaturePath must match the signature path')

    trust_model = verification.get('trustModel')
    if trust_model not in {'local-pinned-public-key', 'x509-rooted-public-key'}:
        return fail('Signed provenance verification.trustModel must be local-pinned-public-key or x509-rooted-public-key')
    if verification.get('trustPolicyPath') != trust_policy_path.relative_to(RUST_ROOT).as_posix():
        return fail('Signed provenance verification.trustPolicyPath must match the provided trust policy path')

    x509_identity = signer.get('x509Identity')
    if trust_model == 'x509-rooted-public-key':
        if not isinstance(x509_identity, dict):
            return fail('Signed provenance signer.x509Identity must be present for x509-rooted-public-key trust')
        certificate_path = RUST_ROOT / x509_identity.get('certificatePath', '')
        trust_root_path = RUST_ROOT / x509_identity.get('trustRootPath', '')
        chain_path_value = x509_identity.get('chainPath')
        if not certificate_path.exists():
            return fail(f'Signed provenance X.509 certificate does not exist: {certificate_path}')
        if not trust_root_path.exists():
            return fail(f'Signed provenance X.509 trust root does not exist: {trust_root_path}')
        if x509_identity.get('certificateSha256') != sha256_bytes(certificate_path.read_bytes()):
            return fail('Signed provenance signer.x509Identity.certificateSha256 does not match the provided certificate')
        if x509_identity.get('trustRootSha256') != sha256_bytes(trust_root_path.read_bytes()):
            return fail('Signed provenance signer.x509Identity.trustRootSha256 does not match the provided trust root')
        if x509_identity.get('verificationMode') != 'openssl-verify-ca':
            return fail('Signed provenance signer.x509Identity.verificationMode must be openssl-verify-ca')
        try:
            verify_public_key_matches_certificate(public_key_path, certificate_path)
        except (subprocess.CalledProcessError, ValueError) as exc:
            return fail(f'Signed provenance X.509 leaf/public key verification failed: {exc}')
        if chain_path_value is not None:
            chain_path = RUST_ROOT / chain_path_value
            if not chain_path.exists():
                return fail(f'Signed provenance X.509 chain does not exist: {chain_path}')
            if x509_identity.get('chainSha256') != sha256_bytes(chain_path.read_bytes()):
                return fail('Signed provenance signer.x509Identity.chainSha256 does not match the provided chain bundle')
    elif x509_identity is not None:
        return fail('Signed provenance signer.x509Identity must only be present for x509-rooted-public-key trust')

    commands = verification.get('commands')
    required_command = 'python3 ../tests/validate_signed_release_provenance.py <provenance-path> <signature-path> <public-key-path>'
    required_policy_command = (
        'python3 ../tests/validate_release_trust_policy.py '
        '<policy-path> <provenance-path> <signature-path> <public-key-path> <manifest-path> <attestation-path>'
    )
    if not isinstance(commands, list) or required_command not in commands:
        return fail('Signed provenance verification.commands must include the signed provenance validator command')
    if required_policy_command not in commands:
        return fail('Signed provenance verification.commands must include the release trust policy validator command')
    notes = verification.get('notes')
    if not isinstance(notes, list) or len(notes) < 3:
        return fail('Signed provenance verification.notes must include at least three trust-boundary reminders')

    print(f'Signed release provenance validation passed: {provenance_path}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
