# ADR 0007: Prepare iterative service evolution next

- Status: Accepted
- Date: 2026-08-07
- Owners: project maintainers
- Documentation layer and scope: validation-slice selection and roadmap

## Context

M23 through M27 established and exercised one bounded architectural-regression
change. The compiler can reject a behaviorally correct `CancelJob` change that
enlarges a dispatch hotspot or moves store authority into transport, and one
non-official operator used that evidence to repair the seeded regression.

That single change does not test the project's larger maintenance claim. Code
generation is cheap; the expensive risk appears when ordinary requirements
accumulate and each new agent must recover earlier decisions, update scattered
consumers, diagnose failures, and avoid concentrating more responsibility in
already attractive shortcuts.

A useful comparison therefore needs a mature service and a cumulative sequence
of changes. The intended scale is approximately 2,000 natural source lines
across approximately five to eight files, followed by six to eight product
requirements applied in fresh agent contexts. Those figures describe the
desired pressure, not quotas.

The current accepted AIL runtime is too narrow to assume this workload can be
implemented honestly. Direct calls, modules, bounded iteration, collections,
or additional capability behavior may be needed, but adding ordinary language
features before the workload is concrete would repeat the conventional feature
progress the successor gate rejects.

## Successor-gate decision

The direction addresses an unresolved thesis uncertainty:

> Does AIL keep cumulative context discovery, consequence analysis, validation,
> repair, regression, and review work under control as a service evolves through
> repeated fresh-context changes?

The affected cost is cumulative agent change work, not source generation. This
requires benchmark, language, compiler, and protocol work only where strong
baseline workspaces and the frozen change sequence show that AIL lacks a
necessary executable behavior or compiler-visible fact.

The later discriminating comparison retains every initiated complete or terminal
trajectory through the same ordered requirement sequence in AIL, Rust, Go,
Python, and TypeScript, using each language's normal strong tools. Correctness
and human auditability remain admission gates. The result must support go,
revise, or stop rather than assuming AIL will win.

This satisfies the successor gate as comparator-enabling work. It does not make
the proposed use case, its requirements, illustrative changes, or future AIL
features accepted.

## Decision

Select **iterative service evolution** as the next validation direction and
activate M28 as its acceptance package.

M28 must freeze a language-independent mature service, six to eight cumulative
requirements, per-change and cumulative correctness evidence, architecture
expectations, fresh-context rules, strong-baseline plan, measurements, budgets,
and stop conditions. It may implement fixtures and dependency-free acceptance
tooling. It may not implement AIL language or compiler behavior.

The bounded path to the north-star simulation is:

1. **M28 — Iterative-evolution acceptance package.** Make the proposed UC-008
   service, sequence, oracles, evidence, baseline plan, scale hypothesis, and
   independent classification concrete. Accept, revise, or reject UC-008 and
   candidate requirements.
2. **M29 — Strong baseline workspaces and trajectories.** Conditional on M28,
   build idiomatic Rust, Go, Python, and TypeScript starting services and
   reference revisions for every change. Freeze parity, normal tools, task
   starts, hidden checks, and observed natural scale. Return to M28 or stop if
   idiomatic implementations do not create the intended pressure naturally.
3. **M30 — AIL gap selection.** Compare the accepted sequence with the
   authoritative compiler and select only the semantic and protocol gaps that
   prevent a natural AIL implementation. Record a bounded contract and
   implementation sequence or stop if broad feature parity is required before
   useful evidence.
4. **M31 — Iterative-workload contract.** Conditional on M30, accept grammar,
   static and runtime semantics, diagnostics, semantic facts, and conformance
   fixtures only for the selected gaps. This milestone adds no Rust behavior.
5. **M32 — Authoritative compiler implementation.** Implement the accepted
   frontend, semantic, protocol, and interpreter behavior with executable
   checks. The interpreter remains the semantic oracle; no native backend is
   added.
6. **M33 — Canonical AIL service and cumulative evidence.** Build and freeze the
   natural AIL starting workspace, every accepted reference revision, behavior
   and architecture evidence, task starts, and full-sequence verifier.
7. **M34 — Non-official full-trajectory pilots.** Run a small number of initiated
   complete or terminal fresh-context sequences to find task, compiler-output,
   measurement, and evidence defects. Pilots may cause revision but support no
   comparative claim.
8. **M35 — Comparative campaign readiness.** Freeze the measured operator,
   environments, ordering, trial counts, safety limits, target-calculation
   formula, decision rule, and evidence closure after the pilots pass.
