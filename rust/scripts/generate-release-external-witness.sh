#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
RUST_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd -- "$RUST_ROOT/.." && pwd)
OUTPUT_DIR=${OUTPUT_DIR:-"$RUST_ROOT/.claw/release-artifacts"}
MANIFEST_PATH=${MANIFEST_PATH:-"$OUTPUT_DIR/release-manifest.json"}
ATTESTATION_PATH=${ATTESTATION_PATH:-"$OUTPUT_DIR/release-attestation.json"}
PROVENANCE_PATH=${PROVENANCE_PATH:-"$OUTPUT_DIR/release-provenance.json"}
SIGNATURE_PATH=${SIGNATURE_PATH:-"$OUTPUT_DIR/release-provenance.sig"}
TRUST_POLICY_PATH=${TRUST_POLICY_PATH:-"$OUTPUT_DIR/release-trust-policy.json"}
OUTPUT_PATH=${OUTPUT_PATH:-"$OUTPUT_DIR/release-external-witness.json"}

mkdir -p "$OUTPUT_DIR"

for path in "$MANIFEST_PATH" "$ATTESTATION_PATH" "$PROVENANCE_PATH" "$SIGNATURE_PATH" "$TRUST_POLICY_PATH"; do
  if [[ ! -f "$path" ]]; then
    echo "ERROR: required file does not exist: $path" >&2
    exit 2
  fi
done

python3 - "$RUST_ROOT" "$REPO_ROOT" "$MANIFEST_PATH" "$ATTESTATION_PATH" "$PROVENANCE_PATH" "$SIGNATURE_PATH" "$TRUST_POLICY_PATH" "$OUTPUT_PATH" <<'PY'
from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import quote

rust_root = Path(sys.argv[1]).resolve()
repo_root = Path(sys.argv[2]).resolve()
manifest_path = Path(sys.argv[3]).resolve()
attestation_path = Path(sys.argv[4]).resolve()
provenance_path = Path(sys.argv[5]).resolve()
signature_path = Path(sys.argv[6]).resolve()
trust_policy_path = Path(sys.argv[7]).resolve()
out_path = Path(sys.argv[8]).resolve()

manifest = json.loads(manifest_path.read_text())
attestation = json.loads(attestation_path.read_text())
provenance = json.loads(provenance_path.read_text())
trust_policy = json.loads(trust_policy_path.read_text())


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def rel(path: Path) -> str:
    return path.relative_to(rust_root).as_posix()


