# Roadmap

This roadmap orders uncertainty before implementation volume. Dates and release
labels should be added only after the core semantics and stack have been chosen.

Documentation role: delivery sequencing across the layers defined in
[README.md](README.md). A roadmap entry does not make its planned behavior
normative.

## Milestone operating model

Milestones describe reviewable capabilities, not dates. Status values are
**Complete**, **Active**, **Planned**, and **Deferred**.

`docs/STATUS.md` names the active milestone and gives the next agent its
immediate handoff. Only one milestone is active unless the user explicitly
starts coordinated parallel work.

A milestone is complete only when:

1. every item in scope is delivered;
2. every non-scope boundary is preserved;
3. the milestone's focused checks pass;
4. `python3 tools/check_docs.py` passes as the repository-wide documentation
   and link check;
5. the exit criterion is demonstrated; and
6. the roadmap and status documents are updated.

M3 through M6 are independent after M2 and may be completed in any order.
The default single-milestone sequence is M3, M4, M5, then M6; another order or
parallel execution requires explicit coordination. They must not silently
change shared fixtures or benchmark rules.

Planned commands describe the verification interface each milestone must
deliver. A milestone must replace any incomplete command contract before it
becomes active.

## Milestone dependency map

| Milestone | Capability | Status | Depends on |
| --- | --- | --- | --- |
| M0 | Accepted reference workload and requirements | Complete | — |
| M1 | Frozen job-service fixture corpus | Complete | M0 |
| M2 | Benchmark harness and frozen task contract | Complete | M1 |
| M3 | Rust baseline | Complete | M2 |
| M4 | Go baseline | Complete | M2 |
| M5 | Python baseline | Active | M2 |
| M6 | TypeScript baseline | Planned | M2 |
| M7 | Cross-baseline parity and freeze | Planned | M3–M6 |
| M8 | Baseline agent calibration | Planned | M7 |
| M9 | Frozen AIL success targets | Planned | M8 |
| M10 | Illustrative AIL comparison | Planned | M9 |
| M11 | Narrow language and protocol contract | Planned | M10 |
| M12 | Comparable compiler-stack spikes | Planned | M11 |
| M13 | Compiler implementation-stack decision | Planned | M12 |

## Delivery milestones

### M0 — Accepted reference workload and requirements

**Status:** Complete

#### Delivered

- Accepted UC-001 create-job behavior
- Accepted UC-003 priority evolution
- Accepted application, language, compiler-interface, and benchmark
  requirements
- Accepted JSON fixture format
- Accepted measure-first benchmark policy
- Accepted deferral of the compiler-stack decision until comparable spikes
- One operational M0–M13 roadmap with a revision-stable handoff

#### Exit criterion

The job service, the priority change, expected evidence, benchmark boundary, and
next implementation step can be explained without choosing AIL syntax or a
compiler stack.

### M1 — Frozen job-service fixture corpus

**Status:** Complete

#### Scope

- Public JSON cases required by
  [the fixture format](benchmarks/job-service-fixtures.md)
- A machine-readable fixture schema
- A dependency-free validator and formatter under `benchmarks/tools/`
- A manifest containing every fixture path and SHA-256 digest
- Checks for required case coverage, explicit versions, Base64 payloads,
  validation issue order, final stored data, and ordered storage calls
- Clear separation between public cases and later hidden regression cases

The preferred implementation is a small Python standard-library tool. This is
benchmark tooling, not a compiler-stack decision.

#### Non-scope

- Baseline language implementations
- Hidden tests
- Agent run capture
- AIL syntax or compiler code
- Production database, HTTP, concurrency, or deployment behavior

#### Focused verification

```bash
python3 benchmarks/tools/fixtures.py check
```

#### Exit criterion

One command verifies that every accepted public case is present, correctly
formatted, schema-valid, covered by the manifest, and traceable to UC-001 or
UC-003. A second implementation can consume the fixtures without reading AIL
design documents.

### M2 — Benchmark harness and frozen task contract

**Status:** Complete

#### Scope

- A language-neutral command contract for running one case or the full corpus
- A common JSON result format
- A run-manifest schema covering source, tools, model, environment, permissions,
  retry policy, and token accounting
