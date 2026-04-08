from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEMO_ROOT = REPO_ROOT / 'docs' / 'examples' / 'blender-scene-cleanup-demo'
PACKAGE_SCRIPT = DEMO_ROOT / 'tools' / 'package_demo_addon.py'
ZIP_PATH = DEMO_ROOT / 'dist' / 'scene_cleanup_helper_demo.zip'
PREP_SCRIPT = REPO_ROOT / 'rust' / 'scripts' / 'prepare-blender-demo.sh'


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

    def test_prep_helper_stages_summary_and_checksums(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            env = os.environ.copy()
            env['ARTIFACT_ROOT'] = tmpdir
            subprocess.run(
                ['bash', str(PREP_SCRIPT)],
                cwd=REPO_ROOT / 'rust',
                check=True,
                capture_output=True,
                text=True,
                env=env,
            )

            run_dirs = sorted(Path(tmpdir).iterdir())
            self.assertEqual(len(run_dirs), 1)
            run_dir = run_dirs[0]

            summary = json.loads((run_dir / 'bundle-summary.json').read_text())
            self.assertEqual(summary['workflow'], 'blender-demo')
            self.assertTrue(summary['manualValidationRequired'])
            self.assertIn('scene_cleanup_helper_demo.zip', summary['bundleEntries'])

            checksums_text = (run_dir / 'bundle-checksums.txt').read_text()
            self.assertIn('./scene_cleanup_helper_demo.zip', checksums_text)
            self.assertIn('./bundle-summary.json', checksums_text)


if __name__ == '__main__':
    unittest.main()
