"""Adapters between frozen JSON fixtures and the typed service boundary."""

from __future__ import annotations

import base64
import binascii
from collections.abc import Mapping
from dataclasses import dataclass
from typing import Any, Never, cast

from v2.domain import (
    AlreadyExists,
    ApiVersion,
    Created,
    CreateJobRequest,
    CreateJobResult,
    InsertOutcome,
    Invalid,
    Job,
    PersistenceUnavailable,
    Priority,
)
from v2.service import create_job
from v2.store import DeterministicJobStore, RecordVersion, StoredJob

type JsonObject = dict[str, Any]


class FixtureError(ValueError):
    """A malformed or unsupported fixture boundary."""


@dataclass(frozen=True, slots=True)
class DecodedRequest:
    version: ApiVersion
    request: CreateJobRequest


def run_case(raw: object) -> JsonObject:
    fixture = _mapping(raw, "fixture")
    operation = _string(fixture, "operation")
    if operation == "create_job":
        return _run_create_case(fixture)
    if operation == "decode_stored_job":
        return _run_decode_stored_case(fixture)
    raise FixtureError(f"unsupported operation {operation!r}")


def _run_create_case(fixture: Mapping[str, object]) -> JsonObject:
    case_id = _string(fixture, "case_id")
    service_version = _record_version(_integer(fixture, "service_version"))
    initial_jobs = tuple(
        _parse_stored_job(value) for value in _list(fixture, "initial_jobs")
    )
    decoded, unknown_priority = _decode_request(
        service_version,
        _mapping(fixture.get("request"), "request"),
    )

    if unknown_priority is not None:
        actual: JsonObject = {
            "decode_error": {
                "code": "unknown_priority",
                "field": "priority",
                "value": unknown_priority,
            },
            "final_jobs": _encode_stored_jobs(initial_jobs),
            "store_calls": [],
        }
    else:
        outcome = _parse_store_outcome(fixture.get("store_outcome"))
        job_store = DeterministicJobStore(initial_jobs, outcome, service_version)
        result = create_job(decoded.request, job_store)
        actual = {
            "response": _encode_response(decoded.version, result),
            "final_jobs": _encode_stored_jobs(job_store.jobs),
            "store_calls": [
                {
                    "operation": "insert_if_absent",
                    "job": _encode_stored_job(call.job),
                }
                for call in job_store.calls
            ],
        }
    return {
        "result_format": 1,
        "case_id": case_id,
        "operation": "create_job",
        "actual": actual,
    }


def _run_decode_stored_case(fixture: Mapping[str, object]) -> JsonObject:
    stored = _parse_stored_job(_mapping(fixture.get("stored_job"), "stored_job"))
    return {
        "result_format": 1,
        "case_id": _string(fixture, "case_id"),
        "operation": "decode_stored_job",
        "actual": {"decoded_job": _encode_job_v2(stored.adapt_to_v2())},
    }


def _decode_request(
    service_version: RecordVersion,
    raw: Mapping[str, object],
) -> tuple[DecodedRequest, str | None]:
    try:
        version = ApiVersion(_integer(raw, "api_version"))
    except ValueError as error:
        raise FixtureError("unsupported request API version") from error
    if service_version is RecordVersion.V1 and version is not ApiVersion.V1:
        raise FixtureError("service version 1 accepts only API version 1")

    unknown_priority: str | None = None
    if version is ApiVersion.V1:
        priority: Priority | None = Priority.NORMAL
    elif "priority" not in raw:
        priority = None
    else:
        value = _string(raw, "priority")
        try:
            priority = Priority(value)
        except ValueError:
            priority = None
            unknown_priority = value

    request = CreateJobRequest(
        job_id=_string(raw, "job_id"),
        task=_string(raw, "task"),
        payload=_decode_base64(_string(raw, "payload_base64"), "payload"),
        priority=priority,
    )
    return DecodedRequest(version, request), unknown_priority


