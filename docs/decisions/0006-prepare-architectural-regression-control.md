# ADR 0006: Prepare architectural regression control next

- Status: Accepted
- Date: 2026-07-21
- Owners: project maintainers
- Documentation layer and scope: validation-slice selection and roadmap

## Context

M19 through M21 completed the accepted UC-003 change loop. At a fixed starting
revision, the compiler now returns the exact required changes, bounded review
items, explicit unchecked consumers, canonical edits, identity mapping,
semantic changes, validation results, and completion evidence. It atomically
commits the accepted five-source priority evolution, preserves the one-store
authority and effect order, passes all 37 public behavior cases, and rejects
stale, incomplete, incompatible, effect-changing, or behaviorally failing
candidates without publishing a partial revision.

This proves that the compiler can help an agent make a complete behavior and
schema change in the bounded job service. It does not prove that a behaviorally
correct change preserves the codebase's declared responsibility, dependency,
authority, state, or review boundaries. An agent can still pass every behavior
test while enlarging a central dispatcher, granting transport code datastore
authority, adding a dependency cycle, or hiding the same concentration behind
private helper functions.

[UC-007](../use-cases/UC-007-architectural-regression-control.md), its
[proposed requirements](../requirements/architectural-health.md), and the
[proposed manifest](../architecture-health.md) describe this gap. They are not
accepted inputs yet. The exact workspace, `CancelJob` behavior, project policy,
baseline, fixtures, comparison method, and budgets still need to pass their
acceptance gate.

The compiler's structured facts are the authoritative agent interface. Compact
text is a derived operator view. The next campaign must develop both together:
an agent should receive a small actionable summary first and be able to request
the exact contributing facts, paths, revisions, policy rules, and coverage.

## Decision

Select **architectural regression control** as the next scaling direction, but
do not accept UC-007 or authorize compiler implementation yet.

The immediate active work is an acceptance package. It must freeze enough
evidence that two independent readers can classify a good change, a centralized
but behaviorally correct change, and a superficial helper-splitting change in
the same way. If that gate passes, the later contract and implementation work
may activate one milestone at a time.

The bounded sequence is:

1. **M23 — UC-007 acceptance package.** Freeze the starting service and
   `CancelJob` behavior, good and seeded bad changes, architecture groups and
   allowed dependencies, capability and state boundaries, hotspot baseline,
   minimal metric set, expected structured and compact text outputs, baseline
   comparison method, false-finding allowance, analysis budget, and manifest
   size budget. Accept or reject UC-007 and its requirements only after the
   gate passes.
2. **M24 — Architectural regression contract.** Conditional on M23 acceptance,
   freeze the smallest numbered language and protocol rules, policy model,
   snapshot and delta encoding, stable diagnostics, compact agent rendering,
   completion evidence, and conformance fixtures. This milestone adds no Rust
   implementation.
3. **M25 — Architectural snapshot and agent rendering.** Implement only the
   accepted revision-bound facts, coverage, contributors, aggregate scopes,
   budgets, and deterministic compact rendering needed for the starting
   revision.
4. **M26 — Delta, policy, and atomic enforcement.** Implement compatible
   revision comparison, no-growth and boundary rules, baseline and exception
   handling, good and seeded bad candidate classifications, and transaction
   rollback for denied or incomplete results.
5. **M27 — Non-official agent usability pilot.** Exercise the compiler feedback
   loop with a small recorded pilot. The pilot may use Codex, Amp, or another
   named operator, but it must record the exact agent, version, mode, prompt,
   tools, permissions, thread or run identity, compiler outputs, edits, checks,
   and repairs. It tests whether the output is actionable; it is not official
   comparative evidence and does not resume M8.

The campaign keeps a deliberate effort bias:

- roughly 15–20% framing and acceptance work;
- roughly 60–70% compiler and protocol implementation; and
- roughly 15–20% validation and learning.

These are scope guardrails, not scheduling or accounting targets. M23 must end
when the acceptance inputs are deterministic and reviewable; it must not grow
into general architecture research. M25 and M26 must not build a universal
maintainability system. M27 must remain a small pilot until a separate decision
authorizes official comparative evidence.

## Agent-facing output boundary

The structured result remains authoritative. The compact text rendering must:

