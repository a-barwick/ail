# UC-008 — Iterative service evolution

Status: **Proposed**

Documentation layer: concrete application scenario. This record is
language-independent and non-normative for AIL syntax or semantics.

[ADR 0007](../decisions/0007-prepare-iterative-service-evolution.md) originally
selected a bounded acceptance package for this direction. A maintainer scope
correction redirected M28 to AIL language composition, so this use case and its
acceptance work remain proposed and inactive.

## Actor and desired outcome

A sequence of agents maintains a service through repeated product changes. Each
agent receives one new requirement in a fresh context and starts from the
accepted revision produced by the previous change. The durable repository and
its normal compiler tools, rather than conversation memory, must provide the
context needed to work safely.

The desired outcome is a service whose behavior remains correct and whose
change cost, authority boundaries, dependencies, state ownership, and review
surface remain controlled as requirements accumulate.

This use case tests a trajectory, not one edit. A language can perform well on
the first change and still fail if later changes require steadily more source
reconstruction, repair, or architectural cleanup.

## Reference system and natural scale

The reference system is a mature, transport-independent job service. It should
naturally require approximately 2,000 lines across approximately five to eight
source files in ordinary strong baseline implementations. These figures are
calibration observations, not acceptance thresholds:

- source must not be padded to reach a line count;
- responsibilities must not be split unnaturally to reach a file count;
- an idiomatic implementation outside either range is not rejected for that
  reason; and
- the accepted package must explain why the service is large enough to expose
  repeated-change costs without becoming a framework benchmark.

The starting service contains multiple public operations, persistent job state,
validation and compatibility rules, closed domain outcomes, explicit store and
other external capabilities, transport adaptation, domain handlers, response
projection, behavior tests, and a declared architecture policy.

The first acceptance package must keep production HTTP, a real database,
deployment infrastructure, and unrelated framework behavior outside the system
boundary. Deterministic host capabilities may stand in for external systems.

## Cumulative requirement sequence

A future acceptance effort must freeze six to eight requirements before measured
trials. The sequence must be ordinary product work that creates natural pressure
on existing responsibilities; it must not tell an agent to create or avoid a
particular source shape.

Candidate changes for that future acceptance effort include:

1. record cancellation reasons and audit facts;
2. retry a failed job under explicit transition rules;
3. schedule execution using supplied time authority;
4. apply tenant-specific policy without moving domain decisions into transport;
5. cancel a bounded group of jobs with fixed ordering and outcome rules;
6. send status notifications with explicit authority and failure behavior;
7. revise priority compatibility across public and stored schemas; and
8. add an administrative override without enlarging an existing dispatch
   hotspot.

This list is behavior illustration, not accepted scope or AIL syntax. A future
acceptance package may revise, replace, reorder, or reduce it. Every retained
change must define:

- the requirement text shown to the agent;
- starting and expected observable behavior;
- domain failures and language/runtime faults;
- state transitions and ordered external effects;
- schema and compatibility consequences;
- architecture boundaries and accepted existing debt;
- public and held-back correctness checks;
- human-review evidence; and
- the exact predecessor in the frozen reference trajectory that proves the
  change is satisfiable.

## Fresh-context maintenance protocol

The benchmark keeps two different revision chains:

1. The **reference trajectory** contains frozen known-good revisions that prove
   the sequence is satisfiable and define oracle results. After the common
   starting revision, it is never used to reset a measured run.
2. A **measured trajectory** starts only from the frozen initial revision. Each
   later change starts from the actual accepted revision produced by the
   previous change in that same trajectory.

Each measured change starts in a fresh agent context. The agent receives its
trajectory's current repository, the new requirement, normal project
instructions, and the tools allowed by the experiment contract. It does not
receive the previous agent's conversation, an answer-bearing change summary, or
the corresponding reference revision.

After a change passes the declared correctness and review gates, its complete
revision becomes the starting point for the next change. Failed or incomplete
changes do not silently enter the sequence. A failed, timed-out, or incomplete
change terminates that trajectory, remains in every applicable denominator, and
marks later changes as not reached. A replacement or makeup run receives a new
trajectory identity and does not erase the original.

Official evidence will retain every initiated complete or terminal trajectory.
One early design choice can make every later change easier or harder, and that
path dependence is part of the result rather than noise to erase.

## Natural failure modes

The sequence should expose ordinary maintenance failures without requiring them:

- a dispatcher or orchestration function accumulates unrelated branches;
- transport code gains domain decisions, state ownership, or external
  authority;
