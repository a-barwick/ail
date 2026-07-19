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
| Normative specification | Planned | What exact source, static semantics, dynamic semantics, diagnostics, and protocol behavior are required? | Numbered accepted rules and conformance fixtures |
| Implementation evidence | [stack-evaluation.md](stack-evaluation.md) and `../prototypes/` | Can a design be implemented, and in which stack? | Evidence only; cannot create semantics |

Architecture decision records under `decisions/` explain why consequential
choices were made. Reviews such as [spec-review.md](spec-review.md) assess the
state of another artifact at a point in time; they do not silently update that
artifact.

The [roadmap](roadmap.md) sequences work across layers. M0–M13 is its sole
operational sequence; the long-range outlook is not active work. A roadmap entry
does not make the content of a planned artifact normative.

The [current status](STATUS.md) names the active milestone and gives the next
agent its immediate handoff.

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
and its frozen language-independent public JSON fixture corpus are accepted.
The active task is M2: freeze the benchmark harness and task contract.
Architectural regression control is a separate proposed scaling use case and
feature specification; it does not expand the active slice until reviewed and
accepted.

There is no accepted AIL syntax or normative language specification yet.
