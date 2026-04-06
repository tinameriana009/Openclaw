from __future__ import annotations

import subprocess
import sys
import unittest
import zipfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEMO_ROOT = REPO_ROOT / 'docs' / 'examples' / 'blender-scene-cleanup-demo'
PACKAGE_SCRIPT = DEMO_ROOT / 'tools' / 'package_demo_addon.py'
ZIP_PATH = DEMO_ROOT / 'dist' / 'scene_cleanup_helper_demo.zip'


class BlenderDemoAssetTests(unittest.TestCase):
    def test_validator_script_runs(self) -> None:
        result = subprocess.run(
            [sys.executable, 'tests/validate_blender_demo.py'],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn('Blender demo validation passed.', result.stdout)

    def test_packaging_helper_creates_installable_zip(self) -> None:
        result = subprocess.run(
            [sys.executable, str(PACKAGE_SCRIPT)],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn('scene_cleanup_helper_demo.zip', result.stdout)
        self.assertTrue(ZIP_PATH.exists())
        with zipfile.ZipFile(ZIP_PATH) as archive:
            names = set(archive.namelist())
        self.assertIn('scene_cleanup_helper/__init__.py', names)
        self.assertIn('scene_cleanup_helper/operators.py', names)


if __name__ == '__main__':
    unittest.main()