- Final task text for implementing UC-001 and evolving it through UC-003
- The definition of a repair cycle, successful run, timeout, and failed run
- Frozen language-independent hidden behaviors, seed categories, and packaging
  rules that do not reveal answers
- A self-test runner that proves pass, fail, malformed result, timeout, and
  changed-manifest behavior

#### Non-scope

- Any language baseline
- Comparative measurements
- AIL success targets
- AIL compiler or syntax

#### Focused verification

```bash
python3 benchmarks/tools/harness.py self-test
python3 benchmarks/tools/fixtures.py check
```

#### Exit criterion

A fake passing implementation is accepted, each seeded failure is rejected for
the expected reason, and a run cannot start with an incomplete or changed locked
manifest. Language-specific seeded locations remain to be instantiated by
M3–M6 without changing the M2 behavior or seeding rules.

### M3 — Rust baseline

**Status:** Complete

#### Scope

- Idiomatic Rust implementation of UC-001 and UC-003
- Stable Rust, Cargo, rustfmt, Clippy, rust-analyzer, and ordinary Rust tests
- Shared fixture and result contracts from M1 and M2
- A V1 checkpoint suitable as the starting point for the priority-change task
- A verified V2 reference result
- Plain setup, build, test, and run instructions

#### Non-scope

- A web framework or production datastore
- Custom tooling that gives Rust information unavailable to normal users
- Changes to shared fixtures or expected behavior

#### Focused verification

```bash
python3 benchmarks/tools/harness.py verify --language rust --visibility public
```

#### Exit criterion

The Rust runner passes every public case, focused tests cover idiomatic type and
error handling, and its normalized results match the shared oracle.

### M4 — Go baseline

**Status:** Complete

#### Scope

- Idiomatic Go implementation of UC-001 and UC-003
- Stable Go, `gofmt`, `go vet`, `gopls`, and `go test`
- The same V1 checkpoint, V2 reference result, fixture contract, and
  instructions required by M3

#### Non-scope

- A web framework or production datastore
- Custom semantic tooling
- Changes to shared fixtures or expected behavior

#### Focused verification

```bash
python3 benchmarks/tools/harness.py verify --language go --visibility public
```

#### Exit criterion

The Go runner passes every public case, focused tests cover idiomatic data and
error handling, and its normalized results match the shared oracle.

### M5 — Python baseline

**Status:** Active

#### Scope

- Idiomatic typed Python implementation of UC-001 and UC-003
- A supported CPython version, normal static type checking, formatting,
  linting, and `pytest`
- The same V1 checkpoint, V2 reference result, fixture contract, and
  instructions required by M3

#### Non-scope

- A web framework or production datastore
- Runtime metaprogramming that hides the affected code in UC-003
- Changes to shared fixtures or expected behavior

#### Focused verification

```bash
python3 benchmarks/tools/harness.py verify --language python --visibility public
```

#### Exit criterion

The Python runner passes every public case, strict static checks pass, and its
normalized results match the shared oracle.

### M6 — TypeScript baseline

**Status:** Planned

#### Scope

- Idiomatic strict TypeScript implementation of UC-001 and UC-003
- A supported Node.js version, TypeScript language service, normal formatting
  and linting, and an ordinary test runner
- The same V1 checkpoint, V2 reference result, fixture contract, and
  instructions required by M3

#### Non-scope

- A web framework or production datastore
- Dynamic patterns that bypass strict checking for the benchmarked contracts
- Changes to shared fixtures or expected behavior

#### Focused verification

```bash
python3 benchmarks/tools/harness.py verify --language typescript --visibility public
```

#### Exit criterion

The TypeScript runner passes every public case, strict type checks pass, and its
normalized results match the shared oracle.

### M7 — Cross-baseline parity and freeze

**Status:** Planned

#### Scope

- Run all four baselines through one harness
- Compare response, final state, and ordered storage calls case by case
- Resolve behavioral differences without weakening the accepted oracle
- Freeze V1 task starts and V2 reference results by source-tree digest
- Instantiate the M2 hidden seed categories in each baseline and prove that
  their behavior and oracle did not change
- Lock tool versions, task text, public and hidden tests, source trees, and run
  manifests by digest
