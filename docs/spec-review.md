# Open technical decisions

The first compiler is running, but the language is still narrow. These questions
block normal application development and independent compatible implementations.

## Language semantics

1. **Numbers:** integer widths, overflow faults, floating-point behavior, NaNs,
   text encoding, and equality.
2. **Memory:** allocation, ownership or tracing, aliasing, mutation, lifetimes,
   and observable resource failure.
3. **Iteration and collections:** ordering, hashing, bounds, allocation, and
   deterministic traversal.
4. **Errors and faults:** the boundary between typed domain errors and language
   faults, and whether faults can be caught.
5. **Concurrency:** child failure, cancellation, simultaneous readiness, scope
   exit, result ordering, capability sharing, and resource bounds.
6. **Packages:** dependency identity, versions, initialization, visibility, and
   package cycles beyond the current source-set modules.
7. **Replay:** which external values and scheduling decisions are recorded, how
   records are versioned, and how secrets are redacted.
8. **Foreign code:** declared requirements and guarantees, sandboxing, coverage,
   replay, and failure containment.

## Compiler interface

The in-process Rust APIs define current behavior. A stable external protocol
still needs transport-independent rules for version negotiation, revision
lifetime, concurrent clients, handle invalidation, transaction authorization,
diagnostic compatibility, context budgets, workspace boundaries, and protocol
evolution.

## Determinism hazards

Future work must prevent platform state from leaking into logical results through
map iteration, hash seeds, filesystem order, path normalization, locale, Unicode
versions, time-zone data, timestamps, absolute build paths, allocator behavior,
dependency retrieval, or simultaneous events.

Logical execution determinism and reproducible build artifacts are separate
requirements. The current interpreter tests the former. No native build exists
to test the latter.

## Next executable move

Select one representative service behavior that the current compiler cannot
express, define its exact source, static, runtime, diagnostic, and protocol
results, then implement the smallest missing semantics and run the full existing
suite. Do not design all eight areas at once.