9. **M36 — Official iterative comparison.** Conditional on a separate launch
   decision after M35, first run and verify the strong baseline trajectories,
   compute and lock AIL targets using the preregistered formula, obtain a
   separate continuation decision, and only then run AIL trajectories.

Only M28 is active. Later milestones are conditional map points. Their names do
not authorize early implementation, evidence collection, or feature selection.

## Near-term operating plan

The next few days should produce executable M28 acceptance infrastructure, not
more abstract design:

- freeze the service boundary and operation inventory;
- make each candidate requirement precise enough to build behavior fixtures;
- define separate reference and measured revision chains plus fresh-context
  task envelopes and terminal-trajectory accounting;
- implement a dependency-free package checker with rejection mutations;
- record natural-scale and architecture-review criteria without enforcing line
  or file quotas; and
- obtain two independent classifications before accepting UC-008.

The following weeks should remain implementation-heavy: baseline services and
reference trajectories first, then only the AIL contract and compiler work the
accepted sequence demonstrates is necessary. Planning documents are gates and
handoffs, not substitutes for those artifacts.

## Evidence and claim boundaries

- M28 and M31 produce conformance inputs only after acceptance.
- M29 and M33 produce mechanism and comparator evidence, not agent-efficiency
  evidence.
- M34 produces non-official usability evidence only.
- M35 produces campaign-readiness evidence only.
- Only M36 can produce official comparative evidence, and only a valid locked
  result can support a claim about lower cumulative agent change cost.

No milestone may report source volume, feature count, compiler size, native
execution, or one successful trajectory as project validation.

## Non-scope

This decision does not authorize:

- accepting UC-008 or its requirements before M28 passes;
- enforcing 2,000 lines or five to eight files;
- choosing AIL syntax from the illustrative requirement sequence;
- broad language feature parity;
- implementing later milestones during M28;
- LLVM, native generation, production runtime work, or self-hosting;
- changing M8 or launching its deferred campaign;
- official trials before M35 and a separate launch decision; or
- weakening existing frozen fixtures, architecture policy, or historical
  completion evidence.

## Consequences

- The next implementation work builds executable benchmark acceptance tooling
  rather than extending the Rust compiler.
- Natural project scale and cumulative interactions become reviewable evidence
  instead of line-count or file-count targets.
- Fresh agent contexts make canonical source and compiler evidence, rather than
  conversation memory, responsible for preserving project knowledge.
- Strong baseline implementation precedes AIL feature selection, so ordinary
  language additions must be justified by observed workload gaps.
- The roadmap gains a conditional path through an official comparison without
  authorizing later milestones early.
- The path is deliberately stoppable at M28, M29, M30, M34, M35, and between
  the two M36 phases if the scenario, natural baseline scale, language cost,
  pilot evidence, campaign design, or official baseline evidence does not
  support continued investment.
- Existing M0–M27 contracts, fixtures, evidence, and historical decision meaning
  remain unchanged.

## Alternatives considered

### Run an official `CancelJob` comparison next

This would test the completed bounded mechanism, but one change is unlikely to
expose cumulative context growth, repeated schema consequences, or the tendency
for reasonable product work to enlarge central responsibilities. UC-008 uses
the completed architecture machinery in a more discriminating maintenance
trajectory.

### Expand the language first

Direct calls, modules, collections, and additional capabilities are plausible
needs. Selecting them before the service and sequence are frozen would turn
ordinary language completeness into the roadmap. M28 and M29 must supply the
evidence that M30 uses to select gaps.

### Prepare native execution

The interpreter remains sufficient for semantic and change-loop evidence. No
accepted deployment envelope or measured interpreter deficiency requires a
backend, so native execution remains outside this campaign.

### Use one continuing agent conversation

Conversation memory would hide whether project decisions are recoverable from
canonical source and compiler evidence. Fresh contexts make the durable
repository carry that burden. Multiple full trajectories later measure path
dependence rather than assuming one sequence is representative.

## Validation

This decision is implemented when:

1. UC-008 exists as Proposed with its natural-scale, cumulative-change, strong
   baseline, fresh-context, evidence, and stop boundaries;
2. the roadmap activates only M28 and records M29 through M36 as conditional;
3. `docs/STATUS.md` gives an executable M28 handoff;
4. project guidance distinguishes M28 acceptance tooling from AIL compiler
   implementation;
5. no accepted specification, fixture, or historical artifact changes; and
6. `python3 tools/check_docs.py` passes.
