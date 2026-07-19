"""Transport-independent create-job handler."""

from __future__ import annotations

import re
import unicodedata
from typing import assert_never

from v2.domain import (
    AlreadyExists,
    Created,
    CreateJobRequest,
    CreateJobResult,
    InsertOutcome,
    Invalid,
    Job,
    JobStore,
    PersistenceUnavailable,
    Priority,
    ValidationField,
    ValidationIssue,
    ValidationReason,
)

_JOB_ID_PATTERN = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]{0,63}\Z")


def create_job(request: CreateJobRequest, store: JobStore) -> CreateJobResult:
    """Complete validation before the only capability call."""
    priority, issues = _validate(request)
    if issues:
        return Invalid(issues)
    if priority is None:
        msg = "validated request must have a priority"
        raise AssertionError(msg)

    job = Job(request.job_id, request.task, request.payload, priority)
    outcome = store.insert_if_absent(job)
    match outcome:
        case InsertOutcome.INSERTED:
            return Created(job)
        case InsertOutcome.DUPLICATE:
            return AlreadyExists(job.job_id)
        case InsertOutcome.UNAVAILABLE_BEFORE_COMMIT:
            return PersistenceUnavailable()
        case _:
            assert_never(outcome)


def _validate(
    request: CreateJobRequest,
) -> tuple[Priority | None, tuple[ValidationIssue, ...]]:
    issues: list[ValidationIssue] = []

    if not request.job_id:
        issues.append(ValidationIssue(ValidationField.JOB_ID, ValidationReason.MISSING))
    elif _JOB_ID_PATTERN.fullmatch(request.job_id) is None:
        issues.append(
            ValidationIssue(
                ValidationField.JOB_ID,
                ValidationReason.INVALID_FORMAT,
            )
        )

    if not request.task:
        issues.append(ValidationIssue(ValidationField.TASK, ValidationReason.MISSING))
    elif len(request.task) > 80:
        issues.append(ValidationIssue(ValidationField.TASK, ValidationReason.TOO_LONG))
    elif any(unicodedata.category(char) == "Cc" for char in request.task):
        issues.append(
            ValidationIssue(
                ValidationField.TASK,
                ValidationReason.CONTROL_CHARACTER,
            )
        )

    if len(request.payload) > 4096:
        issues.append(
            ValidationIssue(
                ValidationField.PAYLOAD,
                ValidationReason.PAYLOAD_TOO_LARGE,
            )
        )
    if request.priority is None:
        issues.append(
            ValidationIssue(ValidationField.PRIORITY, ValidationReason.MISSING)
        )
    return request.priority, tuple(issues)