def digest_subject(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {
        'name': rel(path),
        'digest': {'sha256': sha256_bytes(data)},
        'bytes': len(data),
    }


def git(args: list[str]) -> str:
    return subprocess.check_output(['git', *args], cwd=repo_root, text=True).strip()


def git_ok(args: list[str]) -> str | None:
    try:
        return git(args)
    except subprocess.CalledProcessError:
        return None


def normalize_remote(url: str) -> str | None:
    if url.startswith('git@github.com:'):
        owner_repo = url.removeprefix('git@github.com:')
        if owner_repo.endswith('.git'):
            owner_repo = owner_repo[:-4]
        return f'https://github.com/{owner_repo}'
    if url.startswith('https://github.com/'):
        return url[:-4] if url.endswith('.git') else url
    return None

commit = manifest.get('git', {}).get('commit') or git(['rev-parse', 'HEAD'])
tree = git(['rev-parse', 'HEAD^{tree}'])
branch = manifest.get('git', {}).get('branch') or git(['rev-parse', '--abbrev-ref', 'HEAD'])
remotes = manifest.get('git', {}).get('remotes') or []

base_remote_url = os.environ.get('EXTERNAL_REPOSITORY_URL')
if not base_remote_url:
    for remote in remotes:
        if not isinstance(remote, dict) or remote.get('kind') != 'fetch':
            continue
        normalized = normalize_remote(str(remote.get('url', '')))
        if normalized:
            base_remote_url = normalized
            break

release_tag = os.environ.get('EXTERNAL_RELEASE_TAG')
tag_object = None
tag_commit = None
if release_tag:
    tag_object = git_ok(['rev-parse', release_tag])
    tag_commit = git_ok(['rev-list', '-n', '1', release_tag])
    if tag_commit is None:
        raise SystemExit(f'ERROR: EXTERNAL_RELEASE_TAG does not resolve locally: {release_tag}')
    if tag_commit != commit:
        raise SystemExit(
            f'ERROR: EXTERNAL_RELEASE_TAG {release_tag} resolves to {tag_commit}, expected current commit {commit}'
        )

source_anchors: list[dict[str, object]] = [
    {
        'kind': 'git-commit',
        'commit': commit,
        'tree': tree,
        'branch': branch,
        'remotes': remotes,
    }
]
if base_remote_url:
    source_anchors.append(
        {
            'kind': 'repository',
            'repositoryUrl': base_remote_url,
            'commitUrl': f'{base_remote_url}/commit/{commit}',
            'treeUrl': f'{base_remote_url}/tree/{commit}',
        }
    )
if release_tag:
    tag_anchor: dict[str, object] = {
        'kind': 'git-tag',
        'tag': release_tag,
        'tagObject': tag_object,
        'commit': tag_commit,
    }
    if base_remote_url:
        tag_anchor['tagUrl'] = f'{base_remote_url}/releases/tag/{quote(release_tag, safe="")}'
    source_anchors.append(tag_anchor)

publication_env = {
    'releaseNotesUrl': os.environ.get('EXTERNAL_RELEASE_NOTES_URL'),
    'sourceTarballUrl': os.environ.get('EXTERNAL_SOURCE_TARBALL_URL'),
    'sourceTarballSha256': os.environ.get('EXTERNAL_SOURCE_TARBALL_SHA256'),
    'provenanceUrl': os.environ.get('EXTERNAL_PROVENANCE_URL'),
    'signatureUrl': os.environ.get('EXTERNAL_SIGNATURE_URL'),
    'trustPolicyUrl': os.environ.get('EXTERNAL_TRUST_POLICY_URL'),
    'timestampAuthorityUrl': os.environ.get('EXTERNAL_TIMESTAMP_AUTHORITY_URL'),
    'timestampTokenUrl': os.environ.get('EXTERNAL_TIMESTAMP_TOKEN_URL'),
    'transparencyIndexUrl': os.environ.get('EXTERNAL_TRANSPARENCY_INDEX_URL'),
}
publications = {k: v for k, v in publication_env.items() if v}

if base_remote_url and 'provenanceUrl' not in publications:
    publications['provenanceUrl'] = f'{base_remote_url}/raw/{commit}/rust/.claw/release-artifacts/release-provenance.json'
if base_remote_url and 'trustPolicyUrl' not in publications:
    publications['trustPolicyUrl'] = f'{base_remote_url}/raw/{commit}/rust/.claw/release-artifacts/release-trust-policy.json'

if not publications and not any(anchor.get('kind') in {'repository', 'git-tag'} for anchor in source_anchors):
    raise SystemExit(
        'ERROR: no external witness anchors found. Provide a GitHub-like fetch remote or set EXTERNAL_* publication URLs.'
    )

bundle = {
    'artifactKind': 'claw.release-external-witness',
    'schemaVersion': 1,
    'compatVersion': '0.1',
    'generatedAt': datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace('+00:00', 'Z'),
    'subject': [
        digest_subject(manifest_path),
        digest_subject(attestation_path),
        digest_subject(provenance_path),
        digest_subject(signature_path),
        digest_subject(trust_policy_path),
    ],
    'predicate': {
        'source': {
            'commit': commit,
            'tree': tree,
            'branch': branch,
            'releaseTag': release_tag,
        },
        'anchors': source_anchors,
        'publications': publications,
        'materialBinding': {
            'manifestArtifactKind': manifest.get('artifactKind'),
            'attestationArtifactKind': attestation.get('artifactKind'),
            'provenanceArtifactKind': provenance.get('artifactKind'),
            'trustPolicyArtifactKind': trust_policy.get('artifactKind'),
            'provenanceSha256': sha256_bytes(provenance_path.read_bytes()),
            'signatureSha256': sha256_bytes(signature_path.read_bytes()),
            'trustPolicySha256': sha256_bytes(trust_policy_path.read_bytes()),
        },
        'verification': {
            'commands': [
                'python3 ../tests/validate_signed_release_provenance.py <provenance-path> <signature-path> <public-key-path> <trust-policy-path>',
                'python3 ../tests/validate_release_trust_policy.py <policy-path> <provenance-path> <signature-path> <public-key-path> <manifest-path> <attestation-path>',
                'python3 ../tests/validate_release_external_witness.py <witness-path> <manifest-path> <attestation-path> <provenance-path> <signature-path> <trust-policy-path>',
            ],
            'notes': [
                'This witness file records external publication coordinates and repository anchors for the local signed provenance chain.',
                'It improves auditability and independent retrieval, but does not prove that any remote service verified the build or signer identity.',
                'It is not a Sigstore log entry, not a SLSA hosted-builder statement, and not a public package-registry provenance guarantee.',
                'Operators still need an out-of-band trust decision for the repository host, any publication URLs, and any optional timestamp or transparency references listed here.',
            ],
        },
    },
}

out_path.write_text(json.dumps(bundle, indent=2) + '\n')
print(out_path)
PY
