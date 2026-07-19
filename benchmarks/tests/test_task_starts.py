from __future__ import annotations

import importlib.util
import shutil
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
TOOL_PATH = ROOT / "benchmarks" / "tools" / "task_starts.py"
SPEC = importlib.util.spec_from_file_location("task_starts_tool", TOOL_PATH)
assert SPEC and SPEC.loader
task_starts = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = task_starts
SPEC.loader.exec_module(task_starts)


class TaskStartPackageTests(unittest.TestCase):
    def configuration(
        self,
        language: str,
        task: str,
    ) -> dict[str, object]:
        lock = task_starts._load_object(
            task_starts.LOCK_PATH,
            "test_lock_invalid",
        )
        configurations = task_starts._configuration_map(lock)
        return configurations[(language, task)]

    def build(
        self,
        root: Path,
        language: str,
        task: str,
    ) -> Path:
        workspace = root / language / task.lower()
        task_starts.build_task_start(language, task, workspace)
        return workspace

    def test_lock_matches_all_eight_deterministic_packages(self) -> None:
        task_starts.check_task_start_lock()

    def test_every_configuration_has_an_explicit_complete_file_manifest(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            tree_digests: set[str] = set()
            for language in task_starts.LANGUAGES:
                for task in task_starts.TASKS:
                    first = self.build(root / "first", language, task)
                    second = self.build(root / "second", language, task)
                    first_records = task_starts._tree_records(first)
                    second_records = task_starts._tree_records(second)
                    self.assertEqual(first_records, second_records)

                    configuration = self.configuration(language, task)
                    task_starts.verify_locked_workspace(first, configuration)
                    manifest_paths = [
                        entry["path"]
                        for entry in configuration["files"]  # type: ignore[index]
                    ]
                    self.assertEqual(
                        manifest_paths,
                        [record["path"] for record in first_records],
                    )
                    tree_digest = configuration["tree_sha256"]
                    self.assertIsInstance(tree_digest, str)
                    self.assertNotIn(tree_digest, tree_digests)
                    tree_digests.add(tree_digest)

    def test_uc001_exposes_contracts_tests_and_holes_only(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            for language in task_starts.LANGUAGES:
                workspace = self.build(root, language, "UC-001")
                task_starts.validate_workspace(language, "UC-001", workspace)
                source = (
                    workspace / task_starts.UC001_SOURCE_PATHS[language]
                ).read_text(encoding="utf-8")
                self.assertIn(task_starts.UC001_HOLE_MARKERS[language], source)
                for marker in task_starts.UC001_CONTRACT_MARKERS[language]:
                    self.assertIn(marker, source)

    def test_uc003_exposes_exact_v1_and_v2_tests_without_reference_source(
        self,
    ) -> None:
        reference_digests = {
            task_starts._sha256(ROOT / path)
            for path in task_starts.V2_REFERENCE_SOURCE_FILES
        }
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            for language in task_starts.LANGUAGES:
                workspace = self.build(root, language, "UC-003")
                task_starts.validate_workspace(language, "UC-003", workspace)
                for path in task_starts.UC003_ACCEPTED_V1_FILES[language]:
                    self.assertEqual(
                        task_starts._sha256(workspace / path),
                        task_starts._sha256(ROOT / path),
                    )
                self.assertTrue(
                    reference_digests.isdisjoint(
                        {
                            record["sha256"]
                            for record in task_starts._tree_records(workspace)
                        }
                    )
                )

    def test_other_language_and_freeze_metadata_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            workspace = self.build(root, "rust", "UC-001")
            other = workspace / "benchmarks" / "baselines" / "go" / "unexpected.go"
            other.parent.mkdir(parents=True)
            other.write_text("package go_answer\n", encoding="utf-8")
            with self.assertRaisesRegex(
                task_starts.TaskStartError,
                "task_start_other_language_exposed",
            ):
                task_starts.validate_workspace("rust", "UC-001", workspace)

            other.unlink()
            other.parent.rmdir()
            freeze = workspace / "checkpoints.json"
            freeze.write_text("{}\n", encoding="utf-8")
            with self.assertRaisesRegex(
                task_starts.TaskStartError,
                "task_start_freeze_metadata_exposed",
            ):
                task_starts.validate_workspace("rust", "UC-001", workspace)

    def test_reference_answer_under_a_different_name_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            workspace = self.build(Path(raw), "rust", "UC-003")
            leaked = workspace / "benchmarks" / "baselines" / "rust" / "answer.rs"
            shutil.copyfile(
                ROOT / "benchmarks/baselines/rust/v2/src/service.rs",
                leaked,
            )
            with self.assertRaisesRegex(
                task_starts.TaskStartError,
                "task_start_answer_leak",
            ):
                task_starts.validate_workspace("rust", "UC-003", workspace)

    def test_protected_artifact_change_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            workspace = self.build(Path(raw), "python", "UC-003")
            configuration = self.configuration("python", "UC-003")
            task = workspace / "TASK.md"
            task.write_text(task.read_text(encoding="utf-8") + "\n", encoding="utf-8")
            with self.assertRaisesRegex(
                task_starts.TaskStartError,
                "task_start_protected_artifact_changed",
            ):
                task_starts.verify_locked_workspace(workspace, configuration)

    def test_nonempty_destination_is_never_replaced(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            destination = Path(raw) / "existing"
            destination.mkdir()
            marker = destination / "user-owned.txt"
            marker.write_text("preserve\n", encoding="utf-8")
            with self.assertRaisesRegex(
                task_starts.TaskStartError,
                "task_start_destination_not_empty",
            ):
                task_starts.build_task_start("go", "UC-001", destination)
            self.assertEqual(marker.read_text(encoding="utf-8"), "preserve\n")


if __name__ == "__main__":
    unittest.main()
