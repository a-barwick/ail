#!/usr/bin/env python3
"""Verify benchmark runners and the frozen M2 task contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import zipfile
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterator, NoReturn

import fixtures as fixture_tool
import task_starts as task_start_tool
import calibration as calibration_tool


ROOT = Path(__file__).resolve().parents[2]
SCHEMA_ROOT = ROOT / "benchmarks" / "schemas"
RESULT_SCHEMA = SCHEMA_ROOT / "runner-result.schema.json"
DESCRIPTOR_SCHEMA = SCHEMA_ROOT / "runner-descriptor.schema.json"
RUN_MANIFEST_SCHEMA = SCHEMA_ROOT / "run-manifest.schema.json"
CONTRACT_LOCK = ROOT / "benchmarks" / "contracts" / "contract-lock.json"
HIDDEN_CONTRACT = ROOT / "benchmarks" / "contracts" / "hidden-contract.json"
M7_FREEZE = ROOT / "benchmarks" / "m7-freeze.json"
M7_PARITY_REPORT = ROOT / "benchmarks" / "m7-parity-report.json"
BASELINE_LANGUAGES = ("rust", "go", "python", "typescript")
HIDDEN_PACKAGE_ENVIRONMENT = "AIL_HIDDEN_PACKAGE"
HIDDEN_RUNTIME_ROOT = ROOT / "benchmarks" / "tmp"
REQUIRED_RUN_ARTIFACTS = (
    "benchmarks/contracts/contract-lock.json",
    "benchmarks/schemas/run-manifest.schema.json",
    "benchmarks/schemas/runner-result.schema.json",
    "benchmarks/tools/harness.py",
)
TOKEN_CATEGORIES = (
    "initial_context",
    "source_reads",
    "semantic_tool_output",
    "diagnostics",
    "build_and_test_output",
    "other_tool_output",
)
CONTRACT_ARTIFACTS = (
    "benchmarks/contracts/hidden-contract.json",
    "benchmarks/contracts/run-classification.md",
    "benchmarks/contracts/runner-contract.md",
    "benchmarks/fixtures/manifest.json",
    "benchmarks/schemas/job-service-fixture.schema.json",
    "benchmarks/schemas/run-manifest.schema.json",
    "benchmarks/schemas/runner-descriptor.schema.json",
    "benchmarks/schemas/runner-result.schema.json",
    "benchmarks/tasks/uc001-implement-create-job.md",
    "benchmarks/tasks/uc003-add-priority.md",
    "benchmarks/tests/support/fake-runner.json",
    "benchmarks/tests/support/fake_runner.py",
    "benchmarks/tools/harness.py",
)


@dataclass(frozen=True)
class HarnessError(Exception):
    code: str
    message: str

    def __str__(self) -> str:
        return f"{self.code}: {self.message}"


@dataclass(frozen=True)
class HiddenCase:
    """One private fixture, represented without exposing its content in reports."""

    case: dict[str, Any]
    behavior_category: str
    sha256: str
    path: str


@dataclass(frozen=True)
class Verification:
    """Normalized observations from one baseline's public and hidden cases."""

    language: str
    public: tuple[dict[str, Any], ...]
    hidden: tuple[tuple[HiddenCase, dict[str, Any]], ...]


