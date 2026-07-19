# ADR 0001: Defer authoritative implementation-stack selection

- Status: Accepted
- Date: 2026-07-18
- Owners: project maintainers
- Documentation layer and scope: implementation evidence and repository
  architecture

## Context

AIL needs an authoritative implementation for its parser, canonical formatter,
semantic model, type-and-effect checker, interpreter, structured diagnostic
engine, and eventually a production backend.

Choosing a stack now would rely on preference rather than project-specific
evidence. The wrong early choice risks either a costly semantic-oracle rewrite
or accidental architecture inherited from a disposable prototype.

## Decision

Defer selection of the authoritative implementation stack until the common
compiler spikes are complete.

Keep the repository root stack-neutral. Evaluate candidates with identical,
bounded prototypes described in
[../stack-evaluation.md](../stack-evaluation.md). Prototype dependencies remain
inside their own directories.

## Consequences

- Application use cases and requirements can proceed immediately; normative
  semantics and protocol work follow from their accepted validation slice.
- There is no runnable compiler until a spike is selected.
- Candidate comparisons have a common scope and evidence format.
- Root build tooling will be introduced only after M13 accepts the
  stack-selection ADR that supersedes this record.

## Alternatives considered

- **Select Rust immediately:** credible long-term default, but unvalidated
  against the source-rewriting and semantic-protocol workload.
- **Prototype the product in TypeScript:** maximizes iteration speed, but may
  create a de facto implementation before native-backend needs are understood.
- **Use separate languages for compiler and tooling from day one:** improves
  local fit but creates schema, release, debugging, and contributor overhead
  before the protocol is stable.

## Validation

This deferral remains valid until:

1. M11 freezes the semantic subset and shared spike fixtures;
2. at least Rust and TypeScript complete the M12 common spike;
3. results are scored using weights fixed in advance; and
4. one candidate demonstrates a credible path through the semantic oracle and
   first production backend.

M13 records the selected stack in a new ADR and marks this deferral record
superseded. Reopening the choice earlier requires new evidence and a separate
reviewed decision.