- Publish a parity report covering every case and language

#### Non-scope

- Agent benchmark runs
- AIL targets
- Framework or performance optimization

#### Focused verification

```bash
python3 benchmarks/tools/harness.py verify-all
python3 benchmarks/tools/fixtures.py manifest --check
```

#### Exit criterion

Every language produces the same normalized behavior for every public and
hidden case, and all benchmark inputs are locked by digest before measurements
start.

### M8 — Baseline agent calibration

**Status:** Planned

#### Scope

- At least 10 successful trials for each task and baseline language
- Separate accounting for unsuccessful and timed-out runs
- Complete capture of model input, tool output, edits, checks, repair cycles,
  elapsed time, startup, memory, latency, and throughput
- At least 30 warm-state performance runs per baseline using the NFR-004
  procedure
- At least 30 cold-start and corpus runs per baseline using the NFR-005
  procedure
- A recorded readiness signal, warm-up procedure, clock, affinity, load,
  package manifest, dependency lock, and attempted external access
- Replay of a sample from each configuration to verify the functional result
- A report containing raw counts and distributions without an AIL comparison

#### Non-scope

- Changing task wording, tests, or manifests after results are seen
- AIL implementation or runs
- Selecting an AIL syntax based only on source appearance

#### Focused verification

```bash
python3 benchmarks/tools/harness.py verify-calibration
```

#### Exit criterion

The repository contains complete, reviewable baseline evidence for both tasks
and all four languages, including the required agent, performance, and
cold-start trial counts, with no unexplained missing runs or changed benchmark
inputs.

### M9 — Frozen AIL success targets

**Status:** Planned

#### Scope

- Review M8 context, repair, correctness, time, startup, memory, latency, and
  throughput results
- Set the AIL thresholds used for the first comparison
- State which thresholds apply to the early interpreter and which apply only to
  a later production runtime
- Record the decision and exact benchmark configuration in an ADR

#### Non-scope

- Running AIL before the targets are accepted
- Retuning thresholds after AIL results are known
- Choosing memory management, code generation, or compiler stack

#### Focused verification

```bash
python3 benchmarks/tools/harness.py verify-targets
```

#### Exit criterion

An accepted ADR contains measurable AIL targets, the data behind them, and the
locked configuration to which they apply.

### M10 — Illustrative AIL comparison

**Status:** Planned

#### Scope

- Two or three clearly labeled **Illustrative AIL** versions of the same job
  service
- The UC-001 implementation task and UC-003 priority change shown in each
  version
- A comparison of source and context size, visible dependencies, compiler
  questions, affected-code reporting, edit size, diagnostics, formatting, and
  human review
- A written recommendation that explains tradeoffs rather than selecting by
  visual preference

#### Non-scope

- Accepted AIL syntax
- Compiler implementation
- Conformance fixtures
- Treating an illustrative example as a language rule

#### Focused verification

```bash
python3 benchmarks/tools/harness.py verify-illustrations
```

#### Exit criterion

Reviewers can compare the variants against the accepted requirements and M9
targets, identify the required compiler support for each, and choose a narrow
direction without relying on aesthetics alone.

### M11 — Narrow language and protocol contract

**Status:** Planned

#### Scope

- The first 20–30 language constructs needed by the accepted job service
- Canonical grammar and formatting rules
- Public and local type rules
- Closed result and matching behavior
- Minimal capability and effect rules for job storage
- Deterministic evaluation and fault boundaries
- Module and visibility rules
- Numbered proposed language and protocol rules traceable to the accepted
  requirements
- Ten to twenty proposed canonical fixtures with expected canonical text,
  types, runtime results, storage calls, and diagnostics
- A named five-construct spike subset and the exact fixtures and protocol
  examples that exercise only that subset
- A transport-independent compiler interface for revisions, handles,
  inspection, diagnostics, impact reporting, rename, and identity mapping
- The PROTO-004 contract shape for an atomic UC-003 priority evolution,
  including stale-base rejection, rollback, canonical edits, semantic diff,
  diagnostics, identity mapping, and validation summary
- A dependency-free checker for rule identifiers, requirement traceability,
  fixture coverage, expected-result fields, and spike-subset boundaries

