# Implementation stack evaluation

Status: **Open**

Decision owner: project maintainers

Decision record: [decisions/0001-implementation-stack.md](decisions/0001-implementation-stack.md)
Selection record: a new ADR created in M13 that supersedes ADR 0001

Documentation layer: implementation evidence plan. Candidate behavior is not
normative language or protocol behavior.

The repository is deliberately stack-neutral. The stack should be chosen using
small comparable prototypes, not familiarity alone.

## Prerequisites

Do not begin the common spikes until:

1. the first application use cases and relevant requirements are accepted;
2. the five-construct semantic subset is written;
3. the canonical fixtures and structured diagnostic shape are shared; and
4. the minimal protocol contract for handles, revisions, rename, and identity
   mapping is reviewable.

Candidate prototypes test the same written contract. They must not resolve
semantic ambiguity independently, because that would make their results
incomparable.

## Decision criteria

Score each candidate against the same evidence:

| Criterion | Weight | Evidence |
| --- | ---: | --- |
| Correctness and maintainability of compiler internals | 25% | Typed AST/IR spike and tests |
| Incremental parsing and source rewriting | 15% | Parse/format round-trip spike |
| Type/effect-system implementation ergonomics | 15% | Constraint-checker spike |
| Native performance and deployment | 10% | Build size, startup, throughput |
| Agent protocol and tooling integration | 10% | Versioned protocol spike |
| Cross-platform and WebAssembly support | 10% | CI/build matrix |
| Debugging, profiling, and test ecosystem | 10% | Tooling exercise |
| Contributor accessibility | 5% | Setup time and documentation |

Weights should be changed before results are known, not after.

## Candidates worth testing

### Rust

Likely strengths: memory safety without a garbage collector, strong algebraic
data types, mature parser/compiler crates, predictable native binaries, good
WebAssembly support, and a natural path to code generation libraries.

Main costs: longer compile times, ownership friction while graph-heavy compiler
structures are evolving, and additional complexity for contributors new to
Rust.

### TypeScript

Likely strengths: fastest protocol and tooling iteration, excellent JSON and
editor integration, low setup cost, and easy distribution of early web or
language-server experiments.

Main costs: weaker representation invariants, runtime and memory overhead, and a
likely second implementation or native component once compilation and runtime
performance become central.

### OCaml

Likely strengths: excellent fit for typed ASTs, pattern matching, type-system
work, and persistent data structures; concise compiler implementation.

Main costs: a smaller contributor and library ecosystem, less familiar
deployment tooling, and a less direct path to some IDE and WebAssembly targets.

## Recommended spikes

Each candidate should implement the same bounded work under
`prototypes/<candidate>/`:

1. Parse a five-construct AIL subset into a lossless syntax tree.
2. Canonically print it and prove parse/print idempotence with fixtures.
3. Assign revision-scoped node handles.
4. Type-check local inference plus one explicit public function signature.
5. Detect one undeclared capability and emit the agreed structured diagnostic.
6. Apply a handle-based rename transaction and return an identity map.
7. Expose the result through a minimal versioned request/response interface.

Capture:

- implementation time and code size;
- cold and incremental test times;
- memory use on a shared fixture;
- quality of parser recovery;
- ease of maintaining source spans and stable handles;
- packaging on macOS, Linux, Windows, and WebAssembly; and
- friction encountered, not just successful output.

## Decision rule

Use the smallest stack that can plausibly remain the authoritative compiler
implementation through the first native backend. A prototype can be discarded;
a semantic oracle is expensive to rewrite.

Current recommendation: test Rust as the durable-compiler candidate and
TypeScript as the rapid-tooling baseline. Add OCaml if the team is prepared to
support it operationally. This is a recommendation for evidence gathering, not
a stack decision.
