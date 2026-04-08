from __future__ import annotations

from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = REPO_ROOT / 'rust'

REQUIRED_FILES = [
    RUST_ROOT / 'README.md',
    RUST_ROOT / 'RELEASE.md',
    RUST_ROOT / 'BOOTSTRAP.md',
    RUST_ROOT / 'CHANGELOG.md',
    RUST_ROOT / 'docs' / 'ARTIFACTS.md',
    RUST_ROOT / 'docs' / 'PRIVACY.md',
    RUST_ROOT / 'docs' / 'RELEASE_CANDIDATE.md',
    RUST_ROOT / 'scripts' / 'release-verify.sh',
]


def require_contains(text: str, needle: str, failure: str) -> None:
    if needle not in text:
        raise SystemExit(failure)


def main() -> int:
    missing = [path for path in REQUIRED_FILES if not path.exists()]
    if missing:
        print('Missing RC-readiness files:')
        for path in missing:
            print(f'- {path.relative_to(REPO_ROOT)}')
        return 1

    readme_text = (RUST_ROOT / 'README.md').read_text()
    release_text = (RUST_ROOT / 'RELEASE.md').read_text()
    bootstrap_text = (RUST_ROOT / 'BOOTSTRAP.md').read_text()
    changelog_text = (RUST_ROOT / 'CHANGELOG.md').read_text()
    artifacts_text = (RUST_ROOT / 'docs' / 'ARTIFACTS.md').read_text()
    privacy_text = (RUST_ROOT / 'docs' / 'PRIVACY.md').read_text()
    rc_doc_text = (RUST_ROOT / 'docs' / 'RELEASE_CANDIDATE.md').read_text()
    verify_text = (RUST_ROOT / 'scripts' / 'release-verify.sh').read_text()

    require_contains(
        readme_text,
        'docs/RELEASE_CANDIDATE.md',
        'rust/README.md does not link to the RC discipline guide.',
    )
    require_contains(
        release_text,
        'python3 ../tests/validate_release_candidate_readiness.py',
        'rust/RELEASE.md does not mention the RC readiness validator.',
    )
    require_contains(
        bootstrap_text,
        'RELEASE_CANDIDATE=1 ./scripts/release-verify.sh',
        'rust/BOOTSTRAP.md does not mention the RC verification entrypoint.',
    )
    require_contains(
        changelog_text,
        '### Operator notes',
        'rust/CHANGELOG.md is missing the operator-notes release scaffold.',
    )
    require_contains(
        changelog_text,
        'Compatibility or migration notes:',
        'rust/CHANGELOG.md is missing the compatibility/migration release scaffold.',
    )
    require_contains(
        verify_text,
        'python3 ../tests/validate_release_candidate_readiness.py',
        'release-verify.sh does not run the RC readiness validator.',
    )

    for text, label in [
        (readme_text, 'rust/README.md'),
        (release_text, 'rust/RELEASE.md'),
        (rc_doc_text, 'rust/docs/RELEASE_CANDIDATE.md'),
    ]:
        require_contains(
            text,
            'artifactKind',
            f'{label} does not mention artifactKind in RC/release guidance.',
        )
        require_contains(
            text,
            'compatVersion',
            f'{label} does not mention compatVersion in RC/release guidance.',
        )

    require_contains(
        artifacts_text,
        'Strict third-party parsers',
        'rust/docs/ARTIFACTS.md is missing the defensive-parsing trust guidance.',
    )
    require_contains(
        privacy_text,
        'artifactKind',
        'rust/docs/PRIVACY.md is missing the envelope-sharing guidance.',
    )

    for needle in [
        'clean tree',
        'fresh `.claw/` state',
        'Compatibility or migration notes',
        'no schema change from prior RC',
    ]:
        require_contains(
            rc_doc_text,
            needle,
            f'rust/docs/RELEASE_CANDIDATE.md is missing required RC cue: {needle}',
        )

    print('Release-candidate readiness validation passed.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
