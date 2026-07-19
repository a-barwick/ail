"""Deterministic jobs capability used by the benchmark."""

from __future__ import annotations

from dataclasses import dataclass
from enum import IntEnum

from v2.domain import InsertOutcome, Job, Priority


class StoreContractError(RuntimeError):
    """Raised when the supplied deterministic outcome violates its postcondition."""


class RecordVersion(IntEnum):
    V1 = 1
    V2 = 2


@dataclass(frozen=True, slots=True)
class StoredJob:
    record_version: RecordVersion
    job_id: str
    task: str
    payload: bytes
    priority: Priority | None

    def adapt_to_v2(self) -> Job:
        """Map a V1 record explicitly to normal priority."""
        priority = (
            self.priority
            if self.record_version is RecordVersion.V2 and self.priority is not None
            else Priority.NORMAL
        )
        return Job(self.job_id, self.task, bytes(self.payload), priority)


@dataclass(frozen=True, slots=True)
class StoreCall:
    job: StoredJob


class DeterministicJobStore:
    """Record calls and apply one predefined insert outcome."""

    def __init__(
        self,
        jobs: tuple[StoredJob, ...],
        outcome: InsertOutcome,
        insert_version: RecordVersion,
    ) -> None:
        self._jobs = list(jobs)
        self._outcome = outcome
        self._insert_version = insert_version
        self._calls: list[StoreCall] = []

    @property
    def jobs(self) -> tuple[StoredJob, ...]:
        return tuple(self._jobs)

    @property
    def calls(self) -> tuple[StoreCall, ...]:
        return tuple(self._calls)

    def insert_if_absent(self, job: Job) -> InsertOutcome:
        stored = StoredJob(
            record_version=self._insert_version,
            job_id=job.job_id,
            task=job.task,
            payload=bytes(job.payload),
            priority=(
                job.priority if self._insert_version is RecordVersion.V2 else None
            ),
        )
        self._calls.append(StoreCall(stored))
        if self._outcome is InsertOutcome.INSERTED:
            if any(current.job_id == job.job_id for current in self._jobs):
                msg = (
                    "inserted outcome violates insert-if-absent postcondition "
                    f"for {job.job_id!r}"
                )
                raise StoreContractError(msg)
            self._jobs.append(stored)
        return self._outcome
