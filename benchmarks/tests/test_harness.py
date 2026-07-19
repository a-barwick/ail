from __future__ import annotations

import importlib.util
import hashlib
import json
import sys
import tempfile
import unittest
import zipfile
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

    def private_package(self) -> Path:
        """Create a deterministic test-only ZIP from accepted public behavior."""

        source_paths = (
            "benchmarks/fixtures/public/create_job/uc001-v1-multiple-invalid-ordered.json",
            "benchmarks/fixtures/public/create_job/uc001-v1-payload-maximum.json",
            "benchmarks/fixtures/public/create_job/uc003-v1-request-duplicate.json",
            "benchmarks/fixtures/public/create_job/uc003-v1-request-adapted.json",
            "benchmarks/fixtures/public/create_job/uc003-v2-priority-unknown.json",
        )
        categories = harness._hidden_categories()
        entries: dict[str, bytes] = {}
        cases: list[dict[str, str]] = []
        for index, (category, source) in enumerate(zip(categories, source_paths), 1):
            value = json.loads((ROOT / source).read_text(encoding="utf-8"))
            value["case_id"] = f"hidden-test-{index}"
            path = f"cases/{value['case_id']}.json"
            contents = harness._canonical(value).encode("utf-8")
            entries[path] = contents
            cases.append(
                {
                    "case_id": value["case_id"],
                    "behavior_category": category,
                    "path": path,
                    "sha256": hashlib.sha256(contents).hexdigest(),
                }
            )
        hidden_manifest = {
            "hidden_package_format": 1,
            "hidden_contract_sha256": hashlib.sha256(
                (ROOT / "benchmarks/contracts/hidden-contract.json").read_bytes()
            ).hexdigest(),
            "cases": cases,
        }
        entries["hidden-manifest.json"] = harness._canonical(hidden_manifest).encode(
            "utf-8"
        )
        package = self.directory / "private.zip"
        with zipfile.ZipFile(package, "w", compression=zipfile.ZIP_STORED) as archive:
            for name in sorted(entries):
                info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
                info.compress_type = zipfile.ZIP_STORED
                archive.writestr(info, entries[name])
        return package

    def private_manifest(self, package: Path) -> tuple[Path, Path]:
        value = harness._self_test_manifest()
        value["tests"]["hidden_package"] = "external:test-hidden.zip"
        value["tests"]["hidden_package_sha256"] = hashlib.sha256(
            package.read_bytes()
        ).hexdigest()
        return harness._write_self_test_lock(self.directory, value)

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
            harness._canonical(value), encoding="utf-8"
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

    def test_public_contract_cannot_be_used_as_a_hidden_package(self) -> None:
        with self.assertRaises(harness.HarnessError) as raised:
            harness.verify_from_paths(
                self.descriptor,
                self.manifest,
                self.lock,
                "hidden",
            )
        self.assertEqual(raised.exception.code, "hidden_package_invalid")

    def test_valid_private_package_runs_every_frozen_category(self) -> None:
        package = self.private_package()
        manifest, lock = self.private_manifest(package)
        verification = harness.verify_from_paths(
            self.descriptor,
            manifest,
            lock,
            "hidden",
            hidden_package=package,
        )
        self.assertEqual(len(verification.hidden), 5)
        self.assertEqual(
            [case.behavior_category for case, _ in verification.hidden],
            harness._hidden_categories(),
        )
        self.assertEqual(list(harness.HIDDEN_RUNTIME_ROOT.iterdir()), [])

    def test_private_package_digest_is_checked_before_extraction(self) -> None:
        package = self.private_package()
        manifest, lock = self.private_manifest(package)
        package.write_bytes(package.read_bytes() + b"changed")
        with self.assertRaises(harness.HarnessError) as raised:
            harness.verify_from_paths(
                self.descriptor,
                manifest,
                lock,
                "hidden",
                hidden_package=package,
            )
        self.assertEqual(raised.exception.code, "hidden_package_changed")

    def test_cross_language_comparison_classifies_a_difference(self) -> None:
        result = {"actual": {"response": {"result": "created"}}}
        changed = {"actual": {"response": {"result": "duplicate"}}}
        reference = harness.Verification("rust", (result,), ())
        other = harness.Verification("go", (changed,), ())
        with self.assertRaises(harness.HarnessError) as raised:
            harness._compare_baseline_observations((reference, other))
        self.assertEqual(raised.exception.code, "cross_language_mismatch")


if __name__ == "__main__":
    unittest.main()
