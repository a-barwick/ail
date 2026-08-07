# ADR 0005: Implement compiler-guided schema evolution

- Status: Accepted and complete
- Date: 2026-07-21
- Owners: project maintainers
- Documentation layer and scope: M19–M21 compiler behavior

## Context

After M17, AIL could execute the 37-case job service and atomically rename one
symbol. It could not tell an agent every consumer affected by adding `priority`,
validate the complete multi-file change, or bind the result to immutable source
revisions.

UC-003 requires stable public and stored schema identities, exact impact
categories, atomic workspace changes, semantic diffs, and complete result data.

## Decision

Implement the UC-003 priority change as a general compiler operation:

1. store immutable ordered source sets;
2. keep schema identities separate from revision-scoped source handles;
3. build typed semantic relationships;
4. return exact `must_change`, reasoned `review`, and explicit `unchecked`
   results;
5. validate the complete candidate before publication; and
6. return canonical edits, identity mapping, semantic changes, diagnostics, and
   validation results for the exact revisions.

Do not add a fixture-specific `add_priority` operation. Unavailable external
consumers must appear in `unchecked`.

## Consequences

- M19 defines the rules and fixtures.
- M20 implements source-set revisions, identities, the graph, and impact query.
- M21 implements atomic candidate validation and publication.
- Stale, incomplete, statically invalid, effect-changing, or behaviorally
  invalid candidates publish nothing.
- The operation claims completeness only for declared analyzed coverage.

## Alternatives considered

### Add runtime features

Rejected for this work. Outbound calls and concurrency did not close the already
specified UC-003 change gap.

### Start native lowering

Rejected. The deterministic interpreter already tested the required behavior;
the missing capability was safe change, not execution speed.

### Add a broad language core

Rejected. Schema identity and revision-safe change required a smaller set of
compiler semantics.

## Validation

The R1 impact result must contain every required change and no extra
`must_change` entry. The valid R1-to-R2 candidate must publish one child and pass
all 37 public cases. Every stale, incomplete, incompatible, effect-changing, or
behaviorally invalid candidate must leave R1 unchanged.
