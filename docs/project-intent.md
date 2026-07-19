# AIL project intent

Status: **Design foundation**

Purpose: state the project thesis, the change in programming economics that
motivates AIL, and the claim the project must eventually prove.

Documentation layer: project thesis. Application categories, workloads, and
benchmark scenarios belong in [application-vision.md](application-vision.md).
See the [documentation model](README.md).

## Thesis

AIL is an executable programming language designed for software agents as its
primary authors and operators.

Existing programming languages were largely designed within the constraints of
human programming. They optimize, in different proportions, for concerns such as
ease of typing, local readability, memorability, manual control over resources,
and coordination among human developers.

As software agents become capable of producing large amounts of plausible code,
generation is no longer the dominant bottleneck. The expensive work shifts
toward:

- discovering the context relevant to a change;
- understanding effects, authority, and downstream consequences;
- validating static and dynamic behavior;
- coordinating consistent changes across a program;
- diagnosing and repairing failures;
- controlling processes, concurrency, and resource growth; and
- preventing regressions.

AIL attempts to improve the language and compiler around that changed cost
model. Its primary design question is:

> What language and compiler contract minimizes the total work and risk required
> for an agent to make a correct, reviewable change?

## Optimization target

AIL optimizes the complete change loop, not source brevity or first-pass code
generation in isolation:

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

These terms do not yet constitute a single numerical metric. The benchmark
method must define their units, weighting, workloads, correctness oracle, and
reproducibility rules before the thesis can be evaluated quantitatively.

Additional syntax can reduce total cost when it makes an important boundary or
guarantee locally visible. Conversely, compact syntax is not efficient when it
causes an agent to retrieve more context, infer hidden behavior, or perform more
repair iterations.

Token efficiency therefore means reducing the total information processed to
complete a correct change. It does not mean minimizing source tokens at the
expense of semantics, safety, or auditability.

## Why this may require a language

Many agent-oriented improvements can and should be made in tooling for existing
languages. Semantic navigation, structured diagnostics, context selection, and
safe refactors are not by themselves sufficient justification for a new
language.

AIL is justified only if foundational language choices provide material
advantages that compatibility-bound tooling cannot reliably add. Candidate
advantages include:

- one canonical textual representation for each construct;
- explicit public types, effects, capabilities, errors, and guarantees;
- deterministic and replayable observable semantics;
- constrained authority and the absence of ambient capabilities;
- structured concurrency and visible resource growth;
- typed incomplete programs;
- stable, structured diagnostic categories;
- mechanically complete impact analysis; and
- transactional structural transformations.

The compiler's agent-facing protocol exposes semantics created by the language.
It is not a substitute for missing or ambiguous language semantics. Canonical
text remains the durable artifact, while the compiler's typed semantic model is
the primary interface for inspection, navigation, diagnosis, and change.

## Design consequences

The thesis implies several connected design choices:

- Canonical syntax reduces generation entropy, representational aliases, and
  irrelevant diff noise.
- Explicit public contracts reduce the amount of surrounding implementation an
  agent must load before using or changing a unit.
- Local inference keeps implementations compact, while an elaborated compiler
  view makes inferred facts inspectable.
- Effects and capabilities expose consequences, nondeterministic inputs, and
  authority.
- Fixed observable semantics make executions reproducible and diagnoses
  portable.
- Structured diagnostics reduce prose interpretation and repair cascades.
- Semantic context slices replace file-oriented retrieval with bounded,
  dependency-aware context.
- Semantic diffs describe behavioral consequences that text diffs cannot.
- Transactional refactors let agents change complete program structures rather
  than coordinate independent text replacements.
- Architectural health manifests expose concentrated control flow, authority,
  state, dependencies, and review context as revision-bound facts. Project
  policy can reject regressions without making one architecture universal.
- Controlled concurrency, cancellation, and resource bounds make process
  behavior inspectable and predictable.

These features should be evaluated as a system. AIL's claim is not that any one
of them is individually novel.

## Human role

Software agents are the primary authors and operators in AIL's design model, but
humans remain responsible for review, approval, governance, incident
investigation, and accountability.

Human authorship ergonomics are secondary. Human auditability is a hard
requirement.

AIL must not become an opaque agent bytecode. A human should be able to inspect
canonical source, public contracts, semantic changes, diagnostics, authority,
and externally observable behavior without relying on undocumented model
reasoning.

## Falsifiable project claim

The project should ultimately test this claim:

> On representative implementation, repair, and refactoring tasks, an agent
> using AIL and its semantic compiler interface requires less total context,
> fewer repair iterations, and produces fewer regressions than the same agent
> using a mainstream language with its standard compiler and language-server
> tooling.

Comparisons must use strong baselines. AIL should be evaluated against existing
languages with their normal semantic tools, not against agents limited to raw
text search and replacement.

The application domain, benchmark protocol, and representative agent changes are
defined at the next documentation layer in
[application-vision.md](application-vision.md).

## Boundaries

AIL does not assume that memory safety, concurrency, distributed behavior, or
software complexity stop mattering when agents write code. It seeks semantics
and interfaces through which agents can manage those problems with less hidden
state and less reconstruction work.

AIL is not:

- a prompt language or model-specific encoding;
- an agent communication protocol;
- a compressed syntax optimized only for token count;
- a replacement for human review or governance;
- justified solely by better editor or language-server features; or
- committed to a production backend before its semantic core is validated.

Familiar syntax is useful where model priors improve reliability, but familiarity
does not override canonicality or semantic precision.

## Decision test

Language and tooling proposals should answer:

1. Which part of total agent change cost does this reduce?
2. Why does the behavior belong in the language, the compiler protocol, or both?
3. What semantic fact becomes more explicit or mechanically inspectable?
4. How will the behavior be represented in canonical source?
5. How will it be tested deterministically?
6. How will it be exposed through structured diagnostics or semantic queries?
7. Can a human audit the resulting behavior and change?

If a proposal cannot answer these questions, it is not yet grounded in the
project thesis.
