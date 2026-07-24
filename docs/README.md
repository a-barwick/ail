# AIL documentation model

Status: **Project guidance**

Purpose: keep the project's vision, application goals, requirements, language
design, normative semantics, and implementation evidence distinct.

AIL documentation is intentionally layered. A statement can be important without
being a language rule, and an example can clarify a use case without establishing
syntax.

## Layers

| Layer | Current artifact | Question answered | Authority |
| --- | --- | --- | --- |
| Project thesis | [project-intent.md](project-intent.md) | Why should AIL exist, and what must it prove? | Enduring direction; non-normative for program behavior |
| Application vision | [application-vision.md](application-vision.md) | Where should AIL be used, by whom, and what tradeoff should it change? | Product direction; non-normative |
| Use cases | [use-cases/README.md](use-cases/README.md) | Which systems and agent changes must AIL support first? | Scenario evidence; non-normative |
| Derived requirements | [requirements/README.md](requirements/README.md) | What observable capabilities and constraints follow from the use cases? | Proposed until explicitly accepted |
| Benchmark policy | [benchmarks/README.md](benchmarks/README.md) | How are AIL and strong baselines compared fairly? | Accepted measurement policy and language-independent test data; non-normative for AIL |
| Language design | [design-direction.md](design-direction.md) and [architectural health manifest](architecture-health.md) | Which language and compiler ideas may satisfy the requirements? | Design input; non-normative |
| Normative specification | [M11 core contract](../specs/README.md) | What exact source, static semantics, diagnostics, and protocol behavior are required? | Proposed rules and fixtures binding on the first Rust compiler milestones; not yet accepted as the broader AIL core |
| Implementation architecture | [ADR 0004](decisions/0004-rust-compiler-stack.md) | Which stack owns compiler semantics? | Rust is authoritative through the first production backend |

Architecture decision records under `decisions/` explain why consequential
choices were made. Reviews such as [spec-review.md](spec-review.md) assess the
state of another artifact at a point in time; they do not silently update that
artifact.

The [roadmap](roadmap.md) sequences work across layers. M14–M17 are the completed
authoritative compiler sequence through the deterministic interpreter. M18
selected compiler-guided UC-003 priority evolution as the next validation
slice. M19 accepted its bounded contract, M20 implemented its semantic graph
and impact query, and M21 completed the atomic change and evidence loop. M22
selected architectural regression control as the next direction in
[ADR 0006](decisions/0006-prepare-architectural-regression-control.md). M23
accepted UC-007 and its requirements through a frozen, independently reviewed
acceptance package. M24 now accepts the bounded architectural regression
contract. M25 implements its single-revision snapshot and compact rendering,
and M26 implements cross-revision policy enforcement and atomic publication.
M27 is active for a bounded non-official usability pilot. A roadmap entry does
not make planned behavior normative.

The [current status](STATUS.md) records the active milestone, or states that the
next planned milestone has not been started, and gives the next agent its
immediate handoff.

The deferred [M8 execution plan](m8-execution-plan.md) records how the baseline
calibration campaign could be resumed. M8a through M8f produced reusable
infrastructure and non-official pilots; M8g through M8o are not active work.
The plan is not benchmark policy or language semantics. The
[M8 agent experiment contract](decisions/0002-m8-agent-experiment-contract.md)
records the fixed treatment used by those tasks.

## Required flow

Material decisions should be traceable through these layers:

```text
project thesis
  -> application vision
  -> concrete use case
  -> numbered requirement
  -> language or protocol design
  -> normative rule
  -> conformance fixture
  -> implementation evidence
```

Not every thesis statement produces a language feature. Some produce tooling,
runtime, standard-library, deployment, benchmark, or governance requirements.

Benchmark fixtures test accepted application behavior. They do not establish AIL
syntax or language rules.

## Normativity

- Project intent and application vision constrain project direction but do not
  define program behavior.
- Use-case examples describe desired outcomes. They must not accidentally choose
  AIL syntax or semantics.
- Design documents compare and motivate possible solutions.
- Only accepted, numbered specification rules and their conformance fixtures are
  normative for language or protocol behavior.
- Prototypes test written rules. A successful prototype does not make its
  incidental behavior normative.
- Architecture decisions identify their scope and do not override unrelated
  layers implicitly.

When documents disagree, record the disagreement explicitly. Do not reconcile it
by treating an example or prototype as the latest specification.

## Example labels

Every example must use one of these labels:

- **Behavior illustration:** language-independent scenario or pseudocode.
- **Illustrative AIL:** proposed syntax with no normative authority.
- **Proposed fixture:** candidate canonical source tied to proposed rules.
- **Conformance fixture:** accepted canonical source and expected outcomes tied
  to accepted numbered rules.

## Current position

The job-service reference workload, its two use cases, its first requirements,
its frozen public JSON fixtures, and its language-independent benchmark harness
and task contract are accepted. The Rust, Go, Python, and TypeScript baselines
are complete. M7 froze cross-baseline parity, the public and private benchmark
inputs, and eight deterministic answer-free task starts. M8a through M8f built
calibration, replay, measurement, and provider-readiness infrastructure, but no
official evidence exists. ADR 0003 defers the remaining calibration campaign,
numeric benchmark targets, and illustrative syntax variants because they do not
inform the compiler-stack decision. M11 completed the proposed five-construct
shared contract. ADR 0004 selects Rust directly and supersedes the M12/M13
comparison path. M14 delivered lossless syntax, deterministic recovery, and
canonical formatting. M15 delivered name resolution, local inference,
capability checking, and structured diagnostics in the authoritative Rust
compiler. M16 delivered immutable revisions, deterministic revision-scoped
inspection, validated rename, canonical edits, and identity mapping. M17
delivered the accepted bounded runtime rules, canonical reference service,
deterministic revision-scoped interpreter, and a locked AIL runner that matches
all 37 public job-service fixtures. M18 selected compiler-guided UC-003 priority
evolution in [ADR 0005](decisions/0005-next-validation-slice.md). M19 froze the
bounded schema-identity, impact-query, transaction, semantic-diff, and
completion-evidence contract. M20 implemented its ordered source-set semantic
graph and exact impact query. M21 implemented the atomic R1-to-R2 transaction
and completion evidence. M22 selected architectural regression control as the
next scaling direction. M23 froze its acceptance evidence, including the
starting workspace, `CancelJob` behavior, good and seeded bad changes, project
policy, metrics, expected structured and compact text, baseline comparison, and
budgets. Two independent readers classified every candidate identically, so
UC-007 and its requirements are accepted. No architectural-health compiler
compiler behavior is active yet; only the bounded M24 normative contract is
accepted.

There is no accepted broad AIL syntax or normative language core yet. The M11
subset is the fixed conformance boundary for the first Rust compiler slices.