- schema producers and consumers drift apart;
- validation or compatibility logic is duplicated;
- a new outcome is not handled exhaustively;
- capability calls occur in the wrong order or on invalid input;
- helpers make functions smaller while responsibility remains concentrated in
  the same module or dependency component;
- dependency cycles or broad fan-in appear;
- the minimum context needed to review a change grows; or
- later agents repeatedly rediscover decisions that should be compiler-visible.

The benchmark must not equate a file count, line count, or one complexity score
with architecture quality. Behavior, authority, state, dependencies, aggregate
responsibility, context, and coverage remain separate facts.

## Observable results at every change

For each accepted revision, the evidence records:

### Correctness

- public and held-back behavior results;
- preserved prior behavior;
- schema and compatibility results;
- state transitions and ordered capability calls;
- static diagnostics and runtime faults; and
- complete analyzed and unchecked coverage.

### Agent work

- source and structured compiler context delivered;
- semantic queries, searches, and source reads;
- edits, validation attempts, and incomplete validations;
- repair cycles and diagnosed causes;
- elapsed work under the frozen experiment contract; and
- context that had to be reconstructed from earlier decisions.

### Architectural development

- new, enlarged, reduced, unchanged, and removed hotspots;
- dependency, capability, effect, and state changes;
- responsibility concentration at executable and aggregate scopes;
- review-context growth or reduction;
- policy and baseline changes; and
- exceptions or incomplete analysis.

### Human review

- the requirement and observable behavior changed;
- affected public and stored contracts;
- new authority, effects, state, and dependencies;
- accepted existing debt versus change-introduced regressions;
- canonical textual and semantic changes; and
- evidence sufficient to approve or reject the revision without trusting an
  undocumented agent explanation.

## Strong baseline

Rust, Go, Python, and TypeScript implementations use their normal high-quality
compiler, language-server, formatter, linter, test, refactoring, and static
analysis tools. Their source should be idiomatic rather than forced into AIL's
file layout or internal architecture.

The shared contract fixes behavior, authority, compatibility, architecture
policy, task information, and evidence requirements. It does not require
identical source structures. Language-specific checks may enforce equivalent
facts using the strongest normal mechanism available to that ecosystem.

A future acceptance package must identify the concrete tool and version plan.
The deferred baseline milestone would own workspaces and reference trajectories
only after that package is activated and passes.

## Why this may require AIL work

Existing AIL semantics are intentionally too narrow to assume this service can
be expressed naturally. The sequence may require direct AIL calls, modules,
bounded iteration, collections, additional capabilities, or other behavior.
The maintainer independently selected direct calls and modules for M28. The
remaining items are not selected language requirements.

After baseline evidence exists, a separate gap-selection milestone may propose
the smallest AIL semantic and protocol slice needed for the accepted sequence.
Each addition must name the requirement change that needs it and the agent work
or risk it removes.

## Measurable success question

The later comparative question is:

> Across repeated fresh-context requirement changes, does an agent using AIL
> and its compiler require less cumulative context and repair work and introduce
> fewer behavioral or architectural regressions than the same agent using
> strong mainstream languages with their normal tools?

The comparison reports each change and the cumulative trajectory. It must not
hide an expensive early failure behind a successful final revision or infer
success from source size alone.

## Inactive acceptance gate

UC-008 is ready to drive requirements only if two independent readers can use
the frozen package to agree on:

1. the starting service boundary, scale hypothesis, and language-independent
   pressure criteria;
2. the exact ordered requirement sequence;
3. each change's behavior, state, effects, compatibility, and architecture
   expectations;
4. the fresh-context and revision-carry-forward protocol;
5. correctness, human-review, and stop conditions;
6. strong baseline tools and equivalent task information;
7. measurements for individual and cumulative change cost;
8. which AIL gaps are observations rather than accepted features; and
9. the go, revise, or stop result.

The gate stops if the changes do not interact meaningfully, equivalent strong
baselines cannot be defined, expected classifications depend on preferred
source layout, or a credible AIL path would require broad feature parity before
producing evidence. A future baseline effort must return to its owning gate or
stop if idiomatic baseline implementations reach the planning range only through
padding or artificial file splitting, or do not create the intended maintenance
pressure.

## Status and derived requirements

No requirement is accepted from this proposed use case yet. Candidate
requirements still depend on an activated complete package and independent
review.

## Explicit exclusions

UC-008 does not by itself authorize:

- AIL syntax or compiler implementation;
- a direct-call, module, collection, concurrency, or capability design;
- native lowering, LLVM, generated target source, or production deployment;
- a universal architecture policy;
- official agent trials;
- numeric AIL success targets before strong baseline evidence; or
- a claim that a 2,000-line service is representative merely because of its
  size.
