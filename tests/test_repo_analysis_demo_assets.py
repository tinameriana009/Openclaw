from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT_PATH = REPO_ROOT / 'rust' / 'scripts' / 'run-repo-analysis-demo.sh'


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

    def test_demo_runner_shell_syntax_is_valid(self) -> None:
        result = subprocess.run(
            ['bash', '-n', str(SCRIPT_PATH)],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.stdout, '')
        self.assertEqual(result.stderr, '')


if __name__ == '__main__':
    unittest.main()
