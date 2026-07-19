# AIL application vision

Status: **Application direction**

Documentation layer: application vision. This document translates the enduring
[project thesis](project-intent.md) into a real-world destination and an initial
validation domain. It is not a normative language specification. See the
[documentation model](README.md).

## Destination

AIL is intended to become a default greenfield implementation language for
software primarily built and maintained by agents.

The long-term application category is intentionally ordinary rather than niche:

- backend services and APIs;
- event and queue workers;
- command-line applications;
- scheduled and data-processing applications; and
- eventually, general application software where platform ecosystems do not
  require another language.

The envisioned decision is familiar. When starting a new server, microservice, or
application, a team compares the supported stacks and asks which bundle of
tradeoffs it should accept. Those bundles were shaped partly by the needs of
human authors and organizations. AIL asks whether an agent-operated project can
occupy a different point in that tradeoff space.

AIL should eventually be the default answer when agents are the primary
implementers and maintainers, unless ecosystem, interoperability, hardware, or
deployment constraints require something else.

## Intended tradeoff

Existing language families offer different combinations:

- systems-oriented compiled languages provide strong control, safety, and
  predictable performance, while often exposing substantial type, ownership,
  and representation complexity in source;
- managed compiled languages reduce some source-level resource reasoning while
  accepting runtime and garbage-collection tradeoffs;
- dynamic languages provide compact, highly inferred source while moving many
  guarantees, failures, and performance costs to runtime; and
- ecosystem-bound languages gain extraordinary compatibility while inheriting
  the semantics and constraints of their host platform.

AIL does not assume that agents make memory, concurrency, distribution, or type
problems disappear. It attempts to move mechanically solvable complexity out of
canonical source and into a compiler that can expose the complete result on
demand.

The target combination is:

- predictable compiled execution and resource behavior;
- strong static semantics and no undefined behavior in ordinary code;
- locally inferred, low-entropy source;
- explicit contracts wherever another unit can depend on behavior;
- a fully elaborated semantic model available through the compiler;
- deterministic, inspectable effects and authority; and
- structural program operations that validate complete changes.

The core bet is:

> Static semantic richness does not require equivalent textual verbosity when
> the compiler exposes a complete, queryable semantic model.

AIL may therefore be semantically richer than a conventional statically typed
language while remaining textually sparser in local implementations.

## What AIL may bypass

AIL may reject compromises whose primary purpose is unaided human authoring:

- multiple equivalent syntaxes;
- repetitive local annotations that the compiler can determine;
- shorthand that hides effects or authority;
- ambient runtime state;
- convention-only contracts;
- unrestricted metaprogramming that defeats complete analysis; and
- source-level obligations that can be solved and explained mechanically.

AIL cannot bypass the underlying requirement to manage memory, order concurrent
work, handle failure, control resources, and interact with nondeterministic
systems. It must choose precise semantics for those concerns and make their
observable consequences inspectable.

For example, avoiding mandatory opaque garbage-collection pauses is an
application-level latency and predictability goal. It is not yet a decision to
use ownership, regions, reference counting, compile-time tracing, or another
specific memory model.

## Operator experience

Agents should not need to understand a large AIL program by repeatedly loading
and reconstructing raw source. They should be able to ask the compiler for:

- the public contract of a unit;
- elaborated local types and inferred effects;
- callers and downstream dependencies;
- capability and authority boundaries;
- state, serialization, and schema constraints;
- relevant tests and invariants;
- bounded semantic context for a change;
- the semantic impact of a proposed diff; and
- validated structural edits.

Canonical text remains the durable, version-controlled artifact. The semantic
model and structural protocol are the primary operational interface.

AIL is not intended to be unpleasant or obscure for its own sake. It is not
optimized for unaided human typing, but its canonical source and compiler views
must remain auditable by humans responsible for review, operations, incidents,
and governance.

## First validation wedge

The first validation domain is greenfield backend services and workers primarily
implemented and maintained through agent tooling.

This wedge is broad enough to exercise the thesis while still admitting bounded
examples. A representative logical service has the shape:

```text
ordered request or event
  + explicit database, network, clock, configuration, and telemetry capabilities
  -> response, state transitions, and ordered observable effects
```

