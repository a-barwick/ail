from __future__ import annotations

from dataclasses import replace

import pytest

from v2.domain import (
    AlreadyExists,
    Created,
    CreateJobRequest,
    InsertOutcome,
    Invalid,
    Job,
    PersistenceUnavailable,
    Priority,
    ValidationField,
    ValidationIssue,
    ValidationReason,
)
from v2.service import create_job
from v2.store import DeterministicJobStore, RecordVersion


def request() -> CreateJobRequest:
    return CreateJobRequest(
        "job-1042",
        "rebuild-search-index",
        b'{"tenant":"north"}',
        Priority.HIGH,
    )


def store(
    outcome: InsertOutcome = InsertOutcome.INSERTED,
) -> DeterministicJobStore:
    return DeterministicJobStore((), outcome, RecordVersion.V2)


@pytest.mark.parametrize("priority", list(Priority))
def test_priority_propagates_unchanged(priority: Priority) -> None:
    value = replace(request(), priority=priority)
    jobs = store()

    assert create_job(value, jobs) == Created(
        Job(value.job_id, value.task, value.payload, priority)
    )
    assert jobs.jobs[0].priority is priority
    assert jobs.calls[0].job.priority is priority


def test_missing_priority_is_ordered_after_other_issues_without_effect() -> None:
    value = CreateJobRequest("", "", bytes(4097), None)
    jobs = store()

    result = create_job(value, jobs)

    assert result == Invalid(
        (
            ValidationIssue(ValidationField.JOB_ID, ValidationReason.MISSING),
            ValidationIssue(ValidationField.TASK, ValidationReason.MISSING),
            ValidationIssue(
                ValidationField.PAYLOAD,
                ValidationReason.PAYLOAD_TOO_LARGE,
            ),
            ValidationIssue(ValidationField.PRIORITY, ValidationReason.MISSING),
        )
    )
    assert jobs.calls == ()
    assert jobs.jobs == ()


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
def test_failure_outcomes_keep_state_and_make_exactly_one_call(
    outcome: InsertOutcome,
    expected: object,
) -> None:
    jobs = store(outcome)
    assert create_job(request(), jobs) == expected
    assert len(jobs.calls) == 1
    assert jobs.jobs == ()
