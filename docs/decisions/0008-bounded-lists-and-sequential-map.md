# ADR 0008: Add bounded lists and sequential map

- Status: Accepted and complete
- Date: 2026-08-08

## Context

After M28, AIL could compose modules and ordinary functions but could not
express one common service behavior: accept a runtime-sized bounded group,
perform checked work in order, and return aligned results. Manual unrolling
cannot represent lengths from zero through an application bound. Moving the
operation into the host would hide ordering, dependencies, effects, and element
identity from the compiler.

The checker previously represented value types as strings and used exact string
equality. Adding a structural collection by parsing those strings in each
consumer would make linking, rename, inspection, runtime validation, and graph
facts drift.

## Decision

M29 adds one immutable structural type, `List<Element, MaxLength>`, and one
contextual binder expression:

```ail
map item in items {
  transform(item)
}
```

The type model is recursive and canonical. Initial equality is exact after
linked element-name resolution; bounds do not widen implicitly. Bounds are
positive and at most `u32::MAX`. A list may be empty, preserves insertion order
and duplicates, and carries no trusted runtime bound metadata.

`map` evaluates its source once, then completes one body evaluation before
starting the next. Its result preserves the source bound and actual length.
Calls and capabilities use the existing call paths, so transitive effects,
recursion checks, runtime traces, and faults remain visible. Complete external
ordinary-value validation precedes capability availability checks and calls.

This increment excludes nested lists, literals, indexing, mutation, append,
filter, fold, arbitrary iteration, implicit bound conversion, early successful
exit, parallelism, and catch-and-continue fault handling. `map` and `in` remain
contextual identifiers to preserve existing source compatibility.

## Consequences

- AIL can execute bounded ordered batch cancellation through ordinary modules,
  functions, and capabilities.
- Bounds, resolved element identities, result types, effects, capabilities,
  dependencies, module identity, and runtime calls are compiler facts.
- Public Rust syntax and runtime enums gain structural type and list variants;
  exhaustive embedders must update when adopting M29.
- `map` guarantees one body result per input position, but the current
  set-valued effect analysis does not prove one capability invocation per
  position. The application source and exact trace tests establish that
  workload property.
- Provider faults are fail-stop and preserve the prior observed-call prefix;
  no rollback or partial successful result is promised.
- The in-process host allocates a list before interpreter validation. A future
  transport must enforce bounds during decoding to cap host allocation.

## Alternatives considered

### General loops and mutable collections

Rejected. They require mutation, termination, accumulation, aliasing, and
resource rules not needed for the selected behavior.

### Conventional `map` with lambdas or methods

Rejected. AIL has no closures, first-class functions, or method model. The
binder form exposes the same operation without adding those unrelated systems.

### Fixed tuples or exact-size arrays

Rejected. They require manual unrolling or padding and cannot directly model a
runtime length from zero through 32.

### Host-only batch intrinsic

Rejected. It would make ordering, effects, dependencies, and element identities
opaque and fixture-specific.

### Parallel map

Rejected. Simultaneous readiness, shared capability authority, cancellation,
child failure, and deterministic scheduling are unresolved. Sequential map is
the smallest behavior required by UC-009.

## Validation

[`m29_bounded_lists.rs`](../../compiler/ail-compiler/tests/m29_bounded_lists.rs)
must prove canonical syntax, contextual-name compatibility, bounds and static
diagnostics, linked alias resolution, transitive effects, semantic inspection,
empty and exact-bound execution, ordered mixed outcomes, duplicate preservation,
pre-effect rejection, fail-stop faults, source-order independence, and retained
revision behavior. The standalone
[`bounded-list contract checker`](../../specs/tools/bounded_list_contract.py)
and all pre-M29 compiler, contract, architecture, and benchmark checks must pass.
