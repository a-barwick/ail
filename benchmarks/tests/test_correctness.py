from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
TOOLS = ROOT / "benchmarks" / "tools"
sys.path.insert(0, str(TOOLS))
TOOL_PATH = TOOLS / "correctness.py"
SPEC = importlib.util.spec_from_file_location("correctness_tool", TOOL_PATH)
assert SPEC and SPEC.loader
correctness = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = correctness
SPEC.loader.exec_module(correctness)


class CorrectnessVerifierTests(unittest.TestCase):
    def test_fake_and_dry_outcomes_are_stable(self) -> None:
        self.assertEqual(
            correctness.verify_fake_and_dry_correctness(),
            [
                ("clean", "complete"),
                ("incomplete", "incomplete_final_revision"),
                ("answer-exposure", "answer_exposure"),
                ("protected-artifact", "protected_artifact_changed"),
                ("permission", "permission_violation"),
                ("revision-mismatch", "revision_mismatch"),
                ("seeded-regression", "seeded_regression"),
                ("replay-mismatch", "replay_mismatch"),
            ],
        )


if __name__ == "__main__":
    unittest.main()
