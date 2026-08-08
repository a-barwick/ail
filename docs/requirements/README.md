# AIL requirements

Requirements state observable system constraints. Numbered specification rules
state the language and compiler behavior that satisfies them.

## Current sets

- [Job-service requirements](reference-slice.md): `APP-001`–`APP-005`,
  `LANG-001`–`LANG-005`, `PROTO-001`–`PROTO-005`, and `NFR-001`–`NFR-005`.
- [Architecture requirements](architectural-health.md): `APP-006`, `LANG-006`,
  `PROTO-006`, `PROTO-007`, `NFR-006`, and `NFR-007`.
- [Bounded ordered list requirements](bounded-ordered-lists.md): `APP-007`,
  `LANG-007`, `PROTO-008`, and `NFR-008`.

The first set drives request validation, one conditional store effect, closed
outcomes, schema evolution, revision-safe semantic inspection, impact analysis,
atomic changes, and benchmark measurement. The second drives revision-bound
architecture snapshots, deltas, policy enforcement, honest coverage, and
rollback. The third adds immutable bounded lists, deterministic sequential map,
whole-input validation before effects, and linked list inspection.

## Identifier classes

- `APP-###`: application behavior and operations
- `LANG-###`: language semantics or analyzability
- `PROTO-###`: compiler-interface behavior
- `NFR-###`: measurable performance, scale, portability, security, or
  reliability

Each requirement names its source use case, exact behavior, rationale,
executable or measurable result, dependencies, and unresolved technical choices.

Application requirements must not smuggle in an implementation. “No store call
before validation” is a requirement. “Use ownership and borrowing” is a design
choice.

Every language or compiler-interface rule must trace to a requirement or a
foundational safety or determinism property. Every conformance fixture must name
the rule it tests.
