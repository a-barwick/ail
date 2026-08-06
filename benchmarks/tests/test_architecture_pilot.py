from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
TOOLS = ROOT / "benchmarks" / "tools"
sys.path.insert(0, str(TOOLS))
TOOL_PATH = TOOLS / "architecture_pilot.py"
SPEC = importlib.util.spec_from_file_location("architecture_pilot_tool", TOOL_PATH)
assert SPEC and SPEC.loader
pilot = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = pilot
SPEC.loader.exec_module(pilot)


class ArchitecturePilotTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.pilot_root = self.root / "benchmarks" / "architecture-pilot"
        schema_root = self.root / "benchmarks" / "schemas"
        candidate_root = self.root / "specs" / "architecture-acceptance-fixtures"
        result_root = self.root / "specs" / "architecture-fixtures"
        self.pilot_root.mkdir(parents=True)
        schema_root.mkdir(parents=True)
        candidate_root.mkdir(parents=True)
        result_root.mkdir(parents=True)

        for name in (
            "pilot.json",
            "pilot.lock.json",
            "prompt.txt",
            "operator-report.json",
            "repaired-candidate.json",
        ):
            shutil.copy2(pilot.PILOT_ROOT / name, self.pilot_root / name)
        shutil.copy2(
            ROOT / "benchmarks" / "schemas" / "architecture-pilot.schema.json",
            schema_root / "architecture-pilot.schema.json",
        )
        shutil.copy2(pilot.CANDIDATES, candidate_root / "candidates.json")
        shutil.copy2(pilot.RESULTS, result_root / "results.json")
        self.manifest_path = self.pilot_root / "pilot.json"
        self.lock_path = self.pilot_root / "pilot.lock.json"

    def _load(self, path: Path) -> dict[str, object]:
        return json.loads(path.read_text(encoding="utf-8"))

    def _write_canonical(self, path: Path, value: object) -> None:
        path.write_text(pilot._canonical(value), encoding="utf-8")

    def _refresh_lock(self) -> None:
        lock = self._load(self.lock_path)
        lock["manifest_sha256"] = hashlib.sha256(
            self.manifest_path.read_bytes()
        ).hexdigest()
        self._write_canonical(self.lock_path, lock)

    def _change_manifest(self, change: object) -> dict[str, object]:
        manifest = self._load(self.manifest_path)
        change(manifest)
        self._write_canonical(self.manifest_path, manifest)
        self._refresh_lock()
        return manifest

    def _change_artifact(
        self,
        key: str,
        change: object,
        *,
        canonical: bool = True,
    ) -> tuple[Path, str]:
        manifest = self._load(self.manifest_path)
        item = manifest["artifacts"][key]
        path = self.pilot_root / item["path"]
        value = self._load(path)
        change(value)
        if canonical:
            self._write_canonical(path, value)
        else:
            path.write_text(json.dumps(value, separators=(",", ":")), encoding="utf-8")
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        item["sha256"] = digest
        self._write_canonical(self.manifest_path, manifest)
        self._refresh_lock()
        return path, digest

    def _verify(self) -> dict[str, object]:
        return pilot.verify_architecture_pilot(
            self.manifest_path,
            lock_path=self.lock_path,
            root=self.root,
        )

    def _assert_rejected(self, code: str | None = None) -> pilot.ArchitecturePilotError:
        with self.assertRaises(pilot.ArchitecturePilotError) as raised:
            self._verify()
        if code is not None:
            self.assertEqual(raised.exception.code, code)
        return raised.exception

    def test_complete_replayable_pilot_is_accepted(self) -> None:
        manifest = self._verify()
        self.assertIs(manifest["official"], False)

    def test_altered_candidate_bytes_or_digest_are_rejected(self) -> None:
        candidate = self.pilot_root / "repaired-candidate.json"
        original = candidate.read_bytes()
        candidate.write_bytes(original + b" ")
        self._assert_rejected("architecture_pilot_changed")

        candidate.write_bytes(original)
        self._change_manifest(
            lambda manifest: manifest["artifacts"]["repaired_candidate"].update(
                {"sha256": "0" * 64}
            )
        )
        self._assert_rejected("architecture_pilot_changed")

    def test_missing_metadata_is_rejected(self) -> None:
        self._change_manifest(lambda manifest: manifest["operator"].pop("version"))
        self._assert_rejected("architecture_pilot_invalid")

    def test_noncanonical_json_is_rejected(self) -> None:
        _, digest = self._change_artifact(
            "operator_report", lambda report: None, canonical=False
        )
        with mock.patch.object(pilot, "OPERATOR_REPORT_DIGEST", digest):
            self._assert_rejected("architecture_pilot_noncanonical")

    def test_altered_compact_output_is_rejected(self) -> None:
        _, digest = self._change_artifact(
            "operator_report",
            lambda report: report["initial_attempt"].update(
                {"snapshot_compact": "altered\n"}
            ),
        )
        with mock.patch.object(pilot, "OPERATOR_REPORT_DIGEST", digest):
            self._assert_rejected("architecture_pilot_output_changed")

    def test_altered_behavior_evidence_is_rejected(self) -> None:
        def alter(candidate: dict[str, object]) -> None:
            candidate["observed_results"][1]["outcome"] = "NotFound"

        _, digest = self._change_artifact("repaired_candidate", alter)
        with mock.patch.object(pilot, "REPAIRED_CANDIDATE_DIGEST", digest):
            self._assert_rejected("architecture_pilot_behavior_changed")

    def test_official_claim_is_rejected(self) -> None:
        self._change_manifest(lambda manifest: manifest.update({"official": True}))
        self._assert_rejected("architecture_pilot_invalid")

    def test_transport_owned_repair_is_rejected(self) -> None:
        def move_owner(candidate: dict[str, object]) -> None:
            candidate["operation"]["implementation_owner"] = "transport:dispatch"

        _, digest = self._change_artifact("repaired_candidate", move_owner)
        with mock.patch.object(pilot, "REPAIRED_CANDIDATE_DIGEST", digest):
            self._assert_rejected("architecture_pilot_ownership_changed")

    def test_unrecorded_post_comparison_change_is_rejected(self) -> None:
        def alter(candidate: dict[str, object]) -> None:
            candidate["governance"] = copy.deepcopy(candidate["governance"])
            candidate["governance"]["exceptions"] = ["post-comparison-change"]

        _, digest = self._change_artifact("repaired_candidate", alter)
        with mock.patch.object(pilot, "REPAIRED_CANDIDATE_DIGEST", digest):
            self._assert_rejected("architecture_pilot_post_comparison_change")


if __name__ == "__main__":
    unittest.main()
