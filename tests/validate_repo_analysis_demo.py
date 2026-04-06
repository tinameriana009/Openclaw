from __future__ import annotations

from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEMO_ROOT = REPO_ROOT / 'docs' / 'examples' / 'repo-analysis-demo'
REQUIRED_FILES = [
    DEMO_ROOT / 'README.md',
    DEMO_ROOT / 'brief.md',
    DEMO_ROOT / 'expected-findings.md',
    DEMO_ROOT / 'manual-validation-checklist.md',
    DEMO_ROOT / 'trace-review-checklist.md',
    REPO_ROOT / 'docs' / 'workflows' / 'README.md',
    REPO_ROOT / 'docs' / 'workflows' / 'repo-analysis.md',
]


def main() -> int:
    missing = [path for path in REQUIRED_FILES if not path.exists()]
    if missing:
        print('Missing repo-analysis demo files:')
        for path in missing:
            print(f'- {path.relative_to(REPO_ROOT)}')
        return 1

    expected_text = (DEMO_ROOT / 'expected-findings.md').read_text()
    for needle in ['src/main.py', 'src/runtime.py', 'src/execution_registry.py', 'tests/test_porting_workspace.py']:
        if needle not in expected_text:
            print(f'Expected findings are missing required reference: {needle}')
            return 1

    readme_text = (DEMO_ROOT / 'README.md').read_text()
    if 'python3 tests/validate_repo_analysis_demo.py' not in readme_text:
        print('Repo analysis demo README does not explain how to run validation.')
        return 1

    workflow_text = (REPO_ROOT / 'docs' / 'workflows' / 'repo-analysis.md').read_text()
    for needle in ['repo-analysis-demo', 'manual-validation-checklist.md', 'trace-review-checklist.md']:
        if needle not in workflow_text:
            print(f'Repo analysis workflow does not mention required demo asset: {needle}')
            return 1

    index_text = (REPO_ROOT / 'docs' / 'workflows' / 'README.md').read_text().lower()
    if 'repo analysis demo kit' not in index_text:
        print('Workflow index does not mention the repo analysis demo kit.')
        return 1

    print('Repo analysis demo validation passed.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