def _parse_stored_job(raw: object) -> StoredJob:
    value = _mapping(raw, "stored job")
    version = _record_version(_integer(value, "record_version"))
    priority: Priority | None = None
    if version is RecordVersion.V2:
        if "priority" not in value:
            raise FixtureError("version-two stored job is missing priority")
        raw_priority = _string(value, "priority")
        try:
            priority = Priority(raw_priority)
        except ValueError as error:
            raise FixtureError(
                f"version-two stored job has unknown priority {raw_priority!r}"
            ) from error
    return StoredJob(
        record_version=version,
        job_id=_string(value, "job_id"),
        task=_string(value, "task"),
        payload=_decode_base64(
            _string(value, "payload_base64"),
            "stored payload",
        ),
        priority=priority,
    )


def _parse_store_outcome(value: object) -> InsertOutcome:
    if value is None:
        return InsertOutcome.UNAVAILABLE_BEFORE_COMMIT
    if not isinstance(value, str):
        raise FixtureError("store_outcome must be a string")
    try:
        return InsertOutcome(value)
    except ValueError as error:
        raise FixtureError(f"unsupported store outcome {value!r}") from error


def _record_version(value: int) -> RecordVersion:
    try:
        return RecordVersion(value)
    except ValueError as error:
        raise FixtureError(f"unsupported service or record version {value}") from error


def _encode_response(
    version: ApiVersion,
    result: CreateJobResult,
) -> JsonObject:
    match result:
        case Created(job):
            encoded: JsonObject = {
                "kind": "created",
                "job": (
                    {
                        "job_id": job.job_id,
                        "task": job.task,
                        "payload_base64": _encode_base64(job.payload),
                    }
                    if version is ApiVersion.V1
                    else {
                        "job_id": job.job_id,
                        "task": job.task,
                        "payload_base64": _encode_base64(job.payload),
                        "priority": job.priority.value,
                    }
                ),
            }
        case Invalid(issues):
            encoded = {
                "kind": "invalid",
                "issues": [
                    {"field": issue.field.value, "reason": issue.reason.value}
                    for issue in issues
                ],
            }
        case AlreadyExists(job_id):
            encoded = {"kind": "already_exists", "job_id": job_id}
        case PersistenceUnavailable():
            encoded = {"kind": "persistence_unavailable"}
        case _:
            _assert_never(result)
    return {"api_version": int(version), "result": encoded}


def _encode_job_v2(job: Job) -> JsonObject:
    return {
        "record_version": 2,
        "job_id": job.job_id,
        "task": job.task,
        "payload_base64": _encode_base64(job.payload),
        "priority": job.priority.value,
    }


def _encode_stored_jobs(jobs: tuple[StoredJob, ...]) -> list[JsonObject]:
    return [_encode_stored_job(job) for job in jobs]


def _encode_stored_job(job: StoredJob) -> JsonObject:
    result: JsonObject = {
        "record_version": int(job.record_version),
        "job_id": job.job_id,
        "task": job.task,
        "payload_base64": _encode_base64(job.payload),
    }
    if job.record_version is RecordVersion.V2:
        if job.priority is None:
            msg = "version-two stored job must have priority"
            raise AssertionError(msg)
        result["priority"] = job.priority.value
    return result


def _encode_base64(value: bytes) -> str:
    return base64.b64encode(value).decode("ascii")


def _decode_base64(value: str, label: str) -> bytes:
    try:
        return base64.b64decode(value, validate=True)
    except (binascii.Error, ValueError) as error:
        raise FixtureError(f"invalid {label} Base64") from error


def _mapping(value: object, label: str) -> Mapping[str, object]:
    if not isinstance(value, Mapping):
        raise FixtureError(f"{label} must be an object")
    return cast("Mapping[str, object]", value)


def _list(value: Mapping[str, object], key: str) -> list[object]:
    result = value.get(key)
    if not isinstance(result, list):
        raise FixtureError(f"{key} must be an array")
    return cast("list[object]", result)


def _string(value: Mapping[str, object], key: str) -> str:
    result = value.get(key)
    if not isinstance(result, str):
        raise FixtureError(f"{key} must be a string")
    return result


def _integer(value: Mapping[str, object], key: str) -> int:
    result = value.get(key)
    if not isinstance(result, int) or isinstance(result, bool):
        raise FixtureError(f"{key} must be an integer")
    return result


def _assert_never(value: Never) -> Never:
    raise AssertionError(f"unhandled create-job result {value!r}")
