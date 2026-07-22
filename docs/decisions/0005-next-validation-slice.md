# ADR 0005: Validate compiler-guided schema evolution next

- Status: Accepted
- Date: 2026-07-21
- Owners: project maintainers
- Documentation layer and scope: validation-slice selection and roadmap

## Context

M17 completed the deterministic semantic oracle for the accepted job service.
The canonical AIL service passes all 37 public cases and exposes validation,
priority adaptation, persistence, and closed outcome mapping in checked source.
That evidence establishes the final behavior needed by UC-001 and UC-003, but
it does not yet exercise the representative UC-003 change loop.

The compiler can inspect one source unit and perform an atomic rename. It cannot
yet start from the accepted version-one service and mechanically provide:

- stable public and stored schema identities;
- every statically present producer and consumer affected by adding priority;
- the required `must_change`, `review`, and `unchecked` impact categories;
- one atomic whole-workspace priority evolution;
- a semantic diff covering contracts, effects, capabilities, and compatibility;
  or
- revision-bound completion evidence that accounts for the full change.

These are accepted gaps from UC-003, LANG-001, LANG-005, and PROTO-001 through
PROTO-005. They directly test AIL's claim that the compiler can reduce context
discovery, consequence analysis, repair work, and missed-change risk. Extending
the runtime with an unrelated construct would add execution breadth without
testing that claim.

UC-007 remains proposed. Outbound calls, concurrency, replayable external
inputs, production I/O, and native lowering require later use-case or
requirements gates.

## Decision

The next validation slice is **compiler-guided UC-003 priority evolution**.

The slice starts from a canonical version-one job-service workspace and ends at
the already accepted version-two behavior. It is complete when the compiler can
perform this bounded change loop:

1. retain an immutable, ordered source set for revision R1;
2. expose stable schema identities and a typed semantic relationship graph for
   the analyzed workspace;
3. return the exact pre-edit `must_change`, `review`, and `unchecked` impact
   report required by UC-003;
4. validate and atomically commit the complete priority evolution as R2;
5. return canonical per-path edits, a complete identity map, a semantic diff,
   diagnostics, and a machine-readable validation summary; and
6. prove that R2 preserves the accepted 37-case public behavior and the UC-001
   authority and effect contract.

The fixture workspace must contain the semantic roles required by UC-003. It
may use multiple ordered source paths in one compiler build, but this slice does
not introduce a general package or module system. Source unavailable to the
compiler, including external consumers, must appear as explicit unchecked
coverage rather than being treated as analyzed.

The implementation sequence is:

1. **M19 — UC-003 schema-evolution contract:** accept only the language and
   protocol rules needed by this slice and freeze canonical R1/R2 workspaces,
   impact results, transaction results, diagnostics, and completion evidence.
2. **M20 — Workspace semantic graph and impact query:** implement ordered
   multi-source revisions, stable schema identities, the required relationship
   graph, and deterministic impact categorization.
3. **M21 — Atomic schema evolution and completion evidence:** implement the
   validated R1-to-R2 transaction, semantic diff, validation summary, and
   executable regression evidence against the accepted public corpus.

M19 must resolve the exact persistent-identity representation and transaction
request shape before M20 or M21 implements them. The representation must be
general to schema declarations and source edits; it must not add a
fixture-specific `add_priority` compiler operation.

## Consequences

- The next work exercises an accepted representative agent change rather than
  adding a third use case.
- The current single-source `Workspace` representation must grow into an
  ordered source-set revision while preserving immutable parent revisions and
  deterministic handles.
- Stable schema identities become distinct from revision-scoped source handles,
  as required by PROTO-003.
- Impact completeness is claimed only for the analyzed fixture workspace.
  External or unavailable consumers remain explicit in `unchecked`.
- The R1 and R2 fixtures become normative conformance evidence only after M19
  accepts their numbered rules. Existing M11 and M17 contracts remain intact.
- M20 may expose semantic edges already created by accepted record, variant,
  function, field-access, construction, match, and capability semantics. It may
  not add reflection, generics, code generation, or broad collection syntax.
- M21 must reject stale, incomplete, statically invalid, behaviorally invalid,
  or effect-changing candidates without publishing a partial revision.
- UC-007, native lowering, general concurrency, production adapters, and the
  deferred M8 campaign remain outside this sequence.

## Alternatives considered

### Add outbound-call or concurrency semantics

Those cases would exercise important future safety properties, but UC-002 and
UC-004 are not accepted inputs. Starting them would bypass the use-case and
requirements gate while leaving the accepted UC-003 protocol claims untested.

### Start native lowering

The interpreter is sufficient to validate current observable behavior. Native
lowering would test deployment and backend concerns before the compiler has
demonstrated the core inspection-and-change advantage that motivates AIL.

### Activate architectural-health analysis

UC-007 and its requirements remain proposed and require their own acceptance
gate. Its graph analysis may later reuse the semantic graph built by M20, but it
does not define or expand this slice.

### Expand directly to a broad language core

A broad syntax campaign would make conformance and causality harder to review.
The selected R1-to-R2 change needs a much smaller set of identity and protocol
rules tied to already accepted behavior.

## Validation

This decision is implemented when:

1. M17 evidence and remaining accepted gaps are recorded in this ADR;
2. the roadmap marks M18 complete and adds M19 through M21 with explicit
   dependencies, non-scope, focused checks, and exit criteria;
3. `docs/STATUS.md` activates M19 with a contract-first handoff;
4. project guidance identifies compiler-guided schema evolution as the selected
   slice; and
5. `python3 tools/check_docs.py` passes.
