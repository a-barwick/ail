from __future__ import annotations

import json
from pathlib import Path
from typing import Any, cast

import pytest

from v2.fixture import FixtureError, _encode_stored_job, run_case
from v2.store import RecordVersion, StoredJob


def load(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def test_every_public_fixture_matches_the_shared_oracle(
    repository_root: Path,
) -> None:
    manifest = cast(
        "dict[str, Any]",
        load(repository_root / "benchmarks/fixtures/manifest.json"),
    )
    fixtures = cast("list[dict[str, str]]", manifest["fixtures"])
    assert len(fixtures) == 37
    for entry in fixtures:
        fixture = cast("dict[str, Any]", load(repository_root / entry["path"]))
        result = run_case(fixture)
        assert result["case_id"] == fixture["case_id"]
        assert result["operation"] == fixture["operation"]
        assert result["actual"] == fixture["expected"], entry["path"]


def valid_fixture() -> dict[str, object]:
    return {
        "fixture_format": 1,
        "case_id": "test",
        "service_version": 2,
        "operation": "create_job",
        "request": {
            "api_version": 2,
            "job_id": "job-1",
            "task": "task",
            "payload_base64": "",
            "priority": "high",
        },
        "initial_jobs": [],
        "store_outcome": "inserted",
    }


@pytest.mark.parametrize(
    ("change", "message"),
    [
        ({"operation": "remove_job"}, "unsupported operation"),
        ({"service_version": 3}, "unsupported service or record version"),
        ({"store_outcome": "maybe"}, "unsupported store outcome"),
        ({"initial_jobs": "not-array"}, "must be an array"),
    ],
)
def test_malformed_or_unsupported_fixture_is_rejected(
    change: dict[str, object],
    message: str,
) -> None:
    fixture = valid_fixture()
    fixture.update(change)
    with pytest.raises(FixtureError, match=message):
        run_case(fixture)


def test_invalid_base64_is_rejected() -> None:
    fixture = valid_fixture()
    request = cast("dict[str, object]", fixture["request"])
    request["payload_base64"] = "***"
    with pytest.raises(FixtureError, match="Base64"):
        run_case(fixture)


def test_v1_service_rejects_v2_request() -> None:
    fixture = valid_fixture()
    fixture["service_version"] = 1
    with pytest.raises(FixtureError, match="accepts only API version 1"):
        run_case(fixture)


def test_unsupported_request_version_is_rejected() -> None:
    fixture = valid_fixture()
    request = cast("dict[str, object]", fixture["request"])
    request["api_version"] = 3
    with pytest.raises(FixtureError, match="unsupported request API version"):
        run_case(fixture)


@pytest.mark.parametrize(
    ("stored_job", "message"),
    [
        (
            {
                "record_version": 2,
                "job_id": "job-1",
                "task": "task",
                "payload_base64": "",
            },
            "missing priority",
        ),
        (
            {
                "record_version": 2,
                "job_id": "job-1",
                "task": "task",
                "payload_base64": "",
                "priority": "urgent",
            },
            "unknown priority",
        ),
    ],
)
def test_invalid_v2_stored_record_is_rejected(
    stored_job: dict[str, object],
    message: str,
) -> None:
    fixture = {
        "case_id": "bad-stored",
        "operation": "decode_stored_job",
        "stored_job": stored_job,
    }
    with pytest.raises(FixtureError, match=message):
        run_case(fixture)


@pytest.mark.parametrize(
    "fixture",
    [
        [],
        {"operation": 2},
        {
            "operation": "decode_stored_job",
            "case_id": "bad",
            "stored_job": {"record_version": True},
        },
    ],
)
def test_boundary_types_are_checked(fixture: object) -> None:
    with pytest.raises(FixtureError):
        run_case(fixture)


def test_invalid_internal_v2_record_cannot_be_encoded() -> None:
    stored = StoredJob(RecordVersion.V2, "job-1", "task", b"", None)
    with pytest.raises(AssertionError, match="must have priority"):
        _encode_stored_job(stored)


def test_unknown_priority_is_a_zero_effect_decode_result() -> None:
    fixture = valid_fixture()
    request = cast("dict[str, object]", fixture["request"])
    request["priority"] = "urgent"
    result = run_case(fixture)
    actual = cast("dict[str, Any]", result["actual"])
    assert actual == {
        "decode_error": {
            "code": "unknown_priority",
            "field": "priority",
            "value": "urgent",
        },
        "final_jobs": [],
        "store_calls": [],
    }
