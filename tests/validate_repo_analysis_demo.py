from __future__ import annotations

from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEMO_ROOT = REPO_ROOT / 'docs' / 'examples' / 'repo-analysis-demo'
REQUIRED_FILES = [
    DEMO_ROOT / 'README.md',
    DEMO_ROOT / 'brief.md',
    DEMO_ROOT / 'expected-findings.md',
    DEMO_ROOT / 'manual-validation-checklist.md',
    DEMO_ROOT / 'operator-session-template.md',
    DEMO_ROOT / 'next-prompt-template.md',
    DEMO_ROOT / 'trace-review-checklist.md',
    REPO_ROOT / 'docs' / 'workflows' / 'README.md',
    REPO_ROOT / 'docs' / 'workflows' / 'repo-analysis.md',
    REPO_ROOT / 'rust' / 'scripts' / 'run-repo-analysis-demo.sh',
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
    for needle in [
        'python3 tests/validate_repo_analysis_demo.py',
        './scripts/run-repo-analysis-demo.sh',
        '.demo-artifacts/repo-analysis-demo/',
        'operator-session-template.md',
        'next-prompt-template.md',
        'operator-dashboard.html',
        'bundle-summary.json',
        'operator-handoff.json',
        'bundle-checksums.txt',
        '/trace replay',
        '/trace resume',
    ]:
        if needle not in readme_text:
            print(f'Repo analysis demo README is missing required operator cue: {needle}')
            return 1

    workflow_text = (REPO_ROOT / 'docs' / 'workflows' / 'repo-analysis.md').read_text()
    for needle in [
        'repo-analysis-demo',
        'manual-validation-checklist.md',
        'operator-session-template.md',
        'next-prompt-template.md',
        'trace-review-checklist.md',
        'run-repo-analysis-demo.sh',
        '.demo-artifacts/repo-analysis-demo/',
        'operator-dashboard.html',
        'bundle-summary.json',
        'operator-handoff.json',
        'bundle-checksums.txt',
    ]:
        if needle not in workflow_text:
            print(f'Repo analysis workflow does not mention required demo asset/operator cue: {needle}')
            return 1

    index_text = (REPO_ROOT / 'docs' / 'workflows' / 'README.md').read_text().lower()
    for needle in [
        'repo analysis demo kit',
        'run-repo-analysis-demo.sh',
        'best current end-to-end showcase path',
        'operator-dashboard.html',
        'bundle-summary.json',
        'operator-handoff.json',
        'bundle-checksums.txt',
    ]:
        if needle not in index_text:
            print(f'Workflow index is missing required showcase cue: {needle}')
            return 1

    script_text = (REPO_ROOT / 'rust' / 'scripts' / 'run-repo-analysis-demo.sh').read_text()
    for needle in [
        'bundle-summary.json',
        'operator-handoff.json',
        'operator-dashboard.html',
        'bundle-checksums.txt',
        '/trace replay <trace-file|approval-packet>',
        '/trace resume <trace-file|approval-packet>',
    ]:
        if needle not in script_text:
            print(f'run-repo-analysis-demo.sh is missing required lifecycle artifact or continuity command: {needle}')
            return 1

    print('Repo analysis demo validation passed.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
