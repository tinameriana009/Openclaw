from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
PREP_SCRIPT = REPO_ROOT / 'rust' / 'scripts' / 'prepare-unreal-demo.sh'


class UnrealDemoAssetTests(unittest.TestCase):
    def test_validator_script_runs(self) -> None:
        result = subprocess.run(
            [sys.executable, 'tests/validate_unreal_demo.py'],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn('Unreal demo validation passed.', result.stdout)

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
            self.assertEqual(summary['workflow'], 'unreal-demo')
            self.assertTrue(summary['manualValidationRequired'])
            self.assertIn('RuntimeTelemetry/', summary['bundleEntries'])

            checksums_text = (run_dir / 'bundle-checksums.txt').read_text()
            self.assertIn('./RuntimeTelemetry/RuntimeTelemetry.uplugin', checksums_text)
            self.assertIn('./bundle-summary.json', checksums_text)


if __name__ == '__main__':
    unittest.main()
