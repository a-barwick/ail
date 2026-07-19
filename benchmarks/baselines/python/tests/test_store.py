from __future__ import annotations

import pytest

from v2.domain import InsertOutcome, Job, Priority
from v2.store import (
    DeterministicJobStore,
    RecordVersion,
    StoreContractError,
    StoredJob,
)


def job() -> Job:
    return Job("job-1", "task", b"payload", Priority.HIGH)


def test_v1_record_adapts_explicitly_to_normal() -> None:
    stored = StoredJob(RecordVersion.V1, "job-1", "task", b"x", None)
    assert stored.adapt_to_v2() == Job(
        "job-1",
        "task",
        b"x",
        Priority.NORMAL,
    )


def test_v2_insert_preserves_priority_and_returns_immutable_snapshots() -> None:
    jobs = DeterministicJobStore(
        (),
        InsertOutcome.INSERTED,
        RecordVersion.V2,
    )
    assert jobs.insert_if_absent(job()) is InsertOutcome.INSERTED

    assert jobs.jobs == (
        StoredJob(
            RecordVersion.V2,
            "job-1",
            "task",
            b"payload",
            Priority.HIGH,
        ),
    )
    assert jobs.calls[0].job == jobs.jobs[0]


def test_inserted_outcome_rejects_duplicate_initial_state() -> None:
    existing = StoredJob(
        RecordVersion.V2,
        "job-1",
        "old",
        b"",
        Priority.LOW,
    )
    jobs = DeterministicJobStore(
        (existing,),
        InsertOutcome.INSERTED,
        RecordVersion.V2,
    )
    with pytest.raises(StoreContractError, match="postcondition"):
        jobs.insert_if_absent(job())
    assert len(jobs.calls) == 1
    assert jobs.jobs == (existing,)


@pytest.mark.parametrize(
    "outcome",
    [InsertOutcome.DUPLICATE, InsertOutcome.UNAVAILABLE_BEFORE_COMMIT],
)
def test_non_insert_outcomes_never_mutate_state(outcome: InsertOutcome) -> None:
    existing = StoredJob(
        RecordVersion.V1,
        "other",
        "old",
        b"",
        None,
    )
    jobs = DeterministicJobStore((existing,), outcome, RecordVersion.V2)
    assert jobs.insert_if_absent(job()) is outcome
    assert jobs.jobs == (existing,)
    assert len(jobs.calls) == 1
