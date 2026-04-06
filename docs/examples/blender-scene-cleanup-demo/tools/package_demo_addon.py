from __future__ import annotations

from pathlib import Path
import zipfile


SCRIPT_PATH = Path(__file__).resolve()
DEMO_ROOT = SCRIPT_PATH.parents[1]
ADDON_ROOT = DEMO_ROOT / 'addon' / 'scene_cleanup_helper'
DIST_ROOT = DEMO_ROOT / 'dist'
ZIP_PATH = DIST_ROOT / 'scene_cleanup_helper_demo.zip'


def build_zip() -> Path:
    DIST_ROOT.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(ZIP_PATH, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(ADDON_ROOT.rglob('*')):
            if not path.is_file() or '__pycache__' in path.parts:
                continue
            archive.write(path, arcname=path.relative_to(ADDON_ROOT.parent))
    return ZIP_PATH


if __name__ == '__main__':
    output = build_zip()
    print(output)
