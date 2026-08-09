# M31 bounded outbound workflow contract

Status: **Accepted 2026-08-09**

M31 combines M29 bounded lists with M30 cooperative outbound requests for the
[batch lookup workload](../docs/workloads/bounded-outbound-batch-lookup.md). It
adds one bounded scheduler, not threads or general asynchronous programming.

## Static contract

### M31-LANG-001 — Explicit fixed concurrency

`parallel map item in requests limit 3 { operation(...) }` accepts a direct
bounded-list function parameter with type
`List<T, N>` and a canonical integer limit in `1..=N`. The body is exactly one
direct outbound capability call with no bindings. Its result is `List<U, N>`.
The contextual words remain ordinary identifiers outside this form.

### M31-LANG-002 — Exact authority and complete effects

The operation requires its capability parameter and declared effect. Function
architecture facts include every outside operation and state read/write reached
through calls, including linked-module calls. Callers do not inherit callee CFG
complexity.

### M31-LANG-003 — Controls precede outside work

Timeout and cancellation controls are effect-free names, with a timeout literal
also permitted. The interpreter validates external list shape, concurrency,
every item timeout, and cancellation token before provider support or start.
Invalid input starts and records zero requests.

## Runtime contract

### M31-RUNTIME-001 — Bounded starts and ordered results

Starts follow input order and active handles never exceed the fixed limit. Host
completion order may differ. Each collected value is stored at its input index,
including returned, timed-out, cancelled, and expected remote outcomes.

### M31-RUNTIME-002 — Cooperative whole-batch cancellation

When check reports cancellation, the interpreter collects simultaneously
reported completions, starts no more work, requests cancellation of active
handles, preserves completed values, and marks every other position Cancelled.
The host must cooperate; AIL cannot forcibly interrupt stuck host code.

### M31-RUNTIME-003 — Unexpected failure is fail-stop

An unexpected start, check, collect, or result-contract failure stops new
starts, requests cancellation of active handles, preserves the started trace,
and returns the original fault. Cleanup faults do not replace it. No retry or
remote rollback is implied.

### M31-RUNTIME-004 — Narrow host lifecycle

The host supplies opaque handles and start, check, cancellation-request, and
collect operations. Expected completion remains M30's closed
`Returned | TimedOut | Cancelled` outcome.

## Inspection and revisions

### M31-PROTO-001 — Complete workflow inspection

Inspection reports operation/effect, input bound, concurrency limit,
timeout/cancellation indices, types and inputs, `input-order` result alignment,
all closed variant cases, and the saved capability-environment digest. Execution
reports batch index, start order, and completion order separately.

### M31-PROTO-002 — Inherited immutable settings

Every ordinary parent-based source child inherits the exact architecture config
and digest. Only a dedicated settings operation may replace them. Retained old
revisions preserve their own source, architecture settings, capability
environment, inspection, and behavior.

### M31-PROTO-003 — Atomic safe publication

Candidate construction checks parsing, types, list and concurrency bounds,
capability permission, complete transitive effects, architecture policy, saved
host settings, and behavior before publication. Failure publishes no child.

## Canonical proof

`compiler/examples/batch-lookup/` maps at most eight requests with limit three.
`m31_bounded_outbound_workflows.rs` proves bounded activity, out-of-order
completion with aligned results, zero-start rejection, stable timeout/cancel
positions, whole-batch cancellation, fail-stop cleanup, and inspection.
`source_architecture.rs` proves inherited settings and transitive state facts.

## Non-goals

No LLVM, unrestricted concurrency, general async, threads, retries, networking,
hard deadlines, forced interruption, mutable lists, or remote rollback.