Representative systems should require some combination of:

- request and response schemas;
- validation and domain errors;
- persistent state;
- outbound service calls;
- configuration and secrets;
- time and randomness;
- logging, metrics, and traces;
- timeouts, retries, cancellation, and bounded parallel work;
- serialization compatibility; and
- deterministic tests using recorded or supplied external inputs.

The first validation slice does not need a production HTTP stack, database
driver, scheduler, or deployment platform. Deterministic test capabilities may
stand in for those systems while the language semantics are being established.

## Representative agent changes

The validation corpus should eventually include changes such as:

1. implement a handler from a typed contract and tests;
2. add a field to a request schema and propagate it into persistent state;
3. add a new closed domain-error variant and update exhaustive handling;
4. introduce an outbound call with an explicit timeout and capability;
5. replace sequential work with bounded parallel execution;
6. rename or move a domain concept across schemas, handlers, and tests;
7. determine whether a change introduces a new effect or authority requirement;
8. identify every consumer affected by a serialization change; and
9. diagnose a resource, cancellation, or replay mismatch; and
10. add an operation to a mature service without enlarging a dispatcher,
    bypassing an authority boundary, or increasing an accepted hotspot.

These are behavior-level scenarios. They do not establish AIL syntax.

## Application qualities implied by the vision

The use cases are expected to produce requirements in at least these areas:

- predictable latency, throughput, startup, and memory behavior;
- compact local source with complete compiler elaboration;
- explicit public schemas, errors, effects, capabilities, and resource contracts;
- deterministic evaluation and test substitution for external systems;
- structured concurrency, cancellation, timeouts, and bounds;
- reproducible builds and straightforward deployment;
- stable serialization and migration identities;
- structured observability that preserves semantic identities;
- package and dependency isolation;
- safe foreign-system boundaries; and
- revision-safe inspection, change, and impact analysis; and
- revision-bound architectural facts, deltas, and enforceable project policy.

These are requirement candidates, not accepted language rules. Concrete use cases
must establish their priority, scope, and acceptance criteria.

## Strong baseline

AIL must be compared with existing languages using their normal high-quality
tools. For the initial service domain, likely baselines include:

- Rust with its compiler, package manager, analyzer, formatter, and refactoring
  support;
- Go with its compiler, standard tooling, formatter, and language server;
- Python with type checking, formatting, and normal framework tooling; and
- TypeScript with its compiler, language service, formatting, and relevant
  runtime tooling.

The comparison is not simply source-token count. It should measure:

- context consumed before a correct change;
- semantic queries and source reads;
- first-pass compile and test success;
- repair iterations and diagnostic localization;
- regressions and missed downstream changes;
- semantic and textual diff noise;
- elapsed agent work;
- runtime latency, throughput, startup, and memory against an agreed envelope;
  and
- the completeness of human-reviewable evidence.

The benchmark must record model and agent versions, tools, prompts or task
contracts, supplied context, retries, correctness criteria, and environment.

AIL fails the application thesis if it gains compact source by discarding the
guarantees required for predictable deployment. It also fails if it recreates
systems-language source burden merely to reach systems-language performance.

## Not the first validation target

The first slice does not attempt to validate:

- kernels, device drivers, or direct hardware control;
- hard real-time systems;
- browser or mobile user-interface ecosystems;
- arbitrary legacy foreign-function integration;
- unrestricted plugin or build-script execution;
- source-compatible replacement of an existing language; or
- every component of a production distributed-systems platform.

These exclusions bound the first proof. They are not permanent statements about
AIL's eventual reach.

## Questions to resolve through use cases

Before application qualities become accepted requirements, the project must
decide:

1. Which one or two service shapes form the reference workload?
2. Which deployment target and runtime envelope define success?
3. Which external capabilities are essential in the first executable slice?
4. Which concurrency and resource behaviors must the first slice exercise?
5. Which schema-evolution and compatibility changes are representative?
6. What interoperability is required to build a credible service?
7. Which human review artifacts are required for approval and incident response?
8. Which baseline implementations and agent tasks make the comparison fair?

The answers belong in concrete use-case records and derived requirements before
they constrain syntax or implementation architecture.
