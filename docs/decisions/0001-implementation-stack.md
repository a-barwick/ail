# ADR 0001: Authoritative implementation stack

- Status: Proposed
- Date: 2026-07-18
- Owners: project maintainers

## Context

AIL needs an authoritative implementation for its parser, canonical formatter,
semantic model, type-and-effect checker, interpreter, structured diagnostic
engine, and eventually a production backend.

Choosing a stack now would rely on preference rather than project-specific
evidence. The wrong early choice risks either a costly semantic-oracle rewrite
or accidental architecture inherited from a disposable prototype.

## Decision

No implementation stack is selected yet.

Keep the repository root stack-neutral. Evaluate candidates with identical,
bounded prototypes described in
[../stack-evaluation.md](../stack-evaluation.md). Prototype dependencies remain
inside their own directories.

## Consequences

- Normative semantics and protocol work can proceed immediately.
- There is no runnable compiler until a spike is selected.
- Candidate comparisons have a common scope and evidence format.
- Root build tooling will be introduced only with an accepted update to this
  record.

## Alternatives considered

- **Select Rust immediately:** credible long-term default, but unvalidated
  against the source-rewriting and semantic-protocol workload.
- **Prototype the product in TypeScript:** maximizes iteration speed, but may
  create a de facto implementation before native-backend needs are understood.
- **Use separate languages for compiler and tooling from day one:** improves
  local fit but creates schema, release, debugging, and contributor overhead
  before the protocol is stable.

## Validation

Accept this record with a selected stack after:

1. the core semantic subset used by the spike is written;
2. at least Rust and TypeScript complete the common spike;
3. results are scored using weights fixed in advance; and
4. the chosen stack demonstrates a credible path through the semantic oracle
   and first production backend.