#### Non-scope

- Native code generation
- General concurrency
- Production replay
- Implementation of advanced transactional refactors
- A production source tree

#### Focused verification

```bash
python3 specs/tools/core_contract.py check
```

#### Exit criterion

Two independent readers can predict each fixture's canonical text, type result,
runtime result, storage calls, and primary diagnostic. The compiler interface
can represent the required objects, express the PROTO-004 transaction contract,
and reject stale edits without choosing a transport. The checker proves that the
five-construct spike corpus is a closed subset of the larger contract.

### M12 — Comparable compiler-stack spikes

**Status:** Planned

#### Scope

- The exact M11 five-construct parser, formatter, checker, diagnostic, revision,
  and rename subset in Rust and TypeScript
- OCaml only if maintainers agree to support it operationally
- Identical M11 spike-subset fixtures and protocol examples for every candidate
- Recorded implementation time, code size, test time, memory, recovery quality,
  handle maintenance, packaging, and contributor friction
- No candidate-specific change to semantics
- One candidate-neutral command that runs every spike and compares its results

#### Non-scope

- Production compiler architecture
- Large runtime or backend work
- Features absent from the shared spike contract

#### Focused verification

```bash
python3 prototypes/check.py
python3 specs/tools/core_contract.py check
```

#### Exit criterion

Every candidate passes the same fixtures and produces a complete scorecard
against `docs/stack-evaluation.md`.

### M13 — Compiler implementation-stack decision

**Status:** Planned

#### Scope

- Review M12 evidence against the predeclared weights
- Select the authoritative compiler implementation language
- Record the decision, rejected alternatives, risks, and follow-up work in a new
  stack-selection ADR that supersedes ADR 0001
- Update the roadmap with implementation milestones for the semantic oracle
  based on the chosen stack

#### Non-scope

- Starting the production source tree before the ADR is accepted
- Changing evaluation weights after seeing results without a separate reviewed
  rationale

#### Focused verification

```bash
python3 prototypes/check.py
python3 tools/check_docs.py
```

#### Exit criterion

The stack-selection ADR is accepted and supersedes ADR 0001, the selected stack
can plausibly remain authoritative through the first production backend, and
the next semantic-oracle milestone has a concrete scope and verification plan.

## Deferred scaling candidate: architectural regression control

[UC-007](use-cases/UC-007-architectural-regression-control.md), its
[proposed requirements](requirements/architectural-health.md), and the
[architectural health manifest](architecture-health.md) remain a separate
candidate validation track. They are not part of M0–M13 and do not expand a
milestone implicitly.

After the job-service targets are frozen in M9, maintainers may review whether
UC-007 should be the next scaling use case. Acceptance requires its starting
workspace, cancel-job behavior, project policy, metric catalog, fixtures,
baseline comparison, and budgets to satisfy the gate in its requirement set.
Accepted work must be added as numbered milestones with explicit dependencies.

Architectural-health implementation should normally follow the core semantic
graph and revision protocol. Earlier benchmark or design work requires an
explicitly coordinated milestone and still does not block M10–M13.

## Long-range outlook after M13

This section records intended capability order but is not an operational
roadmap. M13 must replace the next portion with numbered milestones, one active
at a time, based on the selected implementation stack.

1. **Semantic oracle:** parser with recovery and source preservation, canonical
   formatter, resolver, type and minimal effect checker, typed holes, structured
   diagnostics, deterministic interpreter, and conformance harness.
2. **Core agent protocol:** versioned revisions and handles, semantic queries,
   elaborated views, validated rename, identity maps, atomic validation, and
   bounded semantic context.
3. **Accepted scaling features:** only use cases and requirements that have
   passed their own acceptance gates, potentially including UC-007.
4. **Safety and controlled execution:** memory model, structured concurrency,
   cancellation, resource limits, recordable nondeterminism, and replay policy.
5. **Production lowering:** one authoritative backend, reproducible artifacts,
   debugging metadata, foreign primitives, and runtime packaging.
6. **Ecosystem and full validation:** compatible target-source emission,
   advanced refactors, semantic review reports, package and supply-chain policy,
   and the full empirical agent benchmark.
