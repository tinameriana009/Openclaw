from __future__ import annotations

import py_compile
import runpy
from pathlib import Path
import zipfile


REPO_ROOT = Path(__file__).resolve().parents[1]
DEMO_ROOT = REPO_ROOT / 'docs' / 'examples' / 'blender-scene-cleanup-demo'
ADDON_ROOT = DEMO_ROOT / 'addon' / 'scene_cleanup_helper'
PACKAGE_SCRIPT = DEMO_ROOT / 'tools' / 'package_demo_addon.py'
DIST_ZIP = DEMO_ROOT / 'dist' / 'scene_cleanup_helper_demo.zip'

REQUIRED_FILES = [
    DEMO_ROOT / 'README.md',
    DEMO_ROOT / 'brief.md',
    DEMO_ROOT / 'manual-test-checklist.md',
    DEMO_ROOT / 'next-prompt-template.md',
    DEMO_ROOT / 'validation-baseline.md',
    ADDON_ROOT / '__init__.py',
    ADDON_ROOT / 'properties.py',
    ADDON_ROOT / 'operators.py',
    ADDON_ROOT / 'ui.py',
    PACKAGE_SCRIPT,
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
    py_compile.compile(str(PACKAGE_SCRIPT), doraise=True)

    package_module = runpy.run_path(str(PACKAGE_SCRIPT))
    zip_path = package_module['build_zip']()
    if not zip_path.exists():
        print('Packaging helper did not emit a zip artifact.')
        return 1

    with zipfile.ZipFile(zip_path) as archive:
        names = set(archive.namelist())
    required_archive_members = {
        'scene_cleanup_helper/__init__.py',
        'scene_cleanup_helper/properties.py',
        'scene_cleanup_helper/operators.py',
        'scene_cleanup_helper/ui.py',
    }
    if not required_archive_members.issubset(names):
        print('Packaged zip is missing expected add-on files.')
        return 1

    workflow_text = (REPO_ROOT / 'docs' / 'workflows' / 'blender-addon.md').read_text()
    if 'blender-scene-cleanup-demo' not in workflow_text:
        print('Workflow doc does not mention the Blender demo kit.')
        return 1
    if 'validation-baseline.md' not in workflow_text:
        print('Workflow doc does not mention the Blender validation baseline.')
        return 1

    index_text = (REPO_ROOT / 'docs' / 'workflows' / 'README.md').read_text()
    if 'scene cleanup demo kit' not in index_text.lower():
        print('Workflow index does not mention the scene cleanup demo kit.')
        return 1

    rust_readme_text = (REPO_ROOT / 'rust' / 'README.md').read_text()
    if './scripts/prepare-blender-demo.sh' not in rust_readme_text:
        print('rust/README.md does not mention the Blender demo prep helper.')
        return 1

    readme_text = (DEMO_ROOT / 'README.md').read_text()
    if 'prepare-blender-demo.sh' not in readme_text or '.demo-artifacts/blender-demo/' not in readme_text:
        print('Demo README does not mention the Blender prep helper and staged artifact path.')
        return 1
    if 'next-prompt-template.md' not in readme_text or 'operator-findings-template.md' not in readme_text:
        print('Demo README does not mention the staged handoff templates.')
        return 1

    checklist_text = (DEMO_ROOT / 'manual-test-checklist.md').read_text()
    if 'duplicate materials = `1`' not in checklist_text or 'unapplied transforms = `3`' not in checklist_text:
        print('Manual checklist does not contain the expected validation counts.')
        return 1
    if 'python3 tests/validate_blender_demo.py' not in readme_text:
        print('Demo README does not explain how to run validation.')
        return 1

    print('Blender demo validation passed.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
