#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
RUST_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
OUTPUT_DIR=${OUTPUT_DIR:-"$RUST_ROOT/.claw/release-artifacts"}
MANIFEST_PATH=${MANIFEST_PATH:-"$OUTPUT_DIR/release-manifest.json"}
ATTESTATION_PATH=${ATTESTATION_PATH:-"$OUTPUT_DIR/release-attestation.json"}
PROVENANCE_PATH=${PROVENANCE_PATH:-"$OUTPUT_DIR/release-provenance.json"}
SIGNATURE_PATH=${SIGNATURE_PATH:-"$OUTPUT_DIR/release-provenance.sig"}
PUBLIC_KEY_PATH=${PUBLIC_KEY_PATH:-"$OUTPUT_DIR/release-provenance.pub.pem"}
TRUST_POLICY_PATH=${TRUST_POLICY_PATH:-"$OUTPUT_DIR/release-trust-policy.json"}
SIGNING_KEY_PATH=${PROVENANCE_SIGNING_KEY:-${SIGNING_KEY_PATH:-}}
SIGNER_IDENTITY=${PROVENANCE_SIGNER_IDENTITY:-${SIGNER_IDENTITY:-local-operator}}
CLAW_BIN=${CLAW_BIN:-"$RUST_ROOT/target/debug/claw"}

mkdir -p "$OUTPUT_DIR"

if [[ -z "$SIGNING_KEY_PATH" ]]; then
  echo "ERROR: set PROVENANCE_SIGNING_KEY (or SIGNING_KEY_PATH) to a PEM private key for release provenance signing." >&2
  exit 2
fi

for path in "$MANIFEST_PATH" "$ATTESTATION_PATH" "$CLAW_BIN" "$SIGNING_KEY_PATH"; do
  if [[ ! -f "$path" ]]; then
    echo "ERROR: required file does not exist: $path" >&2
    exit 2
  fi
done

if ! command -v openssl >/dev/null 2>&1; then
  echo "ERROR: openssl is required for signed provenance generation." >&2
  exit 2
fi

if [[ ! -f "$PUBLIC_KEY_PATH" ]]; then
  openssl pkey -in "$SIGNING_KEY_PATH" -pubout -out "$PUBLIC_KEY_PATH"
fi

python3 - "$RUST_ROOT" "$CLAW_BIN" "$MANIFEST_PATH" "$ATTESTATION_PATH" "$PUBLIC_KEY_PATH" "$PROVENANCE_PATH" "$TRUST_POLICY_PATH" "$SIGNER_IDENTITY" <<'PY'
from __future__ import annotations

import hashlib
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

rust_root = Path(sys.argv[1]).resolve()
claw_bin = Path(sys.argv[2]).resolve()
manifest_path = Path(sys.argv[3]).resolve()
attestation_path = Path(sys.argv[4]).resolve()
pubkey_path = Path(sys.argv[5]).resolve()
out_path = Path(sys.argv[6]).resolve()
trust_policy_path = Path(sys.argv[7]).resolve()
signer_identity = sys.argv[8]

