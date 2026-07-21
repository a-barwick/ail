from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
TOOL_PATH = ROOT / "benchmarks" / "tools" / "calibration.py"
SPEC = importlib.util.spec_from_file_location("calibration_tool", TOOL_PATH)
assert SPEC and SPEC.loader
calibration = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = calibration
SPEC.loader.exec_module(calibration)


class CalibrationTaskStartTests(unittest.TestCase):
    def test_complete_configuration_matrix_builds(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            for language in calibration.LANGUAGES:
                for task in calibration.TASKS:
                    workspace = root / language / task
                    calibration.build_task_start(language, task, workspace)
                    self.assertTrue((workspace / "task-start.json").is_file())
                    self.assertTrue(
                        (
                            workspace
                            / "benchmarks"
                            / "baselines"
                            / language
                            / "v1"
                        ).is_dir()
                    )

    def test_uc001_has_holes_and_no_v2_source(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            for language in calibration.LANGUAGES:
                workspace = Path(raw) / language
                calibration.build_task_start(language, "UC-001", workspace)
                baseline = workspace / "benchmarks" / "baselines" / language
                source = "\n".join(
                    path.read_text(encoding="utf-8")
                    for path in sorted((baseline / "v1").rglob("*"))
                    if path.is_file()
                )
                self.assertIn("implement UC-001", source)
                self.assertFalse((baseline / "v2").exists())

    def test_uc003_has_v1_and_tests_but_no_v2_implementation(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            for language in calibration.LANGUAGES:
                workspace = Path(raw) / language
                calibration.build_task_start(language, "UC-003", workspace)
                baseline = workspace / "benchmarks" / "baselines" / language
                self.assertTrue((baseline / "v1").is_dir())
                v2_files = [
                    path.relative_to(baseline).as_posix()
                    for path in baseline.rglob("*")
                    if path.is_file() and "/v2/" in f"/{path.relative_to(baseline).as_posix()}"
                ]
                if language in {"rust", "go"}:
                    self.assertTrue(v2_files)
                    self.assertTrue(
                        all(
                            "/tests/" in f"/{path}/"
                            or Path(path).name.endswith("_test.go")
                            for path in v2_files
                        )
                    )
                else:
                    self.assertFalse(v2_files)
                    self.assertTrue((baseline / "tests").is_dir())


if __name__ == "__main__":
    unittest.main()