- lead with `accepted`, `rejected`, or `incomplete` and the exact revisions;
- distinguish behavior success from architecture acceptance;
- state whether a revision was published and whether edits were committed;
- list new regressions and violations before unchanged accepted debt;
- identify the exact dependency, capability, state, control-flow, or context
  contributors;
- show the matching project rule, baseline, or exception;
- state analyzed and unchecked coverage;
- provide stable handles or query inputs for the next inspection; and
- remain deterministic and bounded, with detail available on request.

The compiler must not infer the project's preferred architecture or generate an
opaque quality score. The language creates inspectable facts, the compiler
measures them, the project policy classifies them, and the text renderer
explains the resulting structured record.

## Amp operational note

Amp is a suitable optional operator for M23 authoring or the later M27 pilot,
not a source of language semantics and not comparative evidence by itself.
According to Amp's current
[Owner's Manual](https://ampcode.com/manual), it reads repository `AGENTS.md`
guidance, keeps durable threads, supports interactive terminal use and one-shot
`amp -x` execution, and can emit streaming JSON. The same manual states that
Amp does not ask before running tools by default.

Therefore an Amp run for this repository must:

- start from a clean scoped branch or isolated worktree;
- receive an explicit milestone boundary and required checks;
- forbid compiler implementation during M23;
- forbid weakening policy, baselines, fixtures, or acceptance checks;
- record `amp --version`, selected mode, prompt, thread URL or ID when
  available, and final commit or source revision;
- review the diff and rerun repository checks before merging; and
- remain labeled non-official unless a later experiment contract freezes Amp
  as a measured operator.

Amp's threads and non-interactive mode are useful for resuming work, but the
repository remains the durable handoff. A thread link may supplement a commit;
it may not replace the roadmap, status, decision record, fixtures, or evidence.

## Consequences

- The next result tests whether AIL can reject a behaviorally correct but
  structurally harmful change and explain how to repair it.
- The work reuses immutable revisions, identity mapping, the semantic graph,
  atomic validation, and completion evidence rather than starting an unrelated
  runtime feature.
- Compact agent output becomes an explicit tested product surface while
  remaining derived from machine-readable facts.
- UC-007 remains proposed during M23. M24 through M27 are conditional planned
  milestones and do not authorize implementation or evidence collection early.
- General concurrency, outbound calls, native lowering, production adapters,
  broad language expansion, and the deferred M8 campaign remain outside this
  sequence.
- M23 may reveal that the proposed 24-operation workspace, metric catalog, or
  language subset is too broad. The correct outcome is to amend or reject the
  use case before implementation, not to hide the gap in compiler behavior.

## Alternatives considered

### Add controlled outbound calls

Outbound authority, timeouts, cancellation, and typed failure are important for
backend credibility. They require a new accepted use case and several runtime
rules at once. They are a strong later choice, but they do not use the completed
change-analysis foundation as directly or test behaviorally correct structural
regressions.

### Add bounded concurrency

Parallel work would address a major long-term semantic risk, but it combines
ordering, cancellation, resource bounds, and child-failure behavior. It is a
larger jump than the current evidence supports and should follow a smaller
accepted controlled-execution case.

### Start native lowering or production runtime work

The interpreter is sufficient for the next semantic experiment. Baseline
statistics and numeric AIL performance targets remain deferred, so production
lowering would lack an accepted success threshold.

### Expand the language broadly

Modules, packages, collections, generics, and other ordinary language features
will be necessary. Adding them as a broad campaign would increase code volume
without testing one clear project claim. M23 must identify any minimal language
prerequisite and justify it through UC-007 instead.

### Resume official agent calibration now

The feedback format and architecture task are not stable enough for official
measurement. A small non-official pilot after implementation can expose
usability failures without freezing or overstating evidence.

## Validation

This decision is implemented when:

1. M22 is complete and the roadmap activates M23 with M24 through M27 planned;
2. `docs/STATUS.md` contains the immediate M23 handoff and restart prompt;
3. project guidance states that UC-007 acceptance preparation, not compiler
   implementation, is active;
4. UC-007 and its requirements remain proposed until the M23 gate passes;
5. Amp is recorded only as an optional, versioned, non-official operator; and
6. `python3 tools/check_docs.py` passes.

Revisit the direction if M23 cannot produce deterministic good and bad fixtures,
if independent readers cannot agree on classifications, if required facts are
not analyzable in the bounded language, or if the smallest credible workspace
requires broad unrelated language and runtime expansion.
