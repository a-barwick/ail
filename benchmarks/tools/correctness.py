#!/usr/bin/env python3
"""Post-run correctness, completion-evidence, and replay verification for M8."""

from __future__ import annotations

import hashlib
import json
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, NoReturn

import agent_runner
import fixtures as fixture_tool
import task_starts


ROOT = Path(__file__).resolve().parents[2]
FIXTURE_MANIFEST = ROOT / "benchmarks" / "fixtures" / "manifest.json"
HIDDEN_CONTRACT = ROOT / "benchmarks" / "contracts" / "hidden-contract.json"
PARITY_REPORT = ROOT / "benchmarks" / "m7-parity-report.json"
FINAL_MANIFEST = ".ail-final-source-manifest.json"


@dataclass(frozen=True)
class CorrectnessError(Exception):
    code: str
    message: str

    def __str__(self) -> str:
        return f"{self.code}: {self.message}"


@dataclass(frozen=True)
class OracleObservation:
    public_cases: tuple[tuple[str, str], ...]
    private_cases: tuple[tuple[str, str, str], ...]
    seeded_roles: tuple[tuple[str, str], ...]
    functional_output_sha256: str


@dataclass(frozen=True)
class CorrectnessResult:
    terminal_class: str
    terminal_cause: str
    correctness: dict[str, str]
    protected_artifacts_match: str
    replay_sha256: str | None
    completion_evidence: dict[str, Any] | None


Oracle = Callable[[Path, Path, str, str], OracleObservation]


def _raise(code: str, message: str) -> NoReturn:
    raise CorrectnessError(code, message)


