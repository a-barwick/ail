# UC-009 — Cancel a bounded ordered group of jobs

Status: **Accepted 2026-08-08**

## Task

Accept zero to 32 job identifiers, process every input position in stored order,
call `JobsStore.cancel` once for that position, and return one closed
`CancelOutcome` at the same output position. The accepted outcomes are
`Cancelled`, `AlreadyFinished`, `NotFound`, and `Unavailable`; expected store
failures are outcomes, not runtime faults.

Empty input returns empty output and makes no call. Duplicate identifiers remain
distinct input positions and are processed independently. A list longer than 32
or any malformed element is rejected before capability availability is checked
and before any capability call.

An unexpected provider fault is fail-stop. Calls completed before the failing
index remain observable, the failing call has no result, no later position is
processed, and execution returns no partial list or rollback claim.

## System boundary and authority

The host supplies an immutable ordered list and the `JobsStore` capability. AIL
source has no ambient storage, clock, randomness, network, or concurrency
authority. The service delegates each position to the existing checked
single-item cancellation function, which owns the only capability call site.

The compiler-visible application bound is 32. The language accepts positive
list bounds through 4,294,967,295, but this service deliberately selects 32.
The bound limits values accepted by execution; the in-process host has already
allocated its `RuntimeValue` before the interpreter validates it.

## Canonical program

The executable three-module program is under
[`compiler/examples/batch-cancellation/`](../../compiler/examples/batch-cancellation/).
Its entry point is:

```ail
fn cancel_batch(items: List<domain.JobId, 32>, store: capability JobsStore) -> List<domain.CancelOutcome, 32> effects { store.cancel } {
  map item in items {
    single.cancel_one(item)
  }
}
```

`map` evaluates its source once and its body sequentially for indices
`0..length`. It always produces one result per input position. The language does
not infer an invocation-count proof from the set-valued effect model; the
one-call-per-position property is established by this source's single call path
and its exact runtime trace tests.

## Change and correctness checks

An agent adding or changing this operation must preserve:

- exact `List<T, N>` element identity and bound at function boundaries;
- imported, aliased, and qualified type and call resolution;
- complete transitive effect checking through the map body;
- whole-input validation before effects;
- ordered output and observed-call traces, including duplicates;
- fail-stop provider-fault behavior; and
- revision-bound inspection and execution.

The M29 tests cover empty, mixed, duplicate, exact-bound, oversized, late
malformed, and provider-fault inputs; source-vector reversal; retained
revisions; inspection; diagnostics; and all prior compiler behavior.

## Human audit

A reviewer can inspect the canonical source, linked module and function
identities, exact input and output list types, element schema identities, bound,
capability parameter, declared effect, helper dependency, ordered runtime calls,
and the requested source revision. No claim of parallelism, general collection
support, runtime rollback, or production readiness is implied.

## Requirements

This use case owns `APP-007`, `LANG-007`, `PROTO-008`, and `NFR-008` in
[the bounded ordered list requirements](../requirements/bounded-ordered-lists.md).
The exact language and runtime rules are in
[`specs/bounded-lists.md`](../../specs/bounded-lists.md).
