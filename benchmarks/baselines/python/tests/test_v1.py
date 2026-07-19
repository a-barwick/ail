from __future__ import annotations

from dataclasses import dataclass, field
from typing import cast

import pytest

from v1.job_service import (
    AlreadyExists,
    Created,
    CreateJobRequest,
    InsertOutcome,
    Invalid,
    Job,
    PersistenceUnavailable,
    ValidationField,
    ValidationIssue,
    ValidationReason,
    create_job,
)


@dataclass
class RecordingStore:
    outcome: InsertOutcome
    calls: list[Job] = field(default_factory=list)

    def insert_if_absent(self, job: Job) -> InsertOutcome:
        self.calls.append(job)
        return self.outcome


def valid_request(**changes: object) -> CreateJobRequest:
    values: dict[str, object] = {
        "job_id": "job-1042",
        "task": "rebuild-search-index",
        "payload": b'{"tenant":"north"}',
    }
    values.update(changes)
    return CreateJobRequest(
        job_id=cast("str", values["job_id"]),
        task=cast("str", values["task"]),
        payload=cast("bytes", values["payload"]),
    )


def test_inserted_request_makes_one_exact_call() -> None:
    store = RecordingStore(InsertOutcome.INSERTED)
    request = valid_request()

    result = create_job(request, store)

    expected = Job(request.job_id, request.task, request.payload)
    assert result == Created(expected)
    assert store.calls == [expected]


def test_validation_collects_one_issue_per_field_in_contract_order() -> None:
    store = RecordingStore(InsertOutcome.INSERTED)
    request = CreateJobRequest("", "", bytes(4097))

    result = create_job(request, store)

    assert result == Invalid(
        (
            ValidationIssue(ValidationField.JOB_ID, ValidationReason.MISSING),
            ValidationIssue(ValidationField.TASK, ValidationReason.MISSING),
            ValidationIssue(
                ValidationField.PAYLOAD,
                ValidationReason.PAYLOAD_TOO_LARGE,
            ),
        )
    )
    assert store.calls == []


@pytest.mark.parametrize(
    ("job_id", "valid"),
    [
        ("a", True),
        ("A0", True),
        ("a" + "_" * 63, True),
        ("", False),
        ("_" + "a" * 63, False),
        ("a" * 65, False),
        ("job.dot", False),
        ("jób", False),
    ],
)
def test_job_id_contract(job_id: str, valid: bool) -> None:
    result = create_job(
        valid_request(job_id=job_id),
        RecordingStore(InsertOutcome.INSERTED),
    )
    assert isinstance(result, Created) is valid


@pytest.mark.parametrize(
    ("task", "reason"),
    [
        ("", ValidationReason.MISSING),
        ("界" * 80, None),
        ("界" * 81, ValidationReason.TOO_LONG),
        ("line\nbreak", ValidationReason.CONTROL_CHARACTER),
    ],
)
def test_task_counts_unicode_scalars_and_rejects_controls(
    task: str,
    reason: ValidationReason | None,
) -> None:
    result = create_job(
        valid_request(task=task),
        RecordingStore(InsertOutcome.INSERTED),
    )
    if reason is None:
        assert isinstance(result, Created)
    else:
        assert result == Invalid((ValidationIssue(ValidationField.TASK, reason),))


@pytest.mark.parametrize("size", [0, 4096, 4097])
def test_payload_bounds_count_bytes(size: int) -> None:
    store = RecordingStore(InsertOutcome.INSERTED)
    result = create_job(valid_request(payload=bytes(size)), store)
    if size <= 4096:
        assert isinstance(result, Created)
        assert len(store.calls) == 1
    else:
        assert result == Invalid(
            (
                ValidationIssue(
                    ValidationField.PAYLOAD,
                    ValidationReason.PAYLOAD_TOO_LARGE,
                ),
            )
        )
        assert store.calls == []


@pytest.mark.parametrize(
    ("outcome", "expected"),
    [
        (InsertOutcome.DUPLICATE, AlreadyExists("job-1042")),
        (
            InsertOutcome.UNAVAILABLE_BEFORE_COMMIT,
            PersistenceUnavailable(),
        ),
    ],
)
def test_closed_store_outcomes_follow_one_call(
    outcome: InsertOutcome,
    expected: object,
) -> None:
    store = RecordingStore(outcome)
    assert create_job(valid_request(), store) == expected
    assert len(store.calls) == 1
