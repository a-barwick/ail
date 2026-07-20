# ADR 0004: Use Rust for the authoritative AIL compiler

- Status: Accepted
- Date: 2026-07-19
- Owners: project maintainers
- Supersedes: [ADR 0001](0001-implementation-stack.md)
- Amends: [ADR 0003](0003-prioritize-stack-decision.md)
- Documentation layer and scope: implementation architecture and roadmap

## Context

M11 completed the shared five-construct language and protocol contract. The
roadmap then called for disposable Rust and TypeScript implementations before a
separate stack-selection milestone.

The maintainers have selected Rust directly and removed that comparison from
the delivery path. The TypeScript spike would reduce uncertainty about
implementation convenience, but it would not change the desired long-term
properties of the authoritative compiler:

- strong representation invariants for syntax and typed semantic data;
- predictable native binaries without a required managed runtime;
- a credible path through the semantic oracle to a production backend;
- good cross-platform and WebAssembly support; and
- one implementation language for compiler semantics, diagnostics, structural
  edits, and eventual lowering.

The remaining Rust risks are real: ownership friction in graph-heavy data,
compile time, and contributor learning cost. They are implementation risks to
manage in the production compiler, not reasons to delay it behind a second
throwaway implementation.

## Decision

Rust is the authoritative implementation language for the AIL compiler through
the first production backend.

Cancel the M12 Rust/TypeScript comparison and the separate M13 decision
milestone. Preserve M11 as the first implementation contract. Begin the
production compiler as a Rust workspace at the repository root and deliver it
through small conformance milestones:

1. lossless syntax, deterministic recovery, and canonical formatting;
2. name resolution, local inference, capability checking, and structured
   diagnostics;
3. immutable revisions, inspection, validated rename, and identity mapping;
4. deterministic interpretation of the accepted job-service core; and
5. broader core semantics followed by native lowering.

TypeScript remains available for clients or editor integrations if a concrete
need appears. It is not a second semantic implementation and does not define
AIL behavior.

## Consequences

- A root Cargo workspace and production `compiler/` tree are now authorized.
- M11 fixtures constrain the first Rust implementation; compiler behavior may
  not silently reinterpret them.
- M12 and M13 remain in the roadmap as superseded history rather than active
  delivery gates.
- Stack-comparison weights and prototype scorecards are archived planning
  material, not prerequisites.
- Rust-specific architecture choices still require ordinary review. This ADR
  does not choose a parser library, storage model, incremental engine, backend,
  or memory semantics for AIL programs.
- The deferred benchmark campaign remains deferred and does not block compiler
  implementation.

## Alternatives considered

### Complete the Rust and TypeScript spikes

This would preserve the earlier evidence plan, but it would spend another
milestone implementing semantics in a stack the maintainers do not intend to
keep. The direct Rust decision accepts that foregone comparison explicitly.

### Start in TypeScript and rewrite later

This improves early protocol iteration but creates a likely semantic-oracle
rewrite and a period with two sources of truth. That cost conflicts with the
project's emphasis on deterministic, reviewable semantics.

### Split the compiler and tooling stacks immediately

This adds schema, release, debugging, and contributor boundaries before the
semantic protocol has demonstrated a need for them.

## Validation

This decision is implemented when:

1. ADR 0001 is marked superseded;
2. the roadmap marks M12 and M13 superseded and activates a Rust compiler
   milestone with a concrete verification command;
3. status no longer blocks production source on a candidate comparison;
4. the root Rust workspace builds and tests; and
5. `python3 tools/check_docs.py` passes.