def _raise(code: str, message: str) -> NoReturn:
    raise HarnessError(code, message)


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _canonical(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def _load_object(path: Path, code: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        _raise(code, f"{path}: {error}")
    if not isinstance(value, dict):
        _raise(code, f"{path}: must contain a JSON object")
    return value


def _validate_schema(
    value: dict[str, Any],
    schema_path: Path,
    code: str,
) -> None:
    schema = fixture_tool._load_json(schema_path)
    errors = fixture_tool._schema_errors(value, schema, schema, "$")
    if errors:
        rendered = "; ".join(str(problem) for problem in errors[:8])
        _raise(code, rendered)


def _require_canonical(path: Path, value: dict[str, Any], code: str) -> None:
    if path.read_text(encoding="utf-8") != _canonical(value):
        _raise(code, f"{path}: must be canonical two-space JSON")


def _repo_path(raw_path: str, code: str) -> Path:
    path = (ROOT / raw_path).resolve()
    try:
        path.relative_to(ROOT)
    except ValueError:
        _raise(code, f"artifact path leaves repository: {raw_path!r}")
    return path


def validate_descriptor(path: Path) -> dict[str, Any]:
    descriptor = _load_object(path, "runner_descriptor_invalid")
    _validate_schema(descriptor, DESCRIPTOR_SCHEMA, "runner_descriptor_invalid")
    _require_canonical(path, descriptor, "runner_descriptor_invalid")
    if not descriptor["command"] or any(not part for part in descriptor["command"]):
        _raise("runner_descriptor_invalid", "command entries must be non-empty")
    working_directory = _repo_path(
        descriptor["working_directory"], "runner_descriptor_invalid"
    )
    if not working_directory.is_dir():
        _raise("runner_descriptor_invalid", "working_directory does not exist")
    for name, value in descriptor["environment"].items():
        if not isinstance(name, str) or not isinstance(value, str):
            _raise("runner_descriptor_invalid", "environment must map strings")
    return descriptor


def _resolve_manifest_from_lock(
    lock_path: Path, lock: dict[str, Any]
) -> Path:
    raw = lock["manifest_path"]
    path = Path(raw)
    if not path.is_absolute():
        path = lock_path.parent / path
    return path.resolve()


def _required_artifact(
    manifest: dict[str, Any],
    path: str,
    expected_digest: str,
) -> None:
    if path.startswith("external:"):
        return
    locks = {entry["path"]: entry["sha256"] for entry in manifest["artifact_locks"]}
    if locks.get(path) != expected_digest:
        _raise(
            "run_manifest_invalid",
            f"{path!r} and its declared digest must appear in artifact_locks",
        )


def load_locked_manifest(
    manifest_path: Path,
    lock_path: Path,
) -> dict[str, Any]:
    lock = _load_object(lock_path, "manifest_lock_invalid")
    expected_keys = ["lock_format", "manifest_path", "manifest_sha256"]
    if list(lock) != expected_keys or lock.get("lock_format") != 1:
        _raise(
            "manifest_lock_invalid",
            f"lock properties must be {expected_keys} and lock_format must be 1",
        )
    _require_canonical(lock_path, lock, "manifest_lock_invalid")
    locked_path = _resolve_manifest_from_lock(lock_path, lock)
    if locked_path != manifest_path.resolve():
        _raise("manifest_lock_invalid", "lock names a different run manifest")
    if not manifest_path.is_file():
        _raise("manifest_lock_invalid", "run manifest does not exist")
    if _sha256(manifest_path) != lock["manifest_sha256"]:
        _raise("manifest_changed", "run manifest digest differs from external lock")

    manifest = _load_object(manifest_path, "run_manifest_invalid")
    _validate_schema(manifest, RUN_MANIFEST_SCHEMA, "run_manifest_invalid")
    _require_canonical(manifest_path, manifest, "run_manifest_invalid")

    if any(
        not value
        for value in (
            manifest["configuration_id"],
            manifest["source"]["repository"],
            manifest["source"]["start_revision"],
            manifest["source"]["dependency_revision"],
            manifest["task"]["path"],
            manifest["model"]["provider"],
            manifest["model"]["name"],
            manifest["model"]["version"],
            manifest["model"]["agent"],
            manifest["environment"]["container_image"],
            manifest["environment"]["reference_host"],
        )
    ):
        _raise("run_manifest_invalid", "required identity strings must be non-empty")

    limits = manifest["limits"]
    if any(not isinstance(value, int) or value <= 0 for value in limits.values()):
        _raise("run_manifest_invalid", "all limits must be positive integers")
    retry = manifest["retry_policy"]
    if retry["max_attempts"] <= 0:
        _raise("run_manifest_invalid", "max_attempts must be positive")
    if manifest["token_accounting"]["categories"] != list(TOKEN_CATEGORIES):
        _raise(
            "run_manifest_invalid",
            "token categories must contain the complete frozen ordered set",
        )
    if not manifest["correctness"]["required_checks"]:
        _raise("run_manifest_invalid", "required_checks must not be empty")

    artifact_locks = manifest["artifact_locks"]
    paths = [entry["path"] for entry in artifact_locks]
    if paths != sorted(paths) or len(paths) != len(set(paths)):
        _raise(
            "run_manifest_invalid",
            "artifact_locks must have unique paths in lexical order",
        )
    for entry in artifact_locks:
        path = _repo_path(entry["path"], "run_manifest_invalid")
        if not path.is_file():
            _raise("locked_artifact_changed", f"missing artifact {entry['path']}")
        if _sha256(path) != entry["sha256"]:
            _raise(
                "locked_artifact_changed",
                f"digest changed for {entry['path']}",
            )

    task = manifest["task"]
    runner = manifest["runner"]
    _required_artifact(
        manifest, runner["descriptor_path"], runner["descriptor_sha256"]
    )
    _required_artifact(manifest, task["path"], task["sha256"])
    tests = manifest["tests"]
    _required_artifact(
        manifest,
        tests["public_fixture_manifest"],
        tests["public_fixture_manifest_sha256"],
    )
    _required_artifact(
        manifest,
        tests["hidden_package"],
        tests["hidden_package_sha256"],
    )
    for path in REQUIRED_RUN_ARTIFACTS:
        _required_artifact(manifest, path, _sha256(ROOT / path))
    return manifest


def _checkpoint_tree_digest(files: list[str]) -> str:
    digest = hashlib.sha256()
    for raw_path in files:
        path = _repo_path(raw_path, "checkpoint_invalid")
        if not path.is_file():
            _raise("checkpoint_invalid", f"missing checkpoint artifact {raw_path}")
        digest.update(f"{_sha256(path)}  {raw_path}\n".encode("utf-8"))
    return digest.hexdigest()


def check_checkpoint_lock(language: str, manifest: dict[str, Any]) -> None:
    """Prove that both task checkpoints still name their exact source trees."""

    path = ROOT / "benchmarks" / "baselines" / language / "checkpoints.json"
    value = _load_object(path, "checkpoint_invalid")
    _require_canonical(path, value, "checkpoint_invalid")
    expected_keys = [
        "checkpoint_format",
        "tree_digest_algorithm",
        "checkpoints",
    ]
    if list(value) != expected_keys or value["checkpoint_format"] != 1:
        _raise("checkpoint_invalid", "unexpected checkpoint document shape")
    expected_algorithm = (
        "sha256 of concatenated '<file-sha256>  <repository-relative-path>\\n' "
        "records in listed order"
    )
    if value["tree_digest_algorithm"] != expected_algorithm:
        _raise("checkpoint_invalid", "unexpected checkpoint digest algorithm")
    checkpoints = value["checkpoints"]
    expected_ids = [f"{language}-v1", f"{language}-v2"]
    if not isinstance(checkpoints, list) or [item.get("id") for item in checkpoints] != expected_ids:
        _raise("checkpoint_invalid", "must contain ordered V1 and V2 checkpoints")
    for checkpoint in checkpoints:
        files = checkpoint.get("files")
        if not isinstance(files, list) or not files:
            _raise("checkpoint_invalid", f"{checkpoint['id']}: files must be present")
        if len(files) != len(set(files)):
            _raise("checkpoint_invalid", f"{checkpoint['id']}: files must be unique")
        if _checkpoint_tree_digest(files) != checkpoint.get("source_tree_sha256"):
            _raise("checkpoint_changed", f"{checkpoint['id']}: source tree digest differs")
    if checkpoints[0]["source_tree_sha256"] != manifest["source"]["source_tree_sha256"]:
        _raise("checkpoint_invalid", "manifest V1 source digest differs from checkpoint")
    if not manifest["source"]["start_revision"].endswith(
        checkpoints[0]["source_tree_sha256"][:16]
    ):
        _raise("checkpoint_invalid", "manifest start revision does not name V1 digest")


def check_seed_locations(language: str) -> None:
    """Require every frozen seed category to have a stable baseline location."""

    path = ROOT / "benchmarks" / "baselines" / language / "seed-locations.json"
    value = _load_object(path, "seed_locations_invalid")
    _require_canonical(path, value, "seed_locations_invalid")
    if list(value) != ["seed_location_format", "language", "locations"]:
        _raise("seed_locations_invalid", "unexpected seed location document shape")
    if value["seed_location_format"] != 1 or value["language"] != language:
        _raise("seed_locations_invalid", "seed location language or format differs")
    contract = _load_object(HIDDEN_CONTRACT, "hidden_contract_invalid")
    seed_ids = [entry["id"] for entry in contract["seed_categories"]]
    locations = value["locations"]
    if not isinstance(locations, list) or [entry.get("seed_id") for entry in locations] != seed_ids:
        _raise("seed_locations_invalid", "seed categories must match the frozen order")
    for location in locations:
        semantic_locations = location.get("semantic_locations")
        if (
            not isinstance(semantic_locations, list)
            or not semantic_locations
            or any(not isinstance(item, str) or not item for item in semantic_locations)
        ):
            _raise(
                "seed_locations_invalid",
                f"{location.get('seed_id')}: semantic locations are incomplete",
            )


def _decode_result(stdout: str) -> dict[str, Any]:
    try:
        value = json.loads(stdout)
    except json.JSONDecodeError as error:
        _raise(
            "malformed_result",
            f"stdout is not exactly one JSON value at {error.lineno}:{error.colno}",
        )
    if not isinstance(value, dict):
        _raise("result_schema_invalid", "runner result must be an object")
    _validate_schema(value, RESULT_SCHEMA, "result_schema_invalid")
    return value


def _invoke(
    descriptor: dict[str, Any],
    arguments: list[str],
    timeout_seconds: int,
    test_environment: dict[str, str] | None = None,
) -> dict[str, Any]:
    environment = {
        "PATH": os.environ.get("PATH", ""),
        "LANG": "C",
        "LC_ALL": "C",
        "PYTHONHASHSEED": "0",
    }
    environment.update(descriptor["environment"])
    if test_environment:
        environment.update(test_environment)
    try:
        completed = subprocess.run(
            descriptor["command"] + arguments,
            cwd=_repo_path(
                descriptor["working_directory"], "runner_descriptor_invalid"
            ),
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            timeout=timeout_seconds,
            check=False,
        )
    except subprocess.TimeoutExpired:
        _raise("timeout", f"runner exceeded {timeout_seconds} seconds")
    except OSError as error:
        _raise("runner_start_failed", str(error))
    if completed.returncode != 0:
        _raise(
            "nonzero_exit",
            f"runner exited {completed.returncode}: {completed.stderr.strip()}",
        )
    return _decode_result(completed.stdout)


def _load_public_cases(manifest_path: Path) -> list[dict[str, Any]]:
    fixture_manifest = _load_object(manifest_path, "fixture_manifest_invalid")
    entries = fixture_manifest.get("fixtures")
    if not isinstance(entries, list):
        _raise("fixture_manifest_invalid", "fixtures must be an array")
    return [
        fixture_tool._load_json(_repo_path(entry["path"], "fixture_manifest_invalid"))
        for entry in entries
    ]


def _compare_actual(case: dict[str, Any], result: dict[str, Any]) -> None:
    if result["case_id"] != case["case_id"]:
        _raise(
            "unexpected_case",
            f"expected {case['case_id']!r}, got {result['case_id']!r}",
        )
    if result["operation"] != case["operation"]:
        _raise(
            "response_mismatch",
            f"{case['case_id']}: operation does not match",
        )
    actual = result["actual"]
    expected = case["expected"]
    primary_keys = ("response", "decode_error", "decoded_job")
    expected_primary = {key: expected[key] for key in primary_keys if key in expected}
    actual_primary = {key: actual[key] for key in primary_keys if key in actual}
    if actual_primary != expected_primary:
        _raise("response_mismatch", f"{case['case_id']}: boundary result differs")
    if actual.get("final_jobs") != expected.get("final_jobs"):
        _raise("final_state_mismatch", f"{case['case_id']}: final jobs differ")
    if actual.get("store_calls") != expected.get("store_calls"):
        _raise("store_calls_mismatch", f"{case['case_id']}: store calls differ")
    if actual != expected:
        _raise("response_mismatch", f"{case['case_id']}: actual result differs")


def verify_case(
    descriptor: dict[str, Any],
    manifest: dict[str, Any],
    case_path: str,
    test_environment: dict[str, str] | None = None,
) -> dict[str, Any]:
    case = fixture_tool._load_json(_repo_path(case_path, "fixture_invalid"))
    result = _invoke(
        descriptor,
        ["--case", case_path],
        manifest["limits"]["functional_corpus_seconds"],
        test_environment,
    )
    if "case_id" not in result:
        _raise("result_schema_invalid", "one-case command returned corpus result")
    _compare_actual(case, result)
    return result


def verify_corpus(
    descriptor: dict[str, Any],
    manifest: dict[str, Any],
    test_environment: dict[str, str] | None = None,
) -> list[dict[str, Any]]:
    manifest_path_text = manifest["tests"]["public_fixture_manifest"]
    manifest_path = _repo_path(manifest_path_text, "fixture_manifest_invalid")
    cases = _load_public_cases(manifest_path)
    result = _invoke(
        descriptor,
        ["--corpus", manifest_path_text],
        manifest["limits"]["functional_corpus_seconds"],
        test_environment,
    )
    if "results" not in result:
        _raise("result_schema_invalid", "corpus command returned one-case result")
    if result["fixture_manifest_sha256"] != _sha256(manifest_path):
        _raise("manifest_mismatch", "runner consumed a different fixture manifest")
    actual_results = result["results"]
    expected_ids = [case["case_id"] for case in cases]
    actual_ids = [item["case_id"] for item in actual_results]
    missing = [case_id for case_id in expected_ids if case_id not in actual_ids]
    if missing:
        _raise("missing_case", f"runner omitted {missing[0]!r}")
    unexpected = [case_id for case_id in actual_ids if case_id not in expected_ids]
    if unexpected:
        _raise("unexpected_case", f"runner returned {unexpected[0]!r}")
    if actual_ids != expected_ids:
        _raise("unexpected_case", "runner result order differs from manifest")
    for case, actual in zip(cases, actual_results):
        _compare_actual(case, actual)
    return actual_results


def _hidden_package_path(
    manifest: dict[str, Any], hidden_package: Path | None
) -> Path:
    reference = manifest["tests"]["hidden_package"]
    if not reference.startswith("external:"):
        _raise("hidden_package_invalid", "M7 requires an external hidden package")
    candidate = hidden_package
    if candidate is None:
        configured = os.environ.get(HIDDEN_PACKAGE_ENVIRONMENT)
        candidate = Path(configured) if configured else None
    if candidate is None:
        _raise(
            "hidden_package_missing",
            f"set {HIDDEN_PACKAGE_ENVIRONMENT} to the digest-locked private ZIP",
        )
    if not candidate.is_file():
        _raise("hidden_package_missing", "configured private ZIP does not exist")
    if _sha256(candidate) != manifest["tests"]["hidden_package_sha256"]:
        _raise("hidden_package_changed", "private ZIP digest differs from manifest")
    return candidate


def _hidden_categories() -> list[str]:
    contract = _load_object(HIDDEN_CONTRACT, "hidden_contract_invalid")
    return [entry["id"] for entry in contract["behavior_categories"]]


@contextmanager
def _private_hidden_cases(package_path: Path) -> Iterator[tuple[HiddenCase, ...]]:
    """Validate and materialize a private ZIP only for the runner's lifetime."""

    try:
        archive = zipfile.ZipFile(package_path)
    except (OSError, zipfile.BadZipFile) as error:
        _raise("hidden_package_invalid", f"cannot read private ZIP: {error}")
    with archive:
        entries = archive.infolist()
        names = [entry.filename for entry in entries]
        if not names or names != sorted(names) or len(names) != len(set(names)):
            _raise("hidden_package_invalid", "ZIP entries must be unique lexical order")
        for entry in entries:
            if (
                entry.compress_type != zipfile.ZIP_STORED
                or entry.date_time != (1980, 1, 1, 0, 0, 0)
                or entry.is_dir()
            ):
                _raise("hidden_package_invalid", "ZIP entries must be stored and timestamped")
        if "hidden-manifest.json" not in names:
            _raise("hidden_package_invalid", "ZIP omits hidden-manifest.json")
        try:
            hidden_manifest = json.loads(
                archive.read("hidden-manifest.json").decode("utf-8")
            )
        except (KeyError, UnicodeDecodeError, json.JSONDecodeError) as error:
            _raise("hidden_package_invalid", f"invalid hidden manifest: {error}")
        if not isinstance(hidden_manifest, dict) or list(hidden_manifest) != [
            "hidden_package_format",
            "hidden_contract_sha256",
            "cases",
        ]:
            _raise("hidden_package_invalid", "unexpected hidden manifest shape")
        if (
            hidden_manifest["hidden_package_format"] != 1
            or hidden_manifest["hidden_contract_sha256"] != _sha256(HIDDEN_CONTRACT)
            or _canonical(hidden_manifest)
            != archive.read("hidden-manifest.json").decode("utf-8")
        ):
            _raise("hidden_package_invalid", "hidden manifest is not locked canonical data")
        cases = hidden_manifest["cases"]
        if not isinstance(cases, list) or not cases:
            _raise("hidden_package_invalid", "hidden package has no cases")
        required_categories = _hidden_categories()
        if [entry.get("behavior_category") for entry in cases] != required_categories:
            _raise("hidden_package_invalid", "hidden behavior categories differ from contract")
        paths = [entry.get("path") for entry in cases]
        if (
            any(not isinstance(path, str) or not path.startswith("cases/") for path in paths)
            or paths != sorted(paths)
            or len(paths) != len(set(paths))
            or set(names) != {"hidden-manifest.json", *paths}
        ):
            _raise("hidden_package_invalid", "hidden case entry paths are invalid")
        HIDDEN_RUNTIME_ROOT.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(
            prefix="m7-hidden-", dir=HIDDEN_RUNTIME_ROOT
        ) as raw_directory:
            directory = Path(raw_directory)
            result: list[HiddenCase] = []
            schema = fixture_tool._load_json(SCHEMA_ROOT / "job-service-fixture.schema.json")
            for entry in cases:
                if list(entry) != ["case_id", "behavior_category", "path", "sha256"]:
                    _raise("hidden_package_invalid", "hidden case manifest entry has extra data")
                contents = archive.read(entry["path"])
                if hashlib.sha256(contents).hexdigest() != entry["sha256"]:
                    _raise("hidden_package_changed", "hidden case digest differs")
                destination = directory / entry["path"]
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(contents)
                try:
                    case = fixture_tool.validate_fixture(destination, schema)
                except fixture_tool.FixtureError as error:
                    _raise("hidden_package_invalid", str(error))
                if case["case_id"] != entry["case_id"]:
                    _raise("hidden_package_invalid", "hidden manifest case id differs")
                result.append(
                    HiddenCase(
                        case=case,
                        behavior_category=entry["behavior_category"],
                        sha256=entry["sha256"],
                        path=destination.relative_to(ROOT).as_posix(),
                    )
                )
            yield tuple(result)


def verify_from_paths(
    descriptor_path: Path,
    manifest_path: Path,
    lock_path: Path,
    visibility: str,
    case_path: str | None = None,
    test_environment: dict[str, str] | None = None,
    hidden_package: Path | None = None,
) -> Verification:
    manifest = load_locked_manifest(manifest_path, lock_path)
    descriptor = validate_descriptor(descriptor_path)
    runner = manifest["runner"]
    expected_descriptor = _repo_path(
        runner["descriptor_path"], "run_manifest_invalid"
    )
    if descriptor_path.resolve() != expected_descriptor:
        _raise("run_manifest_invalid", "selected runner differs from locked runner")
    if descriptor["language"] != manifest["source"]["language"]:
        _raise("run_manifest_invalid", "runner language differs from source language")
    if descriptor["language"] in BASELINE_LANGUAGES:
        check_checkpoint_lock(descriptor["language"], manifest)
        check_seed_locations(descriptor["language"])
    public_results: tuple[dict[str, Any], ...] = ()
    hidden_results: tuple[tuple[HiddenCase, dict[str, Any]], ...] = ()
    if visibility in ("public", "all"):
        if case_path is None:
            public_results = tuple(verify_corpus(descriptor, manifest, test_environment))
        else:
            public_results = (
                verify_case(descriptor, manifest, case_path, test_environment),
            )
    if visibility in ("hidden", "all"):
        if case_path is not None:
            _raise("hidden_case_invalid", "--case cannot be combined with hidden visibility")
        package_path = _hidden_package_path(manifest, hidden_package)
        with _private_hidden_cases(package_path) as hidden_cases:
            hidden_results = tuple(
                (
                    hidden_case,
                    verify_case(
                        descriptor,
                        manifest,
                        hidden_case.path,
                        test_environment,
                    ),
                )
                for hidden_case in hidden_cases
            )
    return Verification(descriptor["language"], public_results, hidden_results)


def _contract_value() -> dict[str, Any]:
    return {
        "contract_lock_format": 1,
        "artifacts": [
            {"path": path, "sha256": _sha256(_repo_path(path, "contract_invalid"))}
            for path in CONTRACT_ARTIFACTS
        ],
    }


def write_contract_lock() -> None:
    CONTRACT_LOCK.write_text(_canonical(_contract_value()), encoding="utf-8")
    print(
        f"Wrote {CONTRACT_LOCK.relative_to(ROOT)} with "
        f"{len(CONTRACT_ARTIFACTS)} locked artifacts."
    )


def check_contract_lock() -> None:
    actual = _load_object(CONTRACT_LOCK, "contract_changed")
    _require_canonical(CONTRACT_LOCK, actual, "contract_changed")
    expected = _contract_value()
    if actual != expected:
        _raise("contract_changed", "frozen M2 contract artifact digest differs")
    for raw_path in CONTRACT_ARTIFACTS:
        path = ROOT / raw_path
        if path.suffix == ".json":
            value = _load_object(path, "contract_invalid")
            _require_canonical(path, value, "contract_invalid")
    check_hidden_contract()


def check_hidden_contract() -> None:
    contract = _load_object(HIDDEN_CONTRACT, "hidden_contract_invalid")
    _require_canonical(HIDDEN_CONTRACT, contract, "hidden_contract_invalid")
    if list(contract) != [
        "hidden_contract_format",
        "behavior_categories",
        "seed_categories",
        "packaging",
    ] or contract["hidden_contract_format"] != 1:
        _raise("hidden_contract_invalid", "unexpected top-level contract shape")
    behavior_ids = [entry.get("id") for entry in contract["behavior_categories"]]
    expected_behaviors = [
        "HIDDEN.VALIDATION_COMBINATIONS",
        "HIDDEN.PAYLOAD_BYTES",
        "HIDDEN.STORE_POSTCONDITIONS",
        "HIDDEN.VERSION_COMPATIBILITY",
        "HIDDEN.ZERO_EFFECT_FAILURES",
    ]
    if behavior_ids != expected_behaviors:
        _raise("hidden_contract_invalid", "behavior category set or order changed")
    seed_ids = [entry.get("id") for entry in contract["seed_categories"]]
    expected_seeds = [
        "SEED.PREVALIDATION_EFFECT",
        "SEED.REQUEST_CONSTRUCTOR",
        "SEED.REQUEST_ADAPTER",
        "SEED.HANDLER_PROPAGATION",
        "SEED.STORE_CONTRACT",
        "SEED.PERSISTED_CODEC",
        "SEED.RESPONSE_PROJECTION",
        "SEED.RESULT_CONSUMER",
        "SEED.AUTHORITY_EXPANSION",
        "SEED.STALE_EVIDENCE",
    ]
    if seed_ids != expected_seeds:
        _raise("hidden_contract_invalid", "seed category set or order changed")
    for entry in contract["behavior_categories"]:
        if (
            not entry.get("use_cases")
            or not entry.get("requirements")
            or not entry.get("rule")
        ):
            _raise("hidden_contract_invalid", f"incomplete {entry.get('id')}")
    for entry in contract["seed_categories"]:
        if (
            not entry.get("task")
            or not entry.get("semantic_role")
            or not entry.get("expected_failure")
        ):
            _raise("hidden_contract_invalid", f"incomplete {entry.get('id')}")
    packaging = contract["packaging"]
    if (
        packaging.get("package_format") != "permission-separated-zip-v1"
        or packaging.get("entry_digest") != "sha256"
        or packaging.get("entry_order") != "lexical"
        or packaging.get("compression") != "stored"
        or packaging.get("entry_timestamp") != "1980-01-01T00:00:00Z"
        or packaging.get("agent_visibility") != "denied"
        or packaging.get("runner_visibility") != "read-only"
        or len(packaging.get("rules", ())) != 6
    ):
        _raise("hidden_contract_invalid", "packaging boundary is incomplete")


def _self_test_manifest() -> dict[str, Any]:
    task_path = "benchmarks/tasks/uc001-implement-create-job.md"
    public_path = "benchmarks/fixtures/manifest.json"
    hidden_path = "benchmarks/contracts/hidden-contract.json"
    descriptor_path = "benchmarks/tests/support/fake-runner.json"
    artifacts = sorted(
        {
            task_path,
            public_path,
            hidden_path,
            descriptor_path,
            "benchmarks/tests/support/fake_runner.py",
            *REQUIRED_RUN_ARTIFACTS,
        }
    )
    return {
        "manifest_format": 1,
        "benchmark_id": "ail-job-service",
        "configuration_id": "m2-harness-self-test",
        "source": {
            "language": "self-test",
            "repository": "benchmarks/tests/support",
            "start_revision": "self-test-r1",
            "dependency_revision": "none",
            "source_tree_sha256": _sha256(
                ROOT / "benchmarks" / "tests" / "support" / "fake_runner.py"
            ),
        },
        "runner": {
            "descriptor_path": descriptor_path,
            "descriptor_sha256": _sha256(ROOT / descriptor_path),
        },
        "task": {
            "task_id": "UC-001",
            "path": task_path,
            "sha256": _sha256(ROOT / task_path),
            "initial_context": [task_path],
        },
        "tests": {
            "public_fixture_manifest": public_path,
            "public_fixture_manifest_sha256": _sha256(ROOT / public_path),
            "hidden_package": hidden_path,
            "hidden_package_sha256": _sha256(ROOT / hidden_path),
        },
        "tools": [
            {
                "name": "python",
                "version": f"{sys.version_info.major}.{sys.version_info.minor}",
                "command": [
                    "python3",
                    "benchmarks/tests/support/fake_runner.py",
                ],
            }
        ],
        "model": {
            "provider": "self-test",
            "name": "deterministic-fake",
            "version": "1",
            "agent": "harness-self-test",
            "sampling": {
                "parameters": {
                    "deterministic": True,
                },
                "unavailable": [],
            },
        },
        "environment": {
            "container_image": "self-test@sha256:" + "0" * 64,
            "reference_host": "self-test",
            "operating_system": sys.platform,
            "architecture": "self-test",
        },
        "permissions": {
            "network": "denied",
            "filesystem_read": ["benchmarks"],
            "filesystem_write": [],
            "subprocess": "allowlisted",
        },
        "limits": {
            "run_seconds": 30,
            "functional_corpus_seconds": 1,
            "input_tokens": 100000,
            "startup_seconds": 2,
            "peak_memory_mib": 512,
        },
        "retry_policy": {
            "max_attempts": 1,
            "retryable_failures": [],
        },
        "token_accounting": {
            "tokenizer": "self-test-tokenizer",
            "categories": list(TOKEN_CATEGORIES),
            "repeated_context": "count_every_delivery",
        },
        "repair_policy": {
            "definition_version": 1,
            "pre_edit_checks_count": False,
        },
        "correctness": {
            "required_visibility": "public",
            "required_checks": [
                "response",
                "final_jobs",
                "store_calls",
            ],
            "final_revision_evidence": True,
        },
        "outputs": {
            "events": "artifacts/events.jsonl",
            "results": "artifacts/results.json",
            "measurements": "artifacts/measurements.json",
        },
        "artifact_locks": [
            {"path": path, "sha256": _sha256(ROOT / path)} for path in artifacts
        ],
    }


def _write_self_test_lock(directory: Path, manifest: dict[str, Any]) -> tuple[Path, Path]:
    manifest_path = directory / "run-manifest.json"
    lock_path = directory / "run-manifest.lock.json"
    manifest_path.write_text(_canonical(manifest), encoding="utf-8")
    lock = {
        "lock_format": 1,
        "manifest_path": manifest_path.name,
        "manifest_sha256": _sha256(manifest_path),
    }
    lock_path.write_text(_canonical(lock), encoding="utf-8")
    return manifest_path, lock_path


def _expect(
    label: str,
    code: str | None,
    action: Any,
) -> None:
    try:
        action()
    except HarnessError as error:
        if code is None:
            raise
        if error.code != code:
            raise HarnessError(
                "self_test_failed",
                f"{label}: expected {code}, got {error.code}: {error.message}",
            ) from error
    else:
        if code is not None:
            _raise("self_test_failed", f"{label}: expected {code}, but passed")
    print(f"PASS {label}")


def self_test() -> None:
    check_contract_lock()
    print("PASS frozen-contract-lock")
    fixture_tool.check_all()
    descriptor = ROOT / "benchmarks" / "tests" / "support" / "fake-runner.json"
    case_path = (
        "benchmarks/fixtures/public/create_job/"
        "uc001-v1-created-empty-payload.json"
    )
    with tempfile.TemporaryDirectory(prefix="ail-harness-self-test-") as raw:
        directory = Path(raw)
        manifest_path, lock_path = _write_self_test_lock(
            directory, _self_test_manifest()
        )

        def run(mode: str, *, case: bool = False) -> None:
            verify_from_paths(
                descriptor,
                manifest_path,
                lock_path,
                "public",
                case_path if case else None,
                {"AIL_FAKE_MODE": mode},
            )

        _expect("passing-single-case", None, lambda: run("pass", case=True))
        _expect("passing-corpus", None, lambda: run("pass"))
        for mode, code in (
            ("response_mismatch", "response_mismatch"),
            ("final_state_mismatch", "final_state_mismatch"),
            ("store_calls_mismatch", "store_calls_mismatch"),
            ("missing_case", "missing_case"),
            ("unexpected_case", "unexpected_case"),
            ("manifest_mismatch", "manifest_mismatch"),
            ("result_schema_invalid", "result_schema_invalid"),
            ("malformed_result", "malformed_result"),
            ("nonzero_exit", "nonzero_exit"),
            ("timeout", "timeout"),
        ):
            _expect(mode.replace("_", "-"), code, lambda mode=mode: run(mode))

        marker = directory / "started"
        changed = _load_object(manifest_path, "self_test_failed")
        changed["configuration_id"] = "changed-after-lock"
        manifest_path.write_text(_canonical(changed), encoding="utf-8")
        _expect(
            "changed-manifest-prevents-start",
            "manifest_changed",
            lambda: verify_from_paths(
                descriptor,
                manifest_path,
                lock_path,
                "public",
                test_environment={"AIL_FAKE_MARKER": str(marker)},
            ),
        )
        if marker.exists():
            _raise(
                "self_test_failed",
                "runner started after changed-manifest rejection",
            )

        incomplete = _self_test_manifest()
        incomplete.pop("model")
        manifest_path, lock_path = _write_self_test_lock(directory, incomplete)
        _expect(
            "incomplete-manifest-prevents-start",
            "run_manifest_invalid",
            lambda: verify_from_paths(
                descriptor,
                manifest_path,
                lock_path,
                "public",
                test_environment={"AIL_FAKE_MARKER": str(marker)},
            ),
        )
        if marker.exists():
            _raise(
                "self_test_failed",
                "runner started after incomplete-manifest rejection",
            )

    print("Harness self-test passed: 14 expected outcomes verified.")


def _verification_paths(language: str) -> tuple[Path, Path, Path]:
    directory = ROOT / "benchmarks" / "baselines" / language
    return (
        directory / "runner.json",
        directory / "verification-manifest.json",
        directory / "verification-manifest.lock.json",
    )


def _m7_freeze_artifacts() -> list[str]:
    artifacts = [
        "benchmarks/contracts/contract-lock.json",
        "benchmarks/contracts/hidden-contract.json",
        "benchmarks/fixtures/manifest.json",
        "benchmarks/m7-parity-report.json",
        "benchmarks/schemas/run-manifest.schema.json",
        "benchmarks/tasks/uc001-implement-create-job.md",
        "benchmarks/tasks/uc003-add-priority.md",
        "benchmarks/tools/harness.py",
    ]
    artifacts.extend(task_start_tool.freeze_artifacts())
    for language in BASELINE_LANGUAGES:
        prefix = f"benchmarks/baselines/{language}"
        artifacts.extend(
            [
                f"{prefix}/checkpoints.json",
                f"{prefix}/runner.json",
                f"{prefix}/seed-locations.json",
                f"{prefix}/verification-manifest.json",
                f"{prefix}/verification-manifest.lock.json",
            ]
        )
    return sorted(artifacts)


def _m7_freeze_value() -> dict[str, Any]:
    return {
        "m7_freeze_format": 1,
        "artifacts": [
            {"path": path, "sha256": _sha256(_repo_path(path, "m7_freeze_invalid"))}
            for path in _m7_freeze_artifacts()
        ],
    }


def write_m7_freeze() -> None:
    M7_FREEZE.write_text(_canonical(_m7_freeze_value()), encoding="utf-8")
    print(f"Wrote {M7_FREEZE.relative_to(ROOT)} with {len(_m7_freeze_artifacts())} artifacts.")


def check_m7_freeze() -> None:
    actual = _load_object(M7_FREEZE, "m7_freeze_changed")
    _require_canonical(M7_FREEZE, actual, "m7_freeze_changed")
    if actual != _m7_freeze_value():
        _raise("m7_freeze_changed", "M7 benchmark input digest differs")


def _public_case_records() -> list[dict[str, Any]]:
    fixture_manifest = _load_object(
        ROOT / "benchmarks" / "fixtures" / "manifest.json", "fixture_manifest_invalid"
    )
    return [
        {
            "case_id": entry["case_id"],
            "visibility": "public",
        }
        for entry in fixture_manifest["fixtures"]
    ]


def _parity_report_value(
    hidden_cases: tuple[HiddenCase, ...],
    hidden_package_sha256: str,
) -> dict[str, Any]:
    language_records: list[dict[str, Any]] = []
    for language in BASELINE_LANGUAGES:
        _, manifest_path, _ = _verification_paths(language)
        manifest = _load_object(manifest_path, "run_manifest_invalid")
        checkpoints = _load_object(
            manifest_path.parent / "checkpoints.json", "checkpoint_invalid"
        )["checkpoints"]
        language_records.append(
            {
                "language": language,
                "v1_source_tree_sha256": checkpoints[0]["source_tree_sha256"],
                "v2_source_tree_sha256": checkpoints[1]["source_tree_sha256"],
                "tool_versions": [
                    {"name": tool["name"], "version": tool["version"]}
                    for tool in manifest["tools"]
                ],
            }
        )
    hidden_records = [
        {
            "behavior_category": hidden_case.behavior_category,
            "case_sha256": hidden_case.sha256,
            "visibility": "hidden",
        }
        for hidden_case in hidden_cases
    ]
    return {
        "parity_report_format": 1,
        "public_fixture_manifest_sha256": _sha256(
            ROOT / "benchmarks" / "fixtures" / "manifest.json"
        ),
        "hidden_package_sha256": hidden_package_sha256,
        "comparison": ["response", "final_jobs", "store_calls"],
        "languages": language_records,
        "cases": _public_case_records() + hidden_records,
    }


def write_parity_report(hidden_package: Path | None) -> None:
    package_path = _hidden_package_path(
        load_locked_manifest(*_verification_paths("rust")[1:]), hidden_package
    )
    with _private_hidden_cases(package_path) as hidden_cases:
        value = _parity_report_value(hidden_cases, _sha256(package_path))
    M7_PARITY_REPORT.write_text(_canonical(value), encoding="utf-8")
    print(f"Wrote {M7_PARITY_REPORT.relative_to(ROOT)} for {len(value['cases'])} cases.")


def check_parity_report(
    hidden_cases: tuple[HiddenCase, ...], hidden_package_sha256: str
) -> None:
    actual = _load_object(M7_PARITY_REPORT, "parity_report_changed")
    _require_canonical(M7_PARITY_REPORT, actual, "parity_report_changed")
    expected = _parity_report_value(hidden_cases, hidden_package_sha256)
    if actual != expected:
        _raise("parity_report_changed", "parity report does not match locked inputs")


def _compare_baseline_observations(verifications: tuple[Verification, ...]) -> None:
    """Compare every normalized observation against the first baseline."""

    reference = verifications[0]
    reference_public = [item["actual"] for item in reference.public]
    reference_hidden = [result["actual"] for _, result in reference.hidden]
    for verification in verifications[1:]:
        if [item["actual"] for item in verification.public] != reference_public:
            _raise(
                "cross_language_mismatch",
                f"{verification.language}: public normalized behavior differs",
            )
        if [result["actual"] for _, result in verification.hidden] != reference_hidden:
            _raise(
                "cross_language_mismatch",
                f"{verification.language}: hidden normalized behavior differs",
            )


def verify_all(hidden_package: Path | None = None) -> None:
    """Run the M7 parity gate across every baseline and both visibilities."""

    check_contract_lock()
    fixture_tool.check_all()
    check_m7_freeze()
    task_start_tool.check_task_start_lock()
    verifications: list[Verification] = []
    for language in BASELINE_LANGUAGES:
        descriptor, manifest, lock = _verification_paths(language)
        verifications.append(
            verify_from_paths(
                descriptor, manifest, lock, "all", hidden_package=hidden_package
            )
        )
    _compare_baseline_observations(tuple(verifications))
    reference = verifications[0]
    package_path = _hidden_package_path(
        load_locked_manifest(*_verification_paths(reference.language)[1:]),
        hidden_package,
    )
    check_parity_report(
        tuple(hidden_case for hidden_case, _ in reference.hidden), _sha256(package_path)
    )
    print(
        "M7 parity passed: "
        f"{len(BASELINE_LANGUAGES)} baselines, {len(reference.public)} public cases, "
        f"{len(reference.hidden)} hidden cases, response/final-state/store-call parity."
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("self-test", help="exercise every M2 harness outcome")
    calibration = subparsers.add_parser(
        "verify-calibration",
        help="verify the M8 evidence contract and calibration campaign",
    )
    calibration.add_argument(
        "--campaign",
        type=Path,
        help="evidence-index.json to validate; omit to run M8b synthetic fixtures",
    )
    calibration.add_argument(
        "--index-lock",
        type=Path,
        help="external evidence-index lock; defaults beside --campaign",
    )

    verify = subparsers.add_parser("verify", help="verify a baseline runner")
    verify.add_argument(
        "--language",
        required=True,
        choices=("rust", "go", "python", "typescript", "ail"),
    )
    verify.add_argument(
        "--visibility",
        required=True,
        choices=("public", "hidden", "all"),
    )
    verify.add_argument("--case")
    verify.add_argument(
        "--hidden-package",
        type=Path,
        help=f"private ZIP path; defaults to ${HIDDEN_PACKAGE_ENVIRONMENT}",
    )

    verify_all_parser = subparsers.add_parser(
        "verify-all", help="run the M7 cross-baseline public and hidden parity gate"
    )
    verify_all_parser.add_argument(
        "--hidden-package",
        type=Path,
        help=f"private ZIP path; defaults to ${HIDDEN_PACKAGE_ENVIRONMENT}",
    )

    parity_report = subparsers.add_parser(
        "parity-report", help="manage the deterministic M7 parity report"
    )
    parity_mode = parity_report.add_mutually_exclusive_group(required=True)
    parity_mode.add_argument("--write", action="store_true")
    parity_mode.add_argument("--check", action="store_true")
    parity_report.add_argument("--hidden-package", type=Path)

    freeze = subparsers.add_parser("freeze", help="manage the M7 benchmark input lock")
    freeze_mode = freeze.add_mutually_exclusive_group(required=True)
    freeze_mode.add_argument("--write", action="store_true")
    freeze_mode.add_argument("--check", action="store_true")

    contract = subparsers.add_parser("contract", help="manage the M2 digest lock")
    mode = contract.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--write", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        if args.command == "self-test":
            self_test()
        elif args.command == "verify-calibration":
            if args.index_lock is not None and args.campaign is None:
                _raise(
                    "evidence_index_lock_invalid",
                    "--index-lock requires --campaign",
                )
            if args.campaign is None:
                calibration_tool.verify_calibration()
            else:
                result = calibration_tool.verify_campaign(
                    args.campaign,
                    args.index_lock,
                )
                print(
                    "M8 calibration evidence passed: "
                    f"{result.campaign_id} is {result.completion_state}; "
                    f"{result.agent_trials} agent, "
                    f"{result.warm_measurements} warm, and "
                    f"{result.cold_measurements} cold records."
                )
        elif args.command == "verify":
            descriptor, manifest, lock = _verification_paths(args.language)
            verify_from_paths(
                descriptor,
                manifest,
                lock,
                args.visibility,
                args.case,
                hidden_package=args.hidden_package,
            )
            print(f"{args.language} {args.visibility} verification passed.")
        elif args.command == "verify-all":
            verify_all(args.hidden_package)
        elif args.command == "parity-report":
            if args.write:
                write_parity_report(args.hidden_package)
            else:
                package_path = _hidden_package_path(
                    load_locked_manifest(*_verification_paths("rust")[1:]),
                    args.hidden_package,
                )
                with _private_hidden_cases(package_path) as hidden_cases:
                    check_parity_report(hidden_cases, _sha256(package_path))
                print("M7 parity report matches the locked inputs.")
        elif args.command == "freeze":
            if args.write:
                write_m7_freeze()
            else:
                check_m7_freeze()
                print(f"M7 freeze lock passed: {len(_m7_freeze_artifacts())} artifacts match.")
        elif args.write:
            write_contract_lock()
        else:
            check_contract_lock()
            print(
                f"Contract lock passed: {len(CONTRACT_ARTIFACTS)} artifacts match."
            )
    except HarnessError as error:
        print(f"ERROR [{error.code}]: {error.message}", file=sys.stderr)
        return 1
    except calibration_tool.CalibrationError as error:
        print(f"ERROR [{error.code}]: {error.message}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
