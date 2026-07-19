from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
TOOLS = ROOT / "benchmarks" / "tools"
sys.path.insert(0, str(TOOLS))
TOOL_PATH = TOOLS / "harness.py"
SPEC = importlib.util.spec_from_file_location("harness_tool", TOOL_PATH)
assert SPEC and SPEC.loader
harness = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = harness
SPEC.loader.exec_module(harness)


class HarnessTests(unittest.TestCase):
    def setUp(self) -> None:
        self.descriptor = (
            ROOT / "benchmarks" / "tests" / "support" / "fake-runner.json"
        )
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.manifest, self.lock = harness._write_self_test_lock(
            self.directory, harness._self_test_manifest()
        )

    def run_mode(self, mode: str) -> None:
        harness.verify_from_paths(
            self.descriptor,
            self.manifest,
            self.lock,
            "public",
            test_environment={"AIL_FAKE_MODE": mode},
        )

    def test_passing_fake_runner_is_accepted(self) -> None:
        self.run_mode("pass")

    def test_each_observable_has_a_distinct_failure(self) -> None:
        for mode, code in (
            ("response_mismatch", "response_mismatch"),
            ("final_state_mismatch", "final_state_mismatch"),
            ("store_calls_mismatch", "store_calls_mismatch"),
        ):
            with self.subTest(mode=mode):
                with self.assertRaises(harness.HarnessError) as raised:
                    self.run_mode(mode)
                self.assertEqual(raised.exception.code, code)

    def test_changed_manifest_is_rejected_before_runner_start(self) -> None:
        marker = self.directory / "started"
        value = harness._load_object(self.manifest, "test")
        value["configuration_id"] = "changed"
        self.manifest.write_text(
            harness._canonical(value), encoding="utf-8", newline="\n"
        )
        with self.assertRaises(harness.HarnessError) as raised:
            harness.verify_from_paths(
                self.descriptor,
                self.manifest,
                self.lock,
                "public",
                test_environment={"AIL_FAKE_MARKER": str(marker)},
            )
        self.assertEqual(raised.exception.code, "manifest_changed")
        self.assertFalse(marker.exists())

    def test_incomplete_manifest_is_rejected_before_runner_start(self) -> None:
        marker = self.directory / "started"
        value = harness._self_test_manifest()
        value.pop("permissions")
        self.manifest, self.lock = harness._write_self_test_lock(
            self.directory, value
        )
        with self.assertRaises(harness.HarnessError) as raised:
            harness.verify_from_paths(
                self.descriptor,
                self.manifest,
                self.lock,
                "public",
                test_environment={"AIL_FAKE_MARKER": str(marker)},
            )
        self.assertEqual(raised.exception.code, "run_manifest_invalid")
        self.assertFalse(marker.exists())

    def test_hidden_visibility_waits_for_language_instantiation(self) -> None:
        with self.assertRaises(harness.HarnessError) as raised:
            harness.verify_from_paths(
                self.descriptor,
                self.manifest,
                self.lock,
                "hidden",
            )
        self.assertEqual(
            raised.exception.code, "hidden_package_uninstantiated"
        )


if __name__ == "__main__":
    unittest.main()
