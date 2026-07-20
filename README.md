# AIL

AIL is an experimental deterministic programming language designed for software
agents as its primary authors and operators. It aims to minimize the total work
and risk required for an agent to make a correct, reviewable change. Canonical
text is the durable artifact; the compiler's typed semantic model is the primary
interface for navigation, diagnostics, structural edits, and impact analysis.

Human authorship ergonomics are secondary. Human auditability is a hard
requirement.

The project has entered compiler implementation. Rust is the authoritative
compiler stack, the root Cargo workspace is production code, and the frozen
M11 contract constrains its first slices. The repository also contains
language-independent benchmark data and dependency-free validation tooling.

## Project thesis

Existing languages were largely designed around the constraints of human
programming. As agents make plausible code generation inexpensive, the dominant
costs shift toward context discovery, consequence analysis, validation, repair,
process control, and regression prevention.

AIL treats that complete change loop as the unit of language design:

```text
total_agent_change_cost =
    generation_work
  + context_discovery_work
  + consequence_analysis_work
  + validation_work
  + diagnostic_work
  + repair_work
  + regression_risk
```

The detailed thesis, its limits, and the claim the project must eventually prove
are in [docs/project-intent.md](docs/project-intent.md).

## Application vision

AIL's intended destination is a default greenfield language for software
primarily implemented and maintained by agents. It should combine predictable
compiled execution and strong static semantics with compact, locally inferred
source and a fully queryable compiler model.

The first validation wedge is backend services and workers. This is a proving
ground, not a permanent restriction on the language. See
[docs/application-vision.md](docs/application-vision.md).

The current direction combines:

- one canonical textual representation;
- fixed executable semantics and explicit nondeterministic inputs;
- explicit public types, effects, capabilities, and guarantees;
- local inference with a compiler-provided elaborated view;
- typed incomplete programs and structured diagnostics;
- semantic context slicing, diffs, and transactional edits;
- proposed revision-bound architectural health manifests and enforceable
  project policy;
- deterministic replay and controlled concurrency; and
- native compilation with optional compatible source emission.

See [docs/design-direction.md](docs/design-direction.md) for the captured design
direction, [docs/architecture-health.md](docs/architecture-health.md) for the
proposed architectural-regression feature, and
[docs/spec-review.md](docs/spec-review.md) for the initial review.

## Repository status

The job-service workload, use cases, requirements, public JSON fixtures, and
benchmark harness/task contract are accepted. The Rust, Go, Python, and
TypeScript baselines and the M11 five-construct contract are complete. The
active compiler milestone is tracked in [docs/STATUS.md](docs/STATUS.md).

The operational delivery sequence is defined in
[docs/roadmap.md](docs/roadmap.md).
The immediate handoff is in [docs/STATUS.md](docs/STATUS.md).
The Rust decision is recorded in
[ADR 0004](docs/decisions/0004-rust-compiler-stack.md).

## Working in this repository

The repository contains documentation, language-independent benchmark tooling,
and four baseline implementations:

```text
AGENTS.md          Repository guidance and thesis guardrails for agent work
benchmarks/        Frozen cases, schemas, tools, and baseline implementations
docs/
  README.md        Documentation layers, authority, and traceability
  STATUS.md        Active milestone and next-agent handoff
  application-vision.md
  architecture-health.md
  benchmarks/      Shared benchmark rules and language-independent test format
  decisions/       Architecture decision records
  design-direction.md
  project-intent.md
  requirements/    Numbered requirements and their status
  roadmap.md
  spec-review.md
  stack-evaluation.md
  use-cases/       Concrete application and agent-change scenarios
compiler/          Authoritative Rust compiler crates
tools/             Dependency-free repository documentation checks
```

The root Cargo workspace and `compiler/` tree are authoritative. Keep benchmark
baselines and any future experiments isolated from compiler semantics.

## Project policy

- The canonical formatter is part of the language definition.
- Examples are non-normative until linked to a numbered semantic rule.
- Every externally observable behavior must eventually have a specified
  deterministic meaning.
- Changes to public semantics require a decision record.
- The license has not yet been selected; all rights remain with the copyright
  holder until one is added.
