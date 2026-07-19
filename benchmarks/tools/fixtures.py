#!/usr/bin/env python3
"""Validate, format, and lock the public job-service fixture corpus."""

from __future__ import annotations

import argparse
import base64
import binascii
import copy
import hashlib
import json
import re
import sys
import unicodedata
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, NoReturn


ROOT = Path(__file__).resolve().parents[2]
FIXTURE_ROOT = ROOT / "benchmarks" / "fixtures" / "public"
MANIFEST_PATH = ROOT / "benchmarks" / "fixtures" / "manifest.json"
SCHEMA_PATH = ROOT / "benchmarks" / "schemas" / "job-service-fixture.schema.json"
PRIORITIES = ("low", "normal", "high")
JOB_ID = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$")

UC001_COVERAGE = frozenset(
    {
        "created_empty_payload",
        "created_non_ascii_task",
        "job_id_min_length",
        "job_id_max_length",
        "job_id_missing",
        "job_id_too_long",
        "job_id_invalid_start",
        "job_id_invalid_character",
        "task_min_length",
        "task_max_length",
        "task_missing",
        "task_too_long",
        "task_control_character",
        "payload_empty",
        "payload_max",
        "payload_too_large",
        "multiple_invalid_order",
        "duplicate",
        "unavailable",
    }
)
UC003_COVERAGE = frozenset(
    {
        "v2_priority_low",
        "v2_priority_normal",
        "v2_priority_high",
        "v2_priority_missing",
        "v2_priority_unknown",
        "v1_request_adapter",
        "v1_response_projection",
        "v1_stored_adapter",
        "duplicate_v1_request",
        "duplicate_v2_request",
        "unavailable_v1_request",
        "unavailable_v2_request",
    }
)

KEY_ORDER = {
    name: index
    for index, name in enumerate(
        (
            "fixture_format",
            "case_id",
            "service_version",
            "operation",
            "request",
            "stored_job",
            "api_version",
            "record_version",
            "kind",
            "job_id",
            "task",
            "payload_base64",
            "priority",
            "initial_jobs",
            "store_outcome",
            "expected",
            "response",
            "decode_error",
            "decoded_job",
            "result",
            "job",
            "issues",
            "code",
            "field",
            "reason",
            "value",
            "final_jobs",
            "store_calls",
        )
    )
}


@dataclass(frozen=True)
class Problem:
    path: str
    message: str

    def __str__(self) -> str:
        return f"{self.path}: {self.message}"


class FixtureError(Exception):
    """A fixture or manifest failed a deterministic validation rule."""


def _fail(path: str, message: str) -> NoReturn:
    raise FixtureError(str(Problem(path, message)))


def _relative(path: Path) -> str:
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def _load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except UnicodeDecodeError as error:
        raise FixtureError(f"{_relative(path)}: not UTF-8: {error}") from error
    except json.JSONDecodeError as error:
        raise FixtureError(
            f"{_relative(path)}:{error.lineno}:{error.colno}: invalid JSON: "
            f"{error.msg}"
        ) from error


def canonical_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _resolve_ref(root: dict[str, Any], reference: str) -> dict[str, Any]:
    if not reference.startswith("#/"):
        raise FixtureError(f"schema: unsupported non-local $ref {reference!r}")
    value: Any = root
    for part in reference[2:].split("/"):
        value = value[part.replace("~1", "/").replace("~0", "~")]
    if not isinstance(value, dict):
        raise FixtureError(f"schema: $ref does not resolve to an object: {reference}")
    return value


