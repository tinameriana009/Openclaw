from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]


class RepoAnalysisDemoAssetTests(unittest.TestCase):
    def test_validator_script_runs(self) -> None:
        result = subprocess.run(
            [sys.executable, 'tests/validate_repo_analysis_demo.py'],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn('Repo analysis demo validation passed.', result.stdout)


if __name__ == '__main__':
    unittest.main()
