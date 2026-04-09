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
CERT_OUT_PATH=${CERT_OUT_PATH:-"$OUTPUT_DIR/release-provenance.cert.pem"}
CHAIN_OUT_PATH=${CHAIN_OUT_PATH:-"$OUTPUT_DIR/release-provenance.chain.pem"}
TRUST_ROOT_OUT_PATH=${TRUST_ROOT_OUT_PATH:-"$OUTPUT_DIR/release-provenance.root.pem"}
SIGNING_KEY_PATH=${PROVENANCE_SIGNING_KEY:-${SIGNING_KEY_PATH:-}}
SIGNER_IDENTITY=${PROVENANCE_SIGNER_IDENTITY:-${SIGNER_IDENTITY:-local-operator}}
SIGNING_CERT_PATH=${PROVENANCE_SIGNING_CERT:-${SIGNING_CERT_PATH:-}}
SIGNING_CHAIN_PATH=${PROVENANCE_SIGNING_CHAIN:-${SIGNING_CHAIN_PATH:-}}
TRUST_ROOT_PATH=${PROVENANCE_TRUST_ROOT:-${TRUST_ROOT_PATH:-}}
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

for cmd in openssl python3; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "ERROR: $cmd is required for signed provenance generation." >&2
    exit 2
  fi
done

x509_mode=0
if [[ -n "$SIGNING_CERT_PATH" || -n "$SIGNING_CHAIN_PATH" || -n "$TRUST_ROOT_PATH" ]]; then
  x509_mode=1
fi

if [[ $x509_mode -eq 1 ]]; then
  if [[ -z "$SIGNING_CERT_PATH" || -z "$TRUST_ROOT_PATH" ]]; then
    echo "ERROR: rooted X.509 provenance requires both PROVENANCE_SIGNING_CERT and PROVENANCE_TRUST_ROOT (chain optional)." >&2
    exit 2
  fi
  for path in "$SIGNING_CERT_PATH" "$TRUST_ROOT_PATH"; do
    if [[ ! -f "$path" ]]; then
      echo "ERROR: required X.509 material does not exist: $path" >&2
      exit 2
    fi
  done
  if [[ -n "$SIGNING_CHAIN_PATH" && ! -f "$SIGNING_CHAIN_PATH" ]]; then
    echo "ERROR: required X.509 chain file does not exist: $SIGNING_CHAIN_PATH" >&2
    exit 2
  fi
fi

openssl pkey -in "$SIGNING_KEY_PATH" -pubout -out "$PUBLIC_KEY_PATH"

if [[ $x509_mode -eq 1 ]]; then
  cp "$SIGNING_CERT_PATH" "$CERT_OUT_PATH"
  if [[ -n "$SIGNING_CHAIN_PATH" ]]; then
    cp "$SIGNING_CHAIN_PATH" "$CHAIN_OUT_PATH"
  else
    rm -f "$CHAIN_OUT_PATH"
  fi
  cp "$TRUST_ROOT_PATH" "$TRUST_ROOT_OUT_PATH"

  verify_args=(verify -CAfile "$TRUST_ROOT_OUT_PATH")
  if [[ -f "$CHAIN_OUT_PATH" ]]; then
    verify_args+=( -untrusted "$CHAIN_OUT_PATH" )
  fi
  verify_args+=( "$CERT_OUT_PATH" )
  openssl "${verify_args[@]}" >/dev/null

  cert_pub_der=$(mktemp)
  key_pub_der=$(mktemp)
  trap 'rm -f "$cert_pub_der" "$key_pub_der"' EXIT
  openssl x509 -in "$CERT_OUT_PATH" -pubkey -noout | openssl pkey -pubin -outform DER > "$cert_pub_der"
  openssl pkey -pubin -in "$PUBLIC_KEY_PATH" -outform DER > "$key_pub_der"
  cmp -s "$cert_pub_der" "$key_pub_der" || {
    echo "ERROR: signing certificate public key does not match the derived provenance public key." >&2
    exit 2
  }
fi

python3 - "$RUST_ROOT" "$CLAW_BIN" "$MANIFEST_PATH" "$ATTESTATION_PATH" "$PUBLIC_KEY_PATH" "$PROVENANCE_PATH" "$TRUST_POLICY_PATH" "$SIGNER_IDENTITY" "$CERT_OUT_PATH" "$CHAIN_OUT_PATH" "$TRUST_ROOT_OUT_PATH" "$x509_mode" <<'PY'
from __future__ import annotations

import hashlib
import json
import subprocess
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
cert_path = Path(sys.argv[9]).resolve()
chain_path = Path(sys.argv[10]).resolve()
trust_root_path = Path(sys.argv[11]).resolve()
x509_mode = sys.argv[12] == '1'

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


def openssl_text(*args: str) -> str:
    return subprocess.check_output(['openssl', *args], text=True).strip()


signer: dict[str, object] = {
    'identity': signer_identity,
    'publicKeyPath': pubkey_path.relative_to(rust_root).as_posix(),
    'publicKeySha256': sha256_bytes(pubkey_path.read_bytes()),
    'signaturePath': out_path.with_suffix('.sig').relative_to(rust_root).as_posix(),
    'signatureAlgorithm': 'openssl.dgstrsa-or-ecdsa-sha256',
}
trust_model = 'local-pinned-public-key'