manifest = json.loads(manifest_path.read_text())
attestation = json.loads(attestation_path.read_text())


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def subject(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {
        'name': path.relative_to(rust_root).as_posix(),
        'digest': {'sha256': sha256_bytes(data)},
        'bytes': len(data),
    }


bundle = {
    'artifactKind': 'claw.signed-release-provenance',
    'schemaVersion': 1,
    'compatVersion': '0.1',
    '_type': 'https://in-toto.io/Statement/v1',
    'predicateType': 'https://claw.dev/attestation/local-source-build-signed/v1',
    'generatedAt': datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace('+00:00', 'Z'),
    'subject': [
        subject(claw_bin),
        subject(manifest_path),
        subject(attestation_path),
    ],
    'predicate': {
        'signer': {
            'identity': signer_identity,
            'publicKeyPath': pubkey_path.relative_to(rust_root).as_posix(),
            'publicKeySha256': sha256_bytes(pubkey_path.read_bytes()),
            'signaturePath': out_path.with_suffix('.sig').relative_to(rust_root).as_posix(),
            'signatureAlgorithm': 'openssl.dgstrsa-or-ecdsa-sha256',
        },
        'verification': {
            'materializedFrom': {
                'manifestArtifactKind': manifest.get('artifactKind'),
                'manifestSchemaVersion': manifest.get('schemaVersion'),
                'attestationArtifactKind': attestation.get('artifactKind'),
                'attestationPredicateType': attestation.get('predicateType'),
            },
            'trustPolicyPath': trust_policy_path.relative_to(rust_root).as_posix(),
            'trustModel': 'local-pinned-public-key',
            'commands': [
                'python3 ../tests/validate_release_artifact_manifest.py <manifest-path>',
                'python3 ../tests/validate_release_attestation.py <attestation-path> <manifest-path>',
                'python3 ../tests/validate_signed_release_provenance.py <provenance-path> <signature-path> <public-key-path>',
                'python3 ../tests/validate_release_trust_policy.py <policy-path> <provenance-path> <signature-path> <public-key-path> <manifest-path> <attestation-path>',
            ],
            'notes': [
                'This bundle cryptographically signs the local release provenance statement, but only with an operator-managed key.',
                'It improves tamper evidence for the manifest+attestation chain without claiming transparency-log inclusion, keyless signing, or hosted builder identity.',
                'The attached trust policy pins the signer public key and expected source metadata for this specific local release process; operators still choose whether to trust that key.',
            ],
        },
    },
}

out_path.write_text(json.dumps(bundle, indent=2) + '\n')
PY

openssl dgst -sha256 -sign "$SIGNING_KEY_PATH" -out "$SIGNATURE_PATH" "$PROVENANCE_PATH"

python3 - "$RUST_ROOT" "$MANIFEST_PATH" "$ATTESTATION_PATH" "$PROVENANCE_PATH" "$SIGNATURE_PATH" "$PUBLIC_KEY_PATH" "$TRUST_POLICY_PATH" "$SIGNER_IDENTITY" <<'PY'
from __future__ import annotations

import hashlib
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

rust_root = Path(sys.argv[1]).resolve()
manifest_path = Path(sys.argv[2]).resolve()
attestation_path = Path(sys.argv[3]).resolve()
provenance_path = Path(sys.argv[4]).resolve()
signature_path = Path(sys.argv[5]).resolve()
pubkey_path = Path(sys.argv[6]).resolve()
trust_policy_path = Path(sys.argv[7]).resolve()
signer_identity = sys.argv[8]

manifest = json.loads(manifest_path.read_text())
attestation = json.loads(attestation_path.read_text())
provenance = json.loads(provenance_path.read_text())


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


policy = {
    'artifactKind': 'claw.release-trust-policy',
    'schemaVersion': 1,
    'compatVersion': '0.1',
    'policyType': 'local-pinned-public-key',
    'generatedAt': datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace('+00:00', 'Z'),
    'identity': {
        'signerIdentity': signer_identity,
        'publicKeyPath': pubkey_path.relative_to(rust_root).as_posix(),
        'publicKeySha256': sha256_bytes(pubkey_path.read_bytes()),
    },
    'source': {
        'commit': manifest.get('git', {}).get('commit'),
        'branch': manifest.get('git', {}).get('branch'),
        'dirty': manifest.get('git', {}).get('dirty'),
        'remotes': manifest.get('git', {}).get('remotes'),
    },
    'materials': {
        'manifestPath': manifest_path.relative_to(rust_root).as_posix(),
        'manifestSha256': sha256_bytes(manifest_path.read_bytes()),
        'attestationPath': attestation_path.relative_to(rust_root).as_posix(),
        'attestationSha256': sha256_bytes(attestation_path.read_bytes()),
        'provenancePath': provenance_path.relative_to(rust_root).as_posix(),
        'provenanceSha256': sha256_bytes(provenance_path.read_bytes()),
        'signaturePath': signature_path.relative_to(rust_root).as_posix(),
        'signatureSha256': sha256_bytes(signature_path.read_bytes()),
        'publicKeyPath': pubkey_path.relative_to(rust_root).as_posix(),
        'publicKeySha256': sha256_bytes(pubkey_path.read_bytes()),
    },
    'attestationBinding': {
        'manifestArtifactKind': manifest.get('artifactKind'),
        'manifestSchemaVersion': manifest.get('schemaVersion'),
        'attestationArtifactKind': attestation.get('artifactKind'),
        'attestationPredicateType': attestation.get('predicateType'),
        'provenancePredicateType': provenance.get('predicateType'),
    },
    'verification': {
        'commands': [
            'python3 ../tests/validate_release_artifact_manifest.py <manifest-path>',
            'python3 ../tests/validate_release_attestation.py <attestation-path> <manifest-path>',
            'python3 ../tests/validate_signed_release_provenance.py <provenance-path> <signature-path> <public-key-path>',
            'python3 ../tests/validate_release_trust_policy.py <policy-path> <provenance-path> <signature-path> <public-key-path> <manifest-path> <attestation-path>',
        ],
        'notes': [
            'This trust policy improves repeatability by pinning the exact public key fingerprint, manifest, attestation, and signed provenance bundle for one local release verification flow.',
            'It does not establish a hosted root of trust, external transparency log, identity federation, or public package-registry provenance.',
            'Operators still need an out-of-band decision for whether the pinned signer identity and key fingerprint are acceptable.',
        ],
    },
}

trust_policy_path.write_text(json.dumps(policy, indent=2) + '\n')
PY

echo "$PROVENANCE_PATH"
