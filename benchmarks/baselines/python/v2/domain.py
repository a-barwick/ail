"""Public contracts for the evolved job service."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, IntEnum, StrEnum
from typing import Protocol


class ApiVersion(IntEnum):
    V1 = 1
    V2 = 2


class Priority(StrEnum):
    LOW = "low"
    NORMAL = "normal"
    HIGH = "high"


@dataclass(frozen=True, slots=True)
class CreateJobRequest:
    job_id: str
    task: str
    payload: bytes
    priority: Priority | None


@dataclass(frozen=True, slots=True)
class Job:
    job_id: str
    task: str
    payload: bytes
    priority: Priority


class ValidationField(StrEnum):
    JOB_ID = "job_id"
    TASK = "task"
    PAYLOAD = "payload"
    PRIORITY = "priority"


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
    """The sole external capability available to the handler."""

    def insert_if_absent(self, job: Job) -> InsertOutcome:
        """Conditionally persist one job and return the supplied outcome."""