def _schema_errors(
    value: Any,
    schema: dict[str, Any],
    root: dict[str, Any],
    path: str,
) -> list[Problem]:
    if "$ref" in schema:
        return _schema_errors(value, _resolve_ref(root, schema["$ref"]), root, path)

    errors: list[Problem] = []
    if "allOf" in schema:
        for subschema in schema["allOf"]:
            errors.extend(_schema_errors(value, subschema, root, path))
    if "oneOf" in schema:
        branches = [
            _schema_errors(value, subschema, root, path)
            for subschema in schema["oneOf"]
        ]
        matches = sum(not branch for branch in branches)
        if matches != 1:
            errors.append(
                Problem(path, f"must match exactly one schema branch; matched {matches}")
            )
        return errors

    if "const" in schema and value != schema["const"]:
        errors.append(Problem(path, f"must equal {schema['const']!r}"))
    if "enum" in schema and value not in schema["enum"]:
        errors.append(Problem(path, f"must be one of {schema['enum']!r}"))

    expected_type = schema.get("type")
    type_ok = {
        "object": isinstance(value, dict),
        "array": isinstance(value, list),
        "string": isinstance(value, str),
        "integer": isinstance(value, int) and not isinstance(value, bool),
    }.get(expected_type, True)
    if expected_type and not type_ok:
        return errors + [Problem(path, f"must be a JSON {expected_type}")]

    if isinstance(value, dict):
        required = schema.get("required", ())
        for name in required:
            if name not in value:
                errors.append(Problem(path, f"missing required property {name!r}"))
        properties = schema.get("properties", {})
        if schema.get("additionalProperties") is False:
            for name in value:
                if name not in properties:
                    errors.append(Problem(path, f"unexpected property {name!r}"))
        for name, child in value.items():
            if name in properties:
                errors.extend(
                    _schema_errors(child, properties[name], root, f"{path}.{name}")
                )

    if isinstance(value, list):
        if len(value) < schema.get("minItems", 0):
            errors.append(Problem(path, "has too few items"))
        if "maxItems" in schema and len(value) > schema["maxItems"]:
            errors.append(Problem(path, "has too many items"))
        if "items" in schema:
            for index, child in enumerate(value):
                errors.extend(
                    _schema_errors(child, schema["items"], root, f"{path}[{index}]")
                )

    if isinstance(value, str) and "pattern" in schema:
        if re.fullmatch(schema["pattern"], value) is None:
            errors.append(Problem(path, f"does not match {schema['pattern']!r}"))
    return errors


