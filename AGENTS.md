# AIL repository guidance

Before proposing language features, implementation architecture, or repository
tooling, read these documents in order:

1. `docs/README.md`
2. `docs/project-intent.md`
3. `docs/application-vision.md`
4. `docs/use-cases/README.md`
5. `docs/requirements/README.md`
6. `docs/design-direction.md`
7. `docs/architecture-health.md`
8. `docs/spec-review.md`
9. `docs/roadmap.md`
10. `docs/stack-evaluation.md`

Before implementation work, also read `docs/STATUS.md` and the active milestone
in `docs/roadmap.md`.

## Thesis guardrails

AIL is an executable programming language designed for software agents as its
primary authors and operators. It optimizes the total work and risk required to
make a correct, reviewable change, rather than source brevity or human typing
convenience in isolation.

Preserve these distinctions:

- Plausible code generation is cheap; context discovery, consequence analysis,
  validation, repair, and regression control are the dominant costs.
- Canonical source is the durable artifact. The compiler's structured semantic
  model is the primary interface for inspection and change.
- The agent protocol exposes guarantees created by the language; it must not
  merely compensate for ambiguous language semantics.
- Human authorship ergonomics are secondary, but human auditability is a hard
  requirement.
- Token efficiency means reducing total task information and repair work, not
  minimizing source tokens at any cost.
- AIL must be evaluated against existing languages with their normal compiler
  and language-server tooling, not against raw text editing.
- The long-term destination is a default greenfield language for agent-built
  application software. Backend services and workers are the first validation
  wedge, not the permanent limit of the language.
- AIL is not optimized for unaided human typing, but it must not become obscure
  or unauditable merely to appear agent-native.
- Architectural health is reported as primitive, revision-bound semantic facts.
  Project policy decides which facts warn or deny; do not turn one complexity
  threshold or composite score into universal language semantics.
- Evaluate responsibility at symbol and aggregate semantic scopes. Splitting a
  large function into helpers is not an architectural improvement when
  authority, state, dependencies, and review context remain concentrated.

## Current project state

The repository is in design and validation. The first job-service use cases and
requirements are accepted. `docs/STATUS.md` names the active benchmark
milestone. UC-007 and the architectural health manifest are proposed future
scaling work and do not expand the active milestone until accepted.

Behavior examples at this stage are non-normative and must not establish syntax.

The narrow normative core specification, canonical fixtures, and minimal
transport-independent semantic protocol contract follow the use-case and
requirements gate.

Do not add a production source tree or root package manager until the core spike
fixture has been written, the candidate prototypes have been evaluated, and the
implementation-stack decision has been accepted.

Treat prototypes as disposable evidence. Do not allow prototype choices or
examples to become normative semantics accidentally.

## Milestone workflow

- `docs/roadmap.md` owns milestone scope, dependencies, non-scope, and exit
  criteria.
- `docs/STATUS.md` names the active milestone and contains the immediate handoff
  for the next agent.
- Work only within the active milestone unless the user changes scope.
- Use a scoped `codex/` branch for milestone implementation when the worktree
  and coordinator allow it. Do not disturb unrelated or user-owned changes.
- Commit coherent checkpoints that build and pass the milestone's focused
  checks.
- Add executable checks for every behavior delivered by a milestone.
- Record consequential or expensive-to-reverse choices in `docs/decisions/`.
- A milestone is complete only when its exit criterion, focused checks, and
  full repository checks pass.
- The repository-wide documentation and local-link check is
  `python3 tools/check_docs.py`.
- When a milestone completes, update its status in `docs/roadmap.md`, advance
  `docs/STATUS.md`, and leave a concise handoff for the next agent.
- Keep detailed requirements in their authoritative documents. Milestones link
  to those documents instead of copying them.

## Communication style

Write for an experienced software engineer using plain workplace English.
Apply this style in agent conversations, review summaries, public
documentation, READMEs, proposals, and decision records.

- Start with the concrete system, behavior, decision, or outcome.
- Prefer familiar engineering terms over project-specific terminology.
- Explain a new term in ordinary language before using it as shorthand.
- Use examples to introduce abstractions.
- Keep sentences direct and make the practical consequence clear.
- Preserve technical precision, but do not make the reader translate layers of
  abstract terminology to understand what the software does.
- Write so an engineer can review the behavior, tradeoff, and next decision
  without first learning an AIL-specific vocabulary.

## Test for proposed decisions

A proposal should explain:

1. which accepted use case and requirement it serves;
2. which agent change cost it reduces;
3. why the behavior belongs in the language rather than only in tooling;
4. how the compiler exposes the resulting semantics;
5. how the behavior becomes deterministic and conformantly testable; and
6. how a human can audit the resulting program and change.
