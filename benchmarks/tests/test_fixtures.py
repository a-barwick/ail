from __future__ import annotations

import copy
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
TOOL_PATH = ROOT / "benchmarks" / "tools" / "fixtures.py"
SPEC = importlib.util.spec_from_file_location("fixture_tool", TOOL_PATH)
assert SPEC and SPEC.loader
fixtures = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = fixtures
SPEC.loader.exec_module(fixtures)


class FixtureToolTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = fixtures._load_json(fixtures.SCHEMA_PATH)
        cls.valid_path = next(
            path
            for path in fixtures.fixture_paths()
            if path.stem == "uc001-v1-created-empty-payload"
        )
        cls.valid_case = fixtures._load_json(cls.valid_path)

    def write_case(self, case: dict[str, object], name: str | None = None) -> Path:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        path = Path(directory.name) / f"{name or case['case_id']}.json"
        path.write_text(fixtures.canonical_json(case), encoding="utf-8")
        return path

    def test_complete_corpus_passes(self) -> None:
        fixtures.check_all()

    def test_noncanonical_base64_is_rejected(self) -> None:
        case = copy.deepcopy(self.valid_case)
        case["request"]["payload_base64"] = "YQ"
        path = self.write_case(case)
        with self.assertRaisesRegex(fixtures.FixtureError, "padded Base64"):
            fixtures.validate_fixture(path, self.schema)

    def test_changed_oracle_is_rejected(self) -> None:
        case = copy.deepcopy(self.valid_case)
        case["expected"]["store_calls"] = []
        path = self.write_case(case)
        with self.assertRaisesRegex(fixtures.FixtureError, "oracle does not match"):
            fixtures.validate_fixture(path, self.schema)

    def test_store_outcome_on_invalid_case_is_rejected(self) -> None:
        case = copy.deepcopy(self.valid_case)
        case["case_id"] = "invalid-with-store-outcome"
        case["request"]["job_id"] = ""
        path = self.write_case(case)
        with self.assertRaisesRegex(fixtures.FixtureError, "must omit store_outcome"):
            fixtures.validate_fixture(path, self.schema)

    def test_noncanonical_property_order_is_rejected(self) -> None:
        case = copy.deepcopy(self.valid_case)
        case["request"] = {
            "task": case["request"]["task"],
            "api_version": case["request"]["api_version"],
            "job_id": case["request"]["job_id"],
            "payload_base64": case["request"]["payload_base64"],
        }
        path = self.write_case(case)
        with self.assertRaisesRegex(fixtures.FixtureError, "canonical order"):
            fixtures.validate_fixture(path, self.schema)

    def test_manifest_digest_change_is_rejected(self) -> None:
        cases = {
            fixtures._relative(path): fixtures.validate_fixture(path, self.schema)
            for path in fixtures.fixture_paths()
        }
        manifest = fixtures._load_json(fixtures.MANIFEST_PATH)
        manifest["fixtures"][0]["sha256"] = "0" * 64
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        changed_manifest = Path(directory.name) / "manifest.json"
        changed_manifest.write_text(
            fixtures.canonical_json(manifest), encoding="utf-8"
        )
        with mock.patch.object(fixtures, "MANIFEST_PATH", changed_manifest):
            with self.assertRaisesRegex(fixtures.FixtureError, "SHA-256"):
                fixtures.check_manifest(cases)

    def test_missing_required_coverage_is_rejected(self) -> None:
        manifest = fixtures._load_json(fixtures.MANIFEST_PATH)
        entries = copy.deepcopy(manifest["fixtures"])
        for entry in entries:
            entry["coverage"] = [
                "uc001:payload_empty"
                if tag == "uc001:payload_too_large"
                else tag
                for tag in entry["coverage"]
            ]
        with self.assertRaisesRegex(fixtures.FixtureError, "missing UC-001 coverage"):
            fixtures._check_traceability(entries)


if __name__ == "__main__":
    unittest.main()