def _assert_key_order(value: Any, path: str = "$") -> None:
    if isinstance(value, dict):
        keys = list(value)
        expected = sorted(keys, key=lambda key: (KEY_ORDER.get(key, 10_000), key))
        if keys != expected:
            _fail(path, f"properties are not in canonical order: expected {expected}")
        for key, child in value.items():
            _assert_key_order(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _assert_key_order(child, f"{path}[{index}]")


def _decode_payload(text: str, path: str) -> bytes:
    try:
        payload = base64.b64decode(text, validate=True)
    except (binascii.Error, ValueError) as error:
        _fail(path, f"must be standard padded Base64: {error}")
    if base64.b64encode(payload).decode("ascii") != text:
        _fail(path, "must use canonical standard padded Base64")
    return payload


def _validate_stored_job(job: dict[str, Any], path: str) -> None:
    _decode_payload(job["payload_base64"], f"{path}.payload_base64")
    version = job["record_version"]
    has_priority = "priority" in job
    if version == 1 and has_priority:
        _fail(path, "record version 1 must not contain priority")
    if version == 2 and not has_priority:
        _fail(path, "record version 2 must contain priority")


def _validation_issues(request: dict[str, Any]) -> list[dict[str, str]]:
    issues: list[dict[str, str]] = []
    job_id = request["job_id"]
    if job_id == "":
        issues.append({"field": "job_id", "reason": "missing"})
    elif JOB_ID.fullmatch(job_id) is None:
        issues.append({"field": "job_id", "reason": "invalid_format"})

    task = request["task"]
    if task == "":
        issues.append({"field": "task", "reason": "missing"})
    elif len(task) > 80:
        issues.append({"field": "task", "reason": "too_long"})
    elif any(unicodedata.category(character) == "Cc" for character in task):
        issues.append({"field": "task", "reason": "control_character"})

    payload = _decode_payload(request["payload_base64"], "$.request.payload_base64")
    if len(payload) > 4096:
        issues.append({"field": "payload", "reason": "payload_too_large"})

    if request["api_version"] == 2 and "priority" not in request:
        issues.append({"field": "priority", "reason": "missing"})
    return issues


def _response_job(stored_job: dict[str, Any], api_version: int) -> dict[str, Any]:
    response = {
        "job_id": stored_job["job_id"],
        "task": stored_job["task"],
        "payload_base64": stored_job["payload_base64"],
    }
    if api_version == 2:
        response["priority"] = stored_job["priority"]
    return response


def _oracle_create(case: dict[str, Any]) -> dict[str, Any]:
    request = case["request"]
    service_version = case["service_version"]
    api_version = request["api_version"]
    initial_jobs = copy.deepcopy(case["initial_jobs"])

    if service_version == 1 and api_version != 1:
        _fail("$.request.api_version", "service version 1 accepts only API version 1")
    if service_version == 2 and api_version not in (1, 2):
        _fail("$.request.api_version", "service version 2 accepts API versions 1 or 2")
    if api_version == 1 and "priority" in request:
        _fail("$.request", "API version 1 must not contain priority")
    if api_version == 2 and "priority" in request:
        priority = request["priority"]
        if priority not in PRIORITIES:
            return {
                "decode_error": {
                    "code": "unknown_priority",
                    "field": "priority",
                    "value": priority,
                },
                "final_jobs": initial_jobs,
                "store_calls": [],
            }

    issues = _validation_issues(request)
    if issues:
        if "store_outcome" in case:
            _fail("$", "a no-call validation case must omit store_outcome")
        return {
            "response": {
                "api_version": api_version,
                "result": {"kind": "invalid", "issues": issues},
            },
            "final_jobs": initial_jobs,
            "store_calls": [],
        }

    if "store_outcome" not in case:
        _fail("$", "a valid request must provide store_outcome")
    record_version = 1 if service_version == 1 else 2
    stored_job = {
        "record_version": record_version,
        "job_id": request["job_id"],
        "task": request["task"],
        "payload_base64": request["payload_base64"],
    }
    if record_version == 2:
        stored_job["priority"] = (
            request["priority"] if api_version == 2 else "normal"
        )
    call = {"operation": "insert_if_absent", "job": stored_job}
    outcome = case["store_outcome"]

    if outcome == "inserted":
        if any(job["job_id"] == request["job_id"] for job in initial_jobs):
            _fail("$", "inserted outcome conflicts with a pre-existing job_id")
        final_jobs = initial_jobs + [stored_job]
        result = {
            "kind": "created",
            "job": _response_job(stored_job, api_version),
        }
    elif outcome == "duplicate":
        if not any(job["job_id"] == request["job_id"] for job in initial_jobs):
            _fail("$", "duplicate outcome requires a pre-existing matching job_id")
        final_jobs = initial_jobs
        result = {"kind": "already_exists", "job_id": request["job_id"]}
    elif outcome == "unavailable_before_commit":
        final_jobs = initial_jobs
        result = {"kind": "persistence_unavailable"}
    else:
        _fail("$.store_outcome", f"unsupported store outcome {outcome!r}")

    return {
        "response": {"api_version": api_version, "result": result},
        "final_jobs": final_jobs,
        "store_calls": [call],
    }


def _oracle_decode(case: dict[str, Any]) -> dict[str, Any]:
    stored_job = case["stored_job"]
    if case["service_version"] != 2 or stored_job["record_version"] != 1:
        _fail("$", "decode_stored_job requires service version 2 and record version 1")
    decoded = {
        "record_version": 2,
        "job_id": stored_job["job_id"],
        "task": stored_job["task"],
        "payload_base64": stored_job["payload_base64"],
        "priority": "normal",
    }
    return {"decoded_job": decoded}


def validate_fixture(path: Path, schema: dict[str, Any]) -> dict[str, Any]:
    case = _load_json(path)
    if not isinstance(case, dict):
        raise FixtureError(f"{_relative(path)}: fixture must be a JSON object")

    relative = _relative(path)
    schema_errors = _schema_errors(case, schema, schema, "$")
    if schema_errors:
        raise FixtureError(
            "\n".join(f"{relative}: {problem}" for problem in schema_errors)
        )
    _assert_key_order(case)
    if path.read_text(encoding="utf-8") != canonical_json(case):
        raise FixtureError(f"{relative}: file is not canonical two-space JSON")
    if path.stem != case["case_id"]:
        raise FixtureError(f"{relative}: filename must equal case_id")

    if case["operation"] == "create_job":
        for index, job in enumerate(case["initial_jobs"]):
            _validate_stored_job(job, f"$.initial_jobs[{index}]")
        expected = _oracle_create(case)
    elif case["operation"] == "decode_stored_job":
        _validate_stored_job(case["stored_job"], "$.stored_job")
        expected = _oracle_decode(case)
    else:
        raise FixtureError(f"{relative}: unsupported operation")

    if case["expected"] != expected:
        raise FixtureError(
            f"{relative}: expected oracle does not match accepted behavior\n"
            f"expected:\n{canonical_json(expected)}"
            f"actual:\n{canonical_json(case['expected'])}"
        )
    return case


def fixture_paths() -> list[Path]:
    return sorted(FIXTURE_ROOT.rglob("*.json"))


def _validate_manifest_shape(manifest: Any) -> dict[str, Any]:
    if not isinstance(manifest, dict):
        _fail("manifest", "must be an object")
    if list(manifest) != ["manifest_format", "schema", "fixtures"]:
        _fail("manifest", "properties must be manifest_format, schema, fixtures")
    if manifest["manifest_format"] != 1:
        _fail("manifest.manifest_format", "must equal 1")
    schema = manifest["schema"]
    if not isinstance(schema, dict) or list(schema) != ["path", "sha256"]:
        _fail("manifest.schema", "must contain path and sha256")
    entries = manifest["fixtures"]
    if not isinstance(entries, list):
        _fail("manifest.fixtures", "must be an array")
    return manifest


def _check_traceability(entries: list[dict[str, Any]]) -> None:
    seen_cases: set[str] = set()
    uc001: set[str] = set()
    uc003: set[str] = set()
    regressions: set[str] = set()
    for index, entry in enumerate(entries):
        path = f"manifest.fixtures[{index}]"
        expected_keys = [
            "path",
            "sha256",
            "case_id",
            "operation",
            "use_cases",
            "requirements",
            "coverage",
        ]
        if not isinstance(entry, dict) or list(entry) != expected_keys:
            _fail(path, f"must contain properties in this order: {expected_keys}")
        case_id = entry["case_id"]
        if case_id in seen_cases:
            _fail(path, f"duplicate case_id {case_id!r}")
        seen_cases.add(case_id)
        use_cases = entry["use_cases"]
        requirements = entry["requirements"]
        coverage = entry["coverage"]
        if not use_cases or any(item not in ("UC-001", "UC-003") for item in use_cases):
            _fail(path, "use_cases must contain UC-001 and/or UC-003")
        if not requirements or any(
            not re.fullmatch(r"(APP|NFR)-\d{3}", item) for item in requirements
        ):
            _fail(path, "requirements must contain APP-* or NFR-* identifiers")
        if not isinstance(coverage, list) or not coverage:
            _fail(path, "coverage must be a non-empty array")
        for tag in coverage:
            if tag.startswith("uc001:"):
                uc001.add(tag.removeprefix("uc001:"))
            elif tag.startswith("uc003:"):
                uc003.add(tag.removeprefix("uc003:"))
            elif tag.startswith("regression:"):
                regressions.add(tag.removeprefix("regression:"))
            else:
                _fail(path, f"unknown coverage tag {tag!r}")

    missing_uc001 = sorted(UC001_COVERAGE - uc001)
    missing_uc003 = sorted(UC003_COVERAGE - uc003)
    missing_regressions = sorted(UC001_COVERAGE - regressions)
    if missing_uc001:
        _fail("manifest", f"missing UC-001 coverage: {missing_uc001}")
    if missing_uc003:
        _fail("manifest", f"missing UC-003 coverage: {missing_uc003}")
    if missing_regressions:
        _fail("manifest", f"missing UC-001-on-V2 regressions: {missing_regressions}")


def check_manifest(cases: dict[str, dict[str, Any]]) -> None:
    manifest = _validate_manifest_shape(_load_json(MANIFEST_PATH))
    if MANIFEST_PATH.read_text(encoding="utf-8") != canonical_json(manifest):
        _fail(_relative(MANIFEST_PATH), "file is not canonical two-space JSON")
    schema_entry = manifest["schema"]
    if schema_entry["path"] != _relative(SCHEMA_PATH):
        _fail("manifest.schema.path", f"must equal {_relative(SCHEMA_PATH)!r}")
    if schema_entry["sha256"] != _sha256(SCHEMA_PATH):
        _fail("manifest.schema.sha256", "does not match schema content")

    entries = manifest["fixtures"]
    _check_traceability(entries)
    listed_paths = [entry["path"] for entry in entries]
    actual_paths = sorted(cases)
    if listed_paths != actual_paths:
        _fail("manifest.fixtures", "paths do not exactly match sorted public fixtures")
    for entry in entries:
        path = ROOT / entry["path"]
        case = cases[entry["path"]]
        if entry["sha256"] != _sha256(path):
            _fail(entry["path"], "manifest SHA-256 does not match content")
        if entry["case_id"] != case["case_id"]:
            _fail(entry["path"], "manifest case_id does not match fixture")
        if entry["operation"] != case["operation"]:
            _fail(entry["path"], "manifest operation does not match fixture")


def check_all() -> None:
    schema = _load_json(SCHEMA_PATH)
    if SCHEMA_PATH.read_text(encoding="utf-8") != canonical_json(schema):
        _fail(_relative(SCHEMA_PATH), "schema is not canonical two-space JSON")
    paths = fixture_paths()
    if not paths:
        _fail(_relative(FIXTURE_ROOT), "contains no public fixtures")
    cases = {_relative(path): validate_fixture(path, schema) for path in paths}
    check_manifest(cases)
    print(
        f"Fixture check passed: {len(cases)} public cases, schema valid, "
        "oracles match, coverage complete, manifest locked."
    )


def format_paths(paths: Iterable[Path]) -> None:
    paths = list(paths)
    changed = 0
    for path in paths:
        value = _load_json(path)
        text = canonical_json(value)
        if path.read_text(encoding="utf-8") != text:
            path.write_text(text, encoding="utf-8", newline="\n")
            changed += 1
    print(f"Formatted {changed} of {len(paths)} JSON files.")


def write_manifest() -> None:
    if not MANIFEST_PATH.exists():
        _fail(_relative(MANIFEST_PATH), "create traceability entries before writing")
    manifest = _validate_manifest_shape(_load_json(MANIFEST_PATH))
    entries_by_path = {entry["path"]: entry for entry in manifest["fixtures"]}
    paths = fixture_paths()
    actual = {_relative(path) for path in paths}
    if set(entries_by_path) != actual:
        _fail(
            "manifest.fixtures",
            "traceability entries must exactly match fixtures before digest rewrite",
        )
    manifest["schema"]["sha256"] = _sha256(SCHEMA_PATH)
    manifest["fixtures"] = [
        {
            **entries_by_path[_relative(path)],
            "sha256": _sha256(path),
        }
        for path in paths
    ]
    MANIFEST_PATH.write_text(
        canonical_json(manifest), encoding="utf-8", newline="\n"
    )
    print(f"Wrote {_relative(MANIFEST_PATH)} with {len(paths)} fixture digests.")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("check", help="run the complete public fixture gate")
    format_parser = subparsers.add_parser("format", help="format public fixtures")
    format_parser.add_argument("paths", nargs="*", type=Path)
    manifest_parser = subparsers.add_parser("manifest", help="manage fixture digests")
    mode = manifest_parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--write", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        if args.command == "check":
            check_all()
        elif args.command == "format":
            paths = args.paths or fixture_paths()
            format_paths(paths)
        elif args.write:
            write_manifest()
        else:
            schema = _load_json(SCHEMA_PATH)
            cases = {
                _relative(path): validate_fixture(path, schema)
                for path in fixture_paths()
            }
            check_manifest(cases)
            print(f"Manifest check passed: {len(cases)} fixture digests match.")
    except (FixtureError, KeyError, TypeError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
