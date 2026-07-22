# AIL use cases

Status: **Active discovery**

Documentation layer: concrete application scenarios. Use cases translate the
[application vision](../application-vision.md) into inputs for
[numbered requirements](../requirements/README.md). They are non-normative for
language behavior.

UC-001 and UC-003 were accepted as the first reference slice on 2026-07-18.

## Accepted reference slice

The first reference system is a small, transport-independent job-submission
service:

- accept a structured create-job request;
- validate bounded input;
- conditionally persist one job through an explicit datastore capability;
- return one closed success or domain-failure result; and
- evolve the public and stored schema with a required priority field while
  identifying and updating every affected consumer.

This combines the two smallest candidate cases without requiring concurrency,
networking infrastructure, a production datastore, or a production runtime.

| Record | Status | Representative agent task |
| --- | --- | --- |
| [UC-001 — Request validation and persistence](UC-001-request-validation-and-persistence.md) | Accepted | Implement or repair the bounded handler and prove its ordered effects |
| [UC-003 — Public schema evolution](UC-003-public-schema-evolution.md) | Accepted | Add priority, preserve the declared compatibility policy, and update every affected semantic role |

Their first derived requirements are collected in
[requirements/reference-slice.md](../requirements/reference-slice.md).
The shared test format and comparison rules are under
[benchmarks](../benchmarks/README.md).

## Future cases

After the semantic oracle and core protocol can execute the first reference
slice, choose the next scaling case from:

- **UC-002 — Outbound call control:** call a remote service with explicit
  authority, timeout, cancellation, and typed failure.
- **UC-004 — Bounded fan-out:** perform independent work concurrently with fixed
  ordering, bounds, cancellation, and child-failure behavior.
- **UC-005 — Event worker:** consume an ordered event, enforce idempotency,
  update state, emit telemetry, and acknowledge or reject deterministically.
- **UC-006 — Replayable external inputs:** test behavior involving time,
  randomness, configuration, secrets, filesystem state, or network responses
  through supplied or recorded capabilities.
- **[UC-007 — Architectural regression control](UC-007-architectural-regression-control.md):**
  add an operation to a mature service without enlarging a hotspot, bypassing
  a declared boundary, or hiding responsibility through superficial splitting.

UC-001 and UC-003 were selected first because they exercise public contracts,
errors, stored state, change impact, and safe multi-file edits without requiring
general concurrency or a production runtime.

None of these future cases authorizes language or compiler implementation until
its use case and requirements are accepted and numbered implementation
milestones are added. A bounded acceptance milestone may enter the roadmap to
produce the evidence needed for that decision.

[ADR 0006](../decisions/0006-prepare-architectural-regression-control.md)
selects UC-007 acceptance preparation as active M23. This is a bounded gate,
not acceptance of the use case: UC-007 remains Proposed until its workspace,
behavior, policy, metrics, examples, baseline comparison, and budgets pass.

## Use-case record

Each accepted use-case document should contain:

1. **Identifier and status**
2. **Actor and desired outcome**
3. **System boundary**
4. **Inputs and required capabilities**
5. **Nominal behavior**
6. **Domain errors and language/runtime faults**
7. **Observable state and effects**
8. **Ordering, concurrency, cancellation, and resource bounds**
9. **Schema, serialization, and compatibility constraints**
10. **Representative change tasks**
11. **Human review and operational evidence**
12. **Baseline implementation and tools**
13. **Measurable success criteria**
14. **Derived requirement identifiers**
15. **Explicit exclusions and unresolved questions**

## Examples

Start with language-independent behavior tables, traces, data shapes, or
pseudocode. Do not invent AIL syntax merely to make a use case feel concrete.

Illustrative AIL code may be added only after it is clearly labeled and the
semantic question it explores is identified. It remains non-normative until
linked to proposed numbered rules; it becomes a conformance fixture only after
those rules are accepted.

## Acceptance gate

A use case is ready to drive requirements when two independent readers can agree
on:

- what is inside and outside the system;
- which inputs and effects are observable;
- what success and each failure mean;
- which change the agent must make;
- which evidence proves the change correct; and
- which runtime or operational measurements matter.
