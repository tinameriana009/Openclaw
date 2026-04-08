from __future__ import annotations

from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = REPO_ROOT / 'rust'

REQUIRED_FILES = [
    RUST_ROOT / 'README.md',
    RUST_ROOT / 'RELEASE.md',
    RUST_ROOT / 'CHANGELOG.md',
    RUST_ROOT / 'docs' / 'ARTIFACTS.md',
    RUST_ROOT / 'docs' / 'PRIVACY.md',
    RUST_ROOT / 'scripts' / 'release-verify.sh',
    REPO_ROOT / 'docs' / 'workflows' / 'README.md',
    REPO_ROOT / 'docs' / 'workflows' / 'blender-addon.md',
    REPO_ROOT / 'docs' / 'workflows' / 'unreal-plugin.md',
    REPO_ROOT / 'docs' / 'workflows' / 'repo-analysis.md',
    REPO_ROOT / 'tests' / 'validate_blender_demo.py',
    REPO_ROOT / 'tests' / 'validate_unreal_demo.py',
    REPO_ROOT / 'tests' / 'validate_repo_analysis_demo.py',
    RUST_ROOT / 'scripts' / 'run-repo-analysis-demo.sh',
    RUST_ROOT / 'scripts' / 'prepare-blender-demo.sh',
    RUST_ROOT / 'scripts' / 'prepare-unreal-demo.sh',
]


def require_contains(text: str, needle: str, failure: str) -> None:
    if needle not in text:
        raise SystemExit(failure)


def main() -> int:
    missing = [path for path in REQUIRED_FILES if not path.exists()]
    if missing:
        print('Missing operator-readiness files:')
        for path in missing:
            print(f'- {path.relative_to(REPO_ROOT)}')
        return 1

    readme_text = (RUST_ROOT / 'README.md').read_text()
    release_text = (RUST_ROOT / 'RELEASE.md').read_text()
    artifacts_text = (RUST_ROOT / 'docs' / 'ARTIFACTS.md').read_text()
    workflow_index_text = (REPO_ROOT / 'docs' / 'workflows' / 'README.md').read_text()
    workflow_index_lower = workflow_index_text.lower()
    blender_workflow_text = (REPO_ROOT / 'docs' / 'workflows' / 'blender-addon.md').read_text()
    unreal_workflow_text = (REPO_ROOT / 'docs' / 'workflows' / 'unreal-plugin.md').read_text()
    repo_workflow_text = (REPO_ROOT / 'docs' / 'workflows' / 'repo-analysis.md').read_text()
    release_verify_text = (RUST_ROOT / 'scripts' / 'release-verify.sh').read_text()

    for needle in ['artifactKind', 'schemaVersion', 'compatVersion']:
        require_contains(artifacts_text, needle, f'docs/ARTIFACTS.md is missing required artifact contract marker: {needle}')

    require_contains(
        release_text,
        'RELEASE_CANDIDATE=1 ./scripts/release-verify.sh',
        'RELEASE.md does not document the RC verification entrypoint.',
    )
    require_contains(
        release_text,
        'python3 ../tests/validate_operator_readiness.py',
        'RELEASE.md does not mention the operator-readiness validator.',
    )
    require_contains(
        readme_text,
        'python3 ../tests/validate_operator_readiness.py',
        'rust/README.md does not mention the operator-readiness validator.',
    )
    require_contains(
        readme_text,
        './scripts/run-repo-analysis-demo.sh',
        'rust/README.md does not mention the repo-analysis demo runner.',
    )
    require_contains(
        readme_text,
        './scripts/prepare-blender-demo.sh',
        'rust/README.md does not mention the Blender demo prep helper.',
    )
    require_contains(
        readme_text,
        './scripts/prepare-unreal-demo.sh',
        'rust/README.md does not mention the Unreal demo prep helper.',
    )

    for needle in [
        'python3 ../tests/validate_operator_readiness.py',
        'python3 ../tests/validate_blender_demo.py',
        'python3 ../tests/validate_unreal_demo.py',
        'python3 ../tests/validate_repo_analysis_demo.py',
    ]:
        require_contains(
            release_verify_text,
            needle,
            f'release-verify.sh does not run required readiness/demo validator: {needle}',
        )

    workflow_expectations = {
        'blender workflow': (blender_workflow_text, ['does **not** directly run Blender UI interactions for you', 'Validate in Blender manually']),
        'unreal workflow': (unreal_workflow_text, ['does not do today', 'Manual validation loop']),
        'repo-analysis workflow': (repo_workflow_text, ['Manual validation loop', 'python3 tests/validate_repo_analysis_demo.py', 'run-repo-analysis-demo.sh', '.demo-artifacts/repo-analysis-demo/', 'operator-session-template.md', 'next-prompt-template.md']),
    }
    for label, (text, needles) in workflow_expectations.items():
        for needle in needles:
            require_contains(text, needle, f'{label} is missing honesty/validation cue: {needle}')

    for needle in [
        'scene cleanup demo kit',
        'unreal runtime telemetry demo kit',
        'repo analysis demo kit',
        'run-repo-analysis-demo.sh',
        'prepare-blender-demo.sh',
        'prepare-unreal-demo.sh',
    ]:
        require_contains(workflow_index_lower, needle, f'Workflow index is missing readiness cue: {needle}')
    require_contains(
        workflow_index_text,
        'Always run the lightweight validators before trusting a showcase',
        'Workflow index is missing the showcase-validator reminder.',
    )
    require_contains(
        workflow_index_text,
        'best current end-to-end showcase path',
        'Workflow index does not identify the best current showcase path.',
    )

    print('Operator readiness validation passed.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