def _canonical_payload(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _sha256(path: Path) -> str:
    return _sha256_bytes(path.read_bytes())


def _load_object(path: Path, code: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        _raise(code, f"{path}: {error}")
    if not isinstance(value, dict):
        _raise(code, f"{path}: expected JSON object")
    return value


def _load_object_bytes(contents: bytes, code: str) -> dict[str, Any]:
    try:
        value = json.loads(contents.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        _raise(code, str(error))
    if not isinstance(value, dict):
        _raise(code, "expected JSON object")
    return value


def _configuration(prepared: agent_runner.PreparedTrial) -> dict[str, Any]:
    lock = _load_object(
        agent_runner.TASK_START_LOCK, "correctness_configuration_invalid"
    )
    return agent_runner._configuration(prepared.language, prepared.task, lock)


def _safe_archive_name(raw: str) -> bool:
    path = Path(raw)
    return (
        bool(raw)
        and not path.is_absolute()
        and ".." not in path.parts
        and raw == path.as_posix()
        and not raw.endswith("/")
    )


def _materialize_final_revision(
    archive_path: Path,
    destination: Path,
    expected_revision: str,
) -> dict[str, Any]:
    """Validate and extract the deterministic M8c final-source archive."""

    try:
        archive = zipfile.ZipFile(archive_path)
    except (OSError, zipfile.BadZipFile) as error:
        _raise("incomplete_final_revision", str(error))
    with archive:
        entries = archive.infolist()
        names = [entry.filename for entry in entries]
        if (
            not names
            or names[-1] != FINAL_MANIFEST
            or names[:-1] != sorted(names[:-1])
            or len(names) != len(set(names))
            or names.count(FINAL_MANIFEST) != 1
            or any(not _safe_archive_name(name) for name in names)
        ):
            _raise("incomplete_final_revision", "archive entry set is invalid")
        for entry in entries:
            file_type = (entry.external_attr >> 16) & 0o170000
            if (
                entry.compress_type != zipfile.ZIP_STORED
                or entry.date_time != (1980, 1, 1, 0, 0, 0)
                or entry.is_dir()
                or file_type != 0o100000
            ):
                _raise(
                    "incomplete_final_revision",
                    f"non-canonical archive entry {entry.filename}",
                )
        manifest = _load_object_bytes(
            archive.read(FINAL_MANIFEST), "incomplete_final_revision"
        )
        if list(manifest) != ["tree_sha256", "files"]:
            _raise("incomplete_final_revision", "final manifest shape differs")
        files = manifest["files"]
        if not isinstance(files, list) or not files:
            _raise("incomplete_final_revision", "final manifest is empty")
        paths = [
            record.get("path") for record in files if isinstance(record, dict)
        ]
        if (
            len(paths) != len(files)
            or paths != sorted(paths)
            or len(paths) != len(set(paths))
            or set(names) != {FINAL_MANIFEST, *paths}
        ):
            _raise("incomplete_final_revision", "manifest paths differ from archive")
        records: list[dict[str, str]] = []
        for record in files:
            if (
                list(record) != ["path", "sha256"]
                or not _safe_archive_name(record["path"])
                or not isinstance(record["sha256"], str)
                or len(record["sha256"]) != 64
            ):
                _raise("incomplete_final_revision", "file record is invalid")
            contents = archive.read(record["path"])
            digest = _sha256_bytes(contents)
            if digest != record["sha256"]:
                _raise(
                    "incomplete_final_revision",
                    f"{record['path']}: digest differs",
                )
            target = destination / record["path"]
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(contents)
            records.append({"path": record["path"], "sha256": digest})
    revision = task_starts._tree_digest(records)
    if revision != manifest["tree_sha256"] or revision != expected_revision:
        _raise("revision_mismatch", "retained final revision digest differs")
    return manifest


def _inside_root(path: str, root: str) -> bool:
    return path.startswith(root + "/")


def _check_final_boundary(
    prepared: agent_runner.PreparedTrial,
    manifest: dict[str, Any],
) -> None:
    configuration = _configuration(prepared)
    expected = {entry["path"]: entry for entry in configuration["files"]}
    actual = {entry["path"]: entry["sha256"] for entry in manifest["files"]}
    missing = sorted(set(expected) - set(actual))
    if missing:
        _raise("incomplete_final_revision", f"final source omits {missing}")
    for path, entry in expected.items():
        if entry["role"] == "protected" and actual[path] != entry["sha256"]:
            _raise("protected_artifact_changed", f"{path}: protected digest differs")

    extensions = {
        "rust": {".rs"},
        "go": {".go"},
        "python": {".py"},
        "typescript": {".ts"},
    }[prepared.language]
    source_roots = prepared.permission_profile["workspace_source_write_roots"]
    for path in sorted(set(actual) - set(expected)):
        if (
            not any(_inside_root(path, root) for root in source_roots)
            or Path(path).suffix not in extensions
        ):
            _raise("answer_exposure", f"unexpected retained file {path}")


def _read_private_manifest(
    private_package: Path, expected_digest: str
) -> tuple[dict[str, Any], ...]:
    if not private_package.is_file():
        _raise("private_package_missing", "private package is unavailable")
    if _sha256(private_package) != expected_digest:
        _raise("private_package_changed", "private package digest differs")
    try:
        archive = zipfile.ZipFile(private_package)
    except (OSError, zipfile.BadZipFile) as error:
        _raise("private_package_invalid", str(error))
    with archive:
        entries = archive.infolist()
        names = [entry.filename for entry in entries]
        if (
            not names
            or names != sorted(names)
            or len(names) != len(set(names))
            or "hidden-manifest.json" not in names
        ):
            _raise("private_package_invalid", "private archive entries differ")
        for entry in entries:
            if (
                entry.compress_type != zipfile.ZIP_STORED
                or entry.date_time != (1980, 1, 1, 0, 0, 0)
                or entry.is_dir()
            ):
                _raise("private_package_invalid", "private archive is non-canonical")
        manifest_bytes = archive.read("hidden-manifest.json")
        manifest = _load_object_bytes(manifest_bytes, "private_package_invalid")
        if (
            list(manifest) != [
                "hidden_package_format",
                "hidden_contract_sha256",
                "cases",
            ]
            or manifest["hidden_package_format"] != 1
            or manifest["hidden_contract_sha256"] != _sha256(HIDDEN_CONTRACT)
            or (
                json.dumps(manifest, ensure_ascii=False, indent=2) + "\n"
            ).encode("utf-8")
            != manifest_bytes
        ):
            _raise("private_package_invalid", "private manifest differs")
        cases = manifest["cases"]
        if not isinstance(cases, list) or not cases:
            _raise("private_package_invalid", "private case list is empty")
        paths = [entry.get("path") for entry in cases if isinstance(entry, dict)]
        if len(paths) != len(cases) or set(names) != {"hidden-manifest.json", *paths}:
            _raise("private_package_invalid", "private case paths differ")
        for entry in cases:
            if (
                list(entry) != ["case_id", "behavior_category", "path", "sha256"]
                or _sha256_bytes(archive.read(entry["path"])) != entry["sha256"]
            ):
                _raise("private_package_invalid", "private case digest differs")
        return tuple(cases)


def _public_case_ids(task: str) -> tuple[str, ...]:
    manifest = _load_object(FIXTURE_MANIFEST, "public_oracle_invalid")
    result: list[str] = []
    for entry in manifest.get("fixtures", ()):
        case = fixture_tool._load_json(ROOT / entry["path"])
        if task == "UC-001" and not case["case_id"].startswith("uc001-"):
            continue
        result.append(case["case_id"])
    if not result:
        _raise("public_oracle_invalid", f"{task}: no public cases")
    return tuple(result)


def _private_case_records(
    task: str, cases: tuple[dict[str, Any], ...]
) -> tuple[tuple[str, str], ...]:
    contract = _load_object(HIDDEN_CONTRACT, "private_package_invalid")
    applicable = {
        entry["id"]
        for entry in contract["behavior_categories"]
        if task in entry["use_cases"]
    }
    return tuple(
        (entry["case_id"], entry["behavior_category"])
        for entry in cases
        if entry["behavior_category"] in applicable
    )


def _seed_ids(task: str) -> tuple[str, ...]:
    contract = _load_object(HIDDEN_CONTRACT, "seeded_oracle_invalid")
    accepted_tasks = {task, "UC-001_OR_UC-003"}
    return tuple(
        entry["id"]
        for entry in contract["seed_categories"]
        if entry["task"] in accepted_tasks
    )


def _validate_observation(
    observation: OracleObservation,
    task: str,
    private_cases: tuple[dict[str, Any], ...],
) -> None:
    expected_public = _public_case_ids(task)
    actual_public = tuple(case_id for case_id, _ in observation.public_cases)
    if actual_public != expected_public or any(
        status != "passed" for _, status in observation.public_cases
    ):
        _raise("public_correctness_failed", "public case coverage or result differs")

    expected_private = _private_case_records(task, private_cases)
    actual_private = tuple(
        (case_id, category)
        for case_id, category, _ in observation.private_cases
    )
    if actual_private != expected_private or any(
        status != "passed" for _, _, status in observation.private_cases
    ):
        _raise("private_correctness_failed", "private case coverage or result differs")

    expected_seeds = _seed_ids(task)
    actual_seeds = tuple(seed_id for seed_id, _ in observation.seeded_roles)
    if actual_seeds != expected_seeds or any(
        status != "passed" for _, status in observation.seeded_roles
    ):
        _raise("seeded_regression", "seeded-role coverage or result differs")
    if len(observation.functional_output_sha256) != 64:
        _raise("incomplete_evidence", "functional observation digest is invalid")


def _completion_evidence(
    *,
    prepared: agent_runner.PreparedTrial,
    revision_sha256: str,
    archive_sha256: str,
    private_package_sha256: str,
    observation: OracleObservation,
    replay_sha256: str,
) -> dict[str, Any]:
    return {
        "completion_evidence_format": 1,
        "configuration_id": (
            f"{prepared.language}-{prepared.task.lower().replace('-', '')}"
        ),
        "revision_sha256": revision_sha256,
        "archive_sha256": archive_sha256,
        "task_start_tree_sha256": prepared.task_start_tree_sha256,
        "private_package_sha256": private_package_sha256,
        "checks": {
            "public": "passed",
            "private": "passed",
            "seeded_consumers": "passed",
            "protected_artifacts": "passed",
            "permissions": "passed",
            "replay": "passed",
        },
        "functional_output_sha256": observation.functional_output_sha256,
        "replay_sha256": replay_sha256,
    }


def verify_retained_revision(
    prepared: agent_runner.PreparedTrial,
    archive_path: Path,
    revision_sha256: str,
    private_package: Path,
    oracle: Oracle,
    *,
    permission_violations: int,
    external_access_attempts: int,
    expected_private_sha256: str | None = None,
    claimed_completion_revision: str | None = None,
) -> CorrectnessResult:
    """Run the complete oracle twice from one retained revision and classify it."""

    statuses = {
        "revision_sha256": revision_sha256,
        "public": "not_run",
        "private": "not_run",
        "seeded_consumers": "not_run",
        "completion_evidence": "not_run",
    }
    expected_private = expected_private_sha256
    if expected_private is None:
        expected_private = _load_object(
            PARITY_REPORT, "correctness_configuration_invalid"
        )["hidden_package_sha256"]
    try:
        if permission_violations or external_access_attempts:
            _raise(
                "permission_violation",
                "recorded permission or external-access violation",
            )
        private_cases = _read_private_manifest(private_package, expected_private)
        if (
            claimed_completion_revision is not None
            and claimed_completion_revision != revision_sha256
        ):
            _raise("revision_mismatch", "claimed completion revision is stale")
        with tempfile.TemporaryDirectory(prefix="ail-m8d-correctness-") as raw:
            root = Path(raw)
            first_workspace = root / "first"
            manifest = _materialize_final_revision(
                archive_path, first_workspace, revision_sha256
            )
            _check_final_boundary(prepared, manifest)
            first = oracle(
                first_workspace,
                private_package,
                prepared.language,
                prepared.task,
            )
            _validate_observation(first, prepared.task, private_cases)
            statuses.update(
                {
                    "public": "passed",
                    "private": "passed",
                    "seeded_consumers": "passed",
                }
            )

            replay_workspace = root / "replay"
            replay_manifest = _materialize_final_revision(
                archive_path, replay_workspace, revision_sha256
            )
            _check_final_boundary(prepared, replay_manifest)
            replay = oracle(
                replay_workspace,
                private_package,
                prepared.language,
                prepared.task,
            )
            _validate_observation(replay, prepared.task, private_cases)
            first_payload = _canonical_payload(first.__dict__)
            replay_payload = _canonical_payload(replay.__dict__)
            if first_payload != replay_payload:
                _raise("replay_mismatch", "functional replay result differs")
            replay_sha256 = _sha256_bytes(replay_payload)
            evidence = _completion_evidence(
                prepared=prepared,
                revision_sha256=revision_sha256,
                archive_sha256=_sha256(archive_path),
                private_package_sha256=expected_private,
                observation=first,
                replay_sha256=replay_sha256,
            )
            statuses["completion_evidence"] = "passed"
            return CorrectnessResult(
                terminal_class="successful",
                terminal_cause="complete",
                correctness=statuses,
                protected_artifacts_match="yes",
                replay_sha256=replay_sha256,
                completion_evidence=evidence,
            )
    except CorrectnessError as error:
        protected = "no" if error.code == "protected_artifact_changed" else "yes"
        return CorrectnessResult(
            terminal_class="failed",
            terminal_cause=error.code,
            correctness=statuses,
            protected_artifacts_match=protected,
            replay_sha256=None,
            completion_evidence=None,
        )


def _write_private_package(path: Path) -> tuple[str, tuple[dict[str, Any], ...]]:
    contract_sha256 = _sha256(HIDDEN_CONTRACT)
    cases = (
        {
            "case_id": "dry-validation",
            "behavior_category": "HIDDEN.VALIDATION_COMBINATIONS",
            "path": "cases/01.json",
            "sha256": _sha256_bytes(b"{}\n"),
        },
        {
            "case_id": "dry-payload",
            "behavior_category": "HIDDEN.PAYLOAD_BYTES",
            "path": "cases/02.json",
            "sha256": _sha256_bytes(b"{}\n"),
        },
        {
            "case_id": "dry-store",
            "behavior_category": "HIDDEN.STORE_POSTCONDITIONS",
            "path": "cases/03.json",
            "sha256": _sha256_bytes(b"{}\n"),
        },
        {
            "case_id": "dry-version",
            "behavior_category": "HIDDEN.VERSION_COMPATIBILITY",
            "path": "cases/04.json",
            "sha256": _sha256_bytes(b"{}\n"),
        },
        {
            "case_id": "dry-zero-effect",
            "behavior_category": "HIDDEN.ZERO_EFFECT_FAILURES",
            "path": "cases/05.json",
            "sha256": _sha256_bytes(b"{}\n"),
        },
    )
    manifest = (
        json.dumps(
            {
                "hidden_package_format": 1,
                "hidden_contract_sha256": contract_sha256,
                "cases": list(cases),
            },
            ensure_ascii=False,
            indent=2,
        )
        + "\n"
    ).encode("utf-8")
    contents = {"hidden-manifest.json": manifest}
    contents.update({entry["path"]: b"{}\n" for entry in cases})
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_STORED) as archive:
        for name, value in sorted(contents.items()):
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            archive.writestr(info, value)
    return _sha256(path), cases


def _rewrite_archive(
    source: Path,
    destination: Path,
    *,
    drop: str | None = None,
    replace: tuple[str, bytes] | None = None,
    add: tuple[str, bytes] | None = None,
) -> str:
    with zipfile.ZipFile(source) as archive:
        contents = {
            entry.filename: archive.read(entry.filename)
            for entry in archive.infolist()
            if entry.filename != FINAL_MANIFEST and entry.filename != drop
        }
    if replace is not None:
        contents[replace[0]] = replace[1]
    if add is not None:
        contents[add[0]] = add[1]
    records = [
        {"path": path, "sha256": _sha256_bytes(value)}
        for path, value in sorted(contents.items())
    ]
    tree_sha256 = task_starts._tree_digest(records)
    manifest = (
        json.dumps(
            {"tree_sha256": tree_sha256, "files": records},
            ensure_ascii=False,
            indent=2,
        )
        + "\n"
    ).encode("utf-8")
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_STORED) as output:
        for path, value in [*sorted(contents.items()), (FINAL_MANIFEST, manifest)]:
            info = zipfile.ZipInfo(path, date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            output.writestr(info, value)
    return tree_sha256


def verify_fake_and_dry_correctness() -> list[tuple[str, str]]:
    """Prove the M8d acceptance and rejection matrix without an agent call."""

    outcomes: list[tuple[str, str]] = []
    with tempfile.TemporaryDirectory(prefix="ail-m8d-dry-") as raw:
        root = Path(raw)
        prepared = agent_runner.prepare_trial(
            "python", "UC-001", root / "workspace"
        )
        final_archive = root / "final.zip"
        revision, _ = agent_runner.capture_final_source(
            prepared.workspace, final_archive
        )
        private_package = root / "private.zip"
        private_digest, private_cases = _write_private_package(private_package)

        def observation(seed_status: str = "passed") -> OracleObservation:
            public = tuple((case_id, "passed") for case_id in _public_case_ids("UC-001"))
            private = tuple(
                (case_id, category, "passed")
                for case_id, category in _private_case_records(
                    "UC-001", private_cases
                )
            )
            seeds = tuple(
                (seed_id, seed_status) for seed_id in _seed_ids("UC-001")
            )
            digest = _sha256_bytes(
                _canonical_payload(
                    {"public": public, "private": private, "seeds": seeds}
                )
            )
            return OracleObservation(public, private, seeds, digest)

        def passing_oracle(
            workspace: Path,
            private: Path,
            language: str,
            task: str,
        ) -> OracleObservation:
            del workspace, private, language, task
            return observation()

        clean = verify_retained_revision(
            prepared,
            final_archive,
            revision,
            private_package,
            passing_oracle,
            permission_violations=0,
            external_access_attempts=0,
            expected_private_sha256=private_digest,
            claimed_completion_revision=revision,
        )
        if (
            clean.terminal_class != "successful"
            or clean.completion_evidence is None
            or clean.completion_evidence["revision_sha256"] != revision
        ):
            _raise("correctness_self_test_failed", "clean replay did not pass")
        outcomes.append(("clean", clean.terminal_cause))

        editable = prepared.permission_profile["editable_files"][0]
        incomplete_archive = root / "incomplete.zip"
        incomplete_revision = _rewrite_archive(
            final_archive, incomplete_archive, drop=editable
        )
        incomplete = verify_retained_revision(
            prepared,
            incomplete_archive,
            incomplete_revision,
            private_package,
            passing_oracle,
            permission_violations=0,
            external_access_attempts=0,
            expected_private_sha256=private_digest,
        )
        outcomes.append(("incomplete", incomplete.terminal_cause))

        exposed_archive = root / "exposed.zip"
        exposed_revision = _rewrite_archive(
            final_archive,
            exposed_archive,
            add=("reference-answer.txt", b"not agent-visible\n"),
        )
        exposed = verify_retained_revision(
            prepared,
            exposed_archive,
            exposed_revision,
            private_package,
            passing_oracle,
            permission_violations=0,
            external_access_attempts=0,
            expected_private_sha256=private_digest,
        )
        outcomes.append(("answer-exposure", exposed.terminal_cause))

        protected_archive = root / "protected.zip"
        protected_revision = _rewrite_archive(
            final_archive,
            protected_archive,
            replace=("TASK.md", b"changed\n"),
        )
        protected = verify_retained_revision(
            prepared,
            protected_archive,
            protected_revision,
            private_package,
            passing_oracle,
            permission_violations=0,
            external_access_attempts=0,
            expected_private_sha256=private_digest,
        )
        outcomes.append(("protected-artifact", protected.terminal_cause))

        permission = verify_retained_revision(
            prepared,
            final_archive,
            revision,
            private_package,
            passing_oracle,
            permission_violations=1,
            external_access_attempts=0,
            expected_private_sha256=private_digest,
        )
        outcomes.append(("permission", permission.terminal_cause))

        stale = verify_retained_revision(
            prepared,
            final_archive,
            revision,
            private_package,
            passing_oracle,
            permission_violations=0,
            external_access_attempts=0,
            expected_private_sha256=private_digest,
            claimed_completion_revision="0" * 64,
        )
        outcomes.append(("revision-mismatch", stale.terminal_cause))

        def seeded_regression(
            workspace: Path,
            private: Path,
            language: str,
            task: str,
        ) -> OracleObservation:
            del workspace, private, language, task
            return observation("failed")

        seeded = verify_retained_revision(
            prepared,
            final_archive,
            revision,
            private_package,
            seeded_regression,
            permission_violations=0,
            external_access_attempts=0,
            expected_private_sha256=private_digest,
        )
        outcomes.append(("seeded-regression", seeded.terminal_cause))

        calls = 0

        def divergent_replay(
            workspace: Path,
            private: Path,
            language: str,
            task: str,
        ) -> OracleObservation:
            nonlocal calls
            del workspace, private, language, task
            calls += 1
            base = observation()
            return OracleObservation(
                base.public_cases,
                base.private_cases,
                base.seeded_roles,
                f"{calls:064x}",
            )

        divergent = verify_retained_revision(
            prepared,
            final_archive,
            revision,
            private_package,
            divergent_replay,
            permission_violations=0,
            external_access_attempts=0,
            expected_private_sha256=private_digest,
        )
        outcomes.append(("replay-mismatch", divergent.terminal_cause))

    expected = [
        ("clean", "complete"),
        ("incomplete", "incomplete_final_revision"),
        ("answer-exposure", "answer_exposure"),
        ("protected-artifact", "protected_artifact_changed"),
        ("permission", "permission_violation"),
        ("revision-mismatch", "revision_mismatch"),
        ("seeded-regression", "seeded_regression"),
        ("replay-mismatch", "replay_mismatch"),
    ]
    if outcomes != expected:
        _raise(
            "correctness_self_test_failed",
            f"expected {expected}, received {outcomes}",
        )
    return outcomes


def main() -> int:
    outcomes = verify_fake_and_dry_correctness()
    print(
        "M8d correctness verifier passed: "
        f"{len(outcomes)} stable fake/dry outcomes."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
