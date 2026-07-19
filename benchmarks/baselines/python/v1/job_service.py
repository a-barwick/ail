"""Typed UC-001 job-service checkpoint."""

from __future__ import annotations

import re
import unicodedata
from dataclasses import dataclass
from enum import Enum, StrEnum
from typing import Protocol, assert_never

_JOB_ID_PATTERN = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]{0,63}\Z")


@dataclass(frozen=True, slots=True)
class CreateJobRequest:
    job_id: str
    task: str
    payload: bytes


@dataclass(frozen=True, slots=True)
class Job:
    job_id: str
    task: str
    payload: bytes


class ValidationField(StrEnum):
    JOB_ID = "job_id"
    TASK = "task"
    PAYLOAD = "payload"


class ValidationReason(StrEnum):
    MISSING = "missing"
    INVALID_FORMAT = "invalid_format"
    TOO_LONG = "too_long"
    CONTROL_CHARACTER = "control_character"
    PAYLOAD_TOO_LARGE = "payload_too_large"


@dataclass(frozen=True, slots=True)
class ValidationIssue:
    field: ValidationField
    reason: ValidationReason


@dataclass(frozen=True, slots=True)
class Created:
    job: Job


@dataclass(frozen=True, slots=True)
class Invalid:
    issues: tuple[ValidationIssue, ...]


@dataclass(frozen=True, slots=True)
class AlreadyExists:
    job_id: str


@dataclass(frozen=True, slots=True)
class PersistenceUnavailable:
    pass


type CreateJobResult = Created | Invalid | AlreadyExists | PersistenceUnavailable


class InsertOutcome(Enum):
    INSERTED = "inserted"
    DUPLICATE = "duplicate"
    UNAVAILABLE_BEFORE_COMMIT = "unavailable_before_commit"


class JobStore(Protocol):
    """The only capability available to ``create_job``."""

    def insert_if_absent(self, job: Job) -> InsertOutcome:
        """Record one conditional insert and return its supplied outcome."""


def create_job(request: CreateJobRequest, store: JobStore) -> CreateJobResult:
    """Validate completely, then make exactly one store call."""
    issues = _validate(request)
    if issues:
        return Invalid(issues)

    job = Job(request.job_id, request.task, request.payload)
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


def _validate(request: CreateJobRequest) -> tuple[ValidationIssue, ...]:
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
    return tuple(issues)
