from __future__ import annotations

import py_compile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEMO_ROOT = REPO_ROOT / 'docs' / 'examples' / 'blender-scene-cleanup-demo'
ADDON_ROOT = DEMO_ROOT / 'addon' / 'scene_cleanup_helper'

REQUIRED_FILES = [
    DEMO_ROOT / 'README.md',
    DEMO_ROOT / 'brief.md',
    DEMO_ROOT / 'manual-test-checklist.md',
    ADDON_ROOT / '__init__.py',
    ADDON_ROOT / 'properties.py',
    ADDON_ROOT / 'operators.py',
    ADDON_ROOT / 'ui.py',
    REPO_ROOT / 'docs' / 'workflows' / 'blender-addon.md',
    REPO_ROOT / 'docs' / 'workflows' / 'README.md',
]


def main() -> int:
    missing = [path for path in REQUIRED_FILES if not path.exists()]
    if missing:
        print('Missing demo files:')
        for path in missing:
            print(f'- {path.relative_to(REPO_ROOT)}')
        return 1

    for path in ADDON_ROOT.glob('*.py'):
        py_compile.compile(str(path), doraise=True)

    workflow_text = (REPO_ROOT / 'docs' / 'workflows' / 'blender-addon.md').read_text()
    if 'blender-scene-cleanup-demo' not in workflow_text:
        print('Workflow doc does not mention the Blender demo kit.')
        return 1

    index_text = (REPO_ROOT / 'docs' / 'workflows' / 'README.md').read_text()
    if 'scene cleanup demo kit' not in index_text.lower():
        print('Workflow index does not mention the scene cleanup demo kit.')
        return 1

    print('Blender demo validation passed.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
