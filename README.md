# AIL

AIL is an experimental deterministic programming language designed for agents to
author, inspect, debug, and refactor. Canonical text is the durable artifact;
the compiler's typed semantic model is the primary interface for navigation,
diagnostics, structural edits, and impact analysis.

The project is in design and validation. No implementation stack has been
selected, and this repository intentionally contains no framework, package
manager, or generated compiler skeleton yet.

## Design thesis

AIL aims to minimize the total model work required to produce a correct
executable program:

```text
total_agent_cost =
    generation_tokens
  + context_tokens
  + diagnostic_tokens
  + repair_tokens
  + regression_risk
```

The current direction combines:

- one canonical textual representation;
- fixed executable semantics and explicit nondeterministic inputs;
- explicit public types, effects, capabilities, and guarantees;
- local inference with a compiler-provided elaborated view;
- typed incomplete programs and structured diagnostics;
- semantic context slicing, diffs, and transactional edits;
- deterministic replay and controlled concurrency; and
- native compilation with optional compatible source emission.

See [docs/design-direction.md](docs/design-direction.md) for the captured design
direction and [docs/spec-review.md](docs/spec-review.md) for the initial review.

## Repository status

The first milestone is a small executable language core, not a production
backend:

1. Define the 20–30 core constructs and their semantics.
2. Write the canonical grammar and formatting rules.
3. Specify the compiler's versioned agent protocol.
4. Validate the riskiest implementation choices with disposable prototypes.
5. Select the implementation stack from evidence and record the decision.

The proposed delivery sequence is in [docs/roadmap.md](docs/roadmap.md).
Stack evaluation criteria and candidate spikes are in
[docs/stack-evaluation.md](docs/stack-evaluation.md).

## Working in this repository

Documentation is the only source artifact at this stage:

```text
docs/
  decisions/       Architecture decision records
  design-direction.md
  roadmap.md
  spec-review.md
  stack-evaluation.md
prototypes/        Disposable, isolated implementation spikes
```

Do not introduce a root package manager or production source tree as part of a
prototype. A stack decision should be accepted first so that experimental
dependencies do not become accidental architecture.

## Project policy

- The canonical formatter is part of the language definition.
- Examples are non-normative until linked to a numbered semantic rule.
- Every externally observable behavior must eventually have a specified
  deterministic meaning.
- Changes to public semantics require a decision record.
- The license has not yet been selected; all rights remain with the copyright
  holder until one is added.