if x509_mode:
    x509_identity: dict[str, object] = {
        'certificatePath': cert_path.relative_to(rust_root).as_posix(),
        'certificateSha256': sha256_bytes(cert_path.read_bytes()),
        'certificateSubject': openssl_text('x509', '-in', str(cert_path), '-noout', '-subject').removeprefix('subject=').strip(),
        'certificateIssuer': openssl_text('x509', '-in', str(cert_path), '-noout', '-issuer').removeprefix('issuer=').strip(),
        'trustRootPath': trust_root_path.relative_to(rust_root).as_posix(),
        'trustRootSha256': sha256_bytes(trust_root_path.read_bytes()),
        'verificationMode': 'openssl-verify-ca',
    }
    if chain_path.exists():
        x509_identity['chainPath'] = chain_path.relative_to(rust_root).as_posix()
        x509_identity['chainSha256'] = sha256_bytes(chain_path.read_bytes())
    signer['x509Identity'] = x509_identity
    trust_model = 'x509-rooted-public-key'

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
        'signer': signer,
        'verification': {
            'materializedFrom': {
                'manifestArtifactKind': manifest.get('artifactKind'),
                'manifestSchemaVersion': manifest.get('schemaVersion'),
                'attestationArtifactKind': attestation.get('artifactKind'),
                'attestationPredicateType': attestation.get('predicateType'),
            },
            'trustPolicyPath': trust_policy_path.relative_to(rust_root).as_posix(),
            'trustModel': trust_model,
            'commands': [
                'python3 ../tests/validate_release_artifact_manifest.py <manifest-path>',
                'python3 ../tests/validate_release_attestation.py <attestation-path> <manifest-path>',
                'python3 ../tests/validate_signed_release_provenance.py <provenance-path> <signature-path> <public-key-path>',
                'python3 ../tests/validate_release_trust_policy.py <policy-path> <provenance-path> <signature-path> <public-key-path> <manifest-path> <attestation-path>',
            ],
            'notes': [
                'This bundle cryptographically signs the local release provenance statement, but only with operator-managed signing material.',
                'It improves tamper evidence for the manifest+attestation chain without claiming transparency-log inclusion, keyless signing, or hosted builder identity.',
                'If X.509 material is attached, the root of trust is only the provided CA/root bundle and must still be accepted out-of-band by the operator.',
            ],
        },
    },
}

out_path.write_text(json.dumps(bundle, indent=2) + '\n')
PY

openssl dgst -sha256 -sign "$SIGNING_KEY_PATH" -out "$SIGNATURE_PATH" "$PROVENANCE_PATH"

python3 - "$RUST_ROOT" "$MANIFEST_PATH" "$ATTESTATION_PATH" "$PROVENANCE_PATH" "$SIGNATURE_PATH" "$PUBLIC_KEY_PATH" "$TRUST_POLICY_PATH" "$SIGNER_IDENTITY" "$CERT_OUT_PATH" "$CHAIN_OUT_PATH" "$TRUST_ROOT_OUT_PATH" "$x509_mode" <<'PY'
from __future__ import annotations

import hashlib
import json
import subprocess
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
cert_path = Path(sys.argv[9]).resolve()
chain_path = Path(sys.argv[10]).resolve()
trust_root_path = Path(sys.argv[11]).resolve()
x509_mode = sys.argv[12] == '1'

manifest = json.loads(manifest_path.read_text())
attestation = json.loads(attestation_path.read_text())
provenance = json.loads(provenance_path.read_text())


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def openssl_text(*args: str) -> str:
    return subprocess.check_output(['openssl', *args], text=True).strip()


policy = {
    'artifactKind': 'claw.release-trust-policy',
    'schemaVersion': 1,
    'compatVersion': '0.1',
    'policyType': 'x509-rooted-public-key' if x509_mode else 'local-pinned-public-key',
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
            'This trust policy improves repeatability by pinning the exact signing materials and release artifacts for one local verification flow.',
            'It does not establish a hosted builder identity, transparency log, identity federation, or public package-registry provenance.',
            'Operators still need an out-of-band decision for whether the pinned signer identity and any attached root/certificate bundle are acceptable.',
        ],
    },
}

if x509_mode:
    x509_identity: dict[str, object] = {
        'certificatePath': cert_path.relative_to(rust_root).as_posix(),
        'certificateSha256': sha256_bytes(cert_path.read_bytes()),
        'certificateSubject': openssl_text('x509', '-in', str(cert_path), '-noout', '-subject').removeprefix('subject=').strip(),
        'certificateIssuer': openssl_text('x509', '-in', str(cert_path), '-noout', '-issuer').removeprefix('issuer=').strip(),
        'trustRootPath': trust_root_path.relative_to(rust_root).as_posix(),
        'trustRootSha256': sha256_bytes(trust_root_path.read_bytes()),
    }
    policy['materials']['certificatePath'] = cert_path.relative_to(rust_root).as_posix()
    policy['materials']['certificateSha256'] = sha256_bytes(cert_path.read_bytes())
    policy['materials']['trustRootPath'] = trust_root_path.relative_to(rust_root).as_posix()
    policy['materials']['trustRootSha256'] = sha256_bytes(trust_root_path.read_bytes())
    if chain_path.exists():
        x509_identity['chainPath'] = chain_path.relative_to(rust_root).as_posix()
        x509_identity['chainSha256'] = sha256_bytes(chain_path.read_bytes())
        policy['materials']['chainPath'] = chain_path.relative_to(rust_root).as_posix()
        policy['materials']['chainSha256'] = sha256_bytes(chain_path.read_bytes())
    policy['x509Identity'] = x509_identity

trust_policy_path.write_text(json.dumps(policy, indent=2) + '\n')
PY

echo "$PROVENANCE_PATH"
