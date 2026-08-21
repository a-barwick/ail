# ADR 0010: Add one bounded outbound scheduler

- Status: Accepted contract
- Date: 2026-08-09

## Context

M29 can repeat work only sequentially and M30 completes one request before
returning. Neither can execute the bounded batch lookup while exposing a fixed
active-request limit, completion order, or whole-batch cancellation.

## Decision

Add contextual `parallel map ... limit C` only for one direct outbound
operation over `List<T, N>`, with `1 <= C <= N`. The interpreter prepares all
controls before work, uses host-supplied opaque handles and start/check/cancel/
collect methods, bounds active handles, and stores completions by input index.

Whole-batch cancellation is cooperative. Unexpected host faults remain faults,
stop starts, and trigger best-effort cancellation without being replaced by a
cleanup fault. Inspection exposes bounds, controls, ordering, outcomes, digest,
and separate start/completion order.

## Consequences

The compiler can audit one real batch service without introducing general
concurrency. The synchronous interpreter can wait forever if a host never
reports progress; it cannot preempt host code. General async, retries, clocks,
networking, and hard deadlines remain excluded.

## Alternatives considered

- General futures or threads: unnecessary and substantially broader.
- Sequential M29 map: cannot demonstrate overlapping host work.
- Completion-order output: breaks positional auditability.
- Converting host faults to Cancelled: hides the original failure.

## Validation

`python3 specs/tools/bounded_outbound_workflow_contract.py check` validates the
contract artifacts and canonical source. Rust tests execute scheduler and
failure behavior.
