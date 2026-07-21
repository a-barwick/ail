# Roadmap

This roadmap orders uncertainty before implementation volume. Dates and release
labels should be added only after the core semantics and stack have been chosen.

Documentation role: delivery sequencing across the layers defined in
[README.md](README.md). A roadmap entry does not make its planned behavior
normative.

## Milestone operating model

Milestones describe reviewable capabilities, not dates. Status values are
**Complete**, **Active**, **Planned**, **Deferred**, and **Superseded**.

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
| M5 | Python baseline | Complete | M2 |
| M6 | TypeScript baseline | Complete | M2 |
| M7 | Cross-baseline parity and freeze | Complete | M3–M6 |
| M8 | Baseline agent calibration | Deferred | M7 |
| M9 | Frozen AIL success targets | Deferred | M8 campaign |
| M10 | Illustrative AIL comparison | Deferred | — |
| M11 | Compiler-stack spike contract | Complete | M0, M7 |
| M12 | Comparable compiler-stack spikes | Superseded | M11 |
| M13 | Compiler implementation-stack decision | Superseded | M11 |
| M14 | Rust lossless syntax and canonical formatter | Complete | M11, ADR 0004 |
| M15 | Rust static semantics and diagnostics | Complete | M14 |
| M16 | Rust revision protocol and validated rename | Active | M15 |
| M17 | Deterministic core interpreter | Planned | M16 |

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
- One revision-stable operational roadmap and handoff

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

**Status:** Complete

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

**Status:** Complete

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

**Status:** Complete

#### Scope

- Run all four baselines through one harness
- Compare response, final state, and ordered storage calls case by case
- Resolve behavioral differences without weakening the accepted oracle
- Freeze distinct answer-free UC-001 and UC-003 task-start packages for every
  baseline language, with explicit file manifests and independent tree digests
- Freeze the accepted V1 and V2 reference results by source-tree digest
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
python3 benchmarks/tools/task_starts.py check --run-starting-state
python3 benchmarks/tools/harness.py verify-all
python3 benchmarks/tools/fixtures.py manifest --check
```

#### Exit criterion

All eight answer-free task starts pass their expected starting-state checks.
Every language produces the same normalized behavior for every public and
hidden case, and all benchmark inputs are locked by digest before measurements
start.

### M8 — Baseline agent calibration

**Status:** Deferred

Deferred after M8f.

[ADR 0003](decisions/0003-prioritize-stack-decision.md) removes the remaining
statistical campaign from the compiler-stack critical path. M8a through M8f are
preserved as benchmark infrastructure and non-official pilot evidence. M8g
through M8o are not active work and no official M8 evidence exists.

M8 is delivered through the sequential M8a–M8o submilestones in the accepted
[M8 execution plan](m8-execution-plan.md). That plan is retained so the
campaign can be resumed explicitly; it no longer blocks M11 through M17.

#### Completed submilestone: M8a — Freeze the experiment contract

[ADR 0002](decisions/0002-m8-agent-experiment-contract.md) fixes the candidate
measured agent, model, reasoning, interactive tool protocol, prompt wrapper,
initial context, normal tools, token accounting, permissions, limits, retry and
termination rules, reference environment, and run classifications. It amends
the NFR-002 safety limit from 100,000 to 500,000 cumulative delivered input
tokens because ordinary interactive pilots exceeded the lower limit.

M8a collected no official evidence and did not implement the later trial
runner.

#### Completed submilestone: M8b — Build the evidence contracts and verifier

M8b delivered schemas and digest locks for agent trials, raw model and tool
events, warm-state measurements, cold-start and memory measurements, campaign
configuration and ordering, evidence indexes, and the final fact-only report.
The calibration verifier checks the complete digest graph, exact M8a inputs,
token reconciliation, raw-event continuity, counts, exclusions, summaries, and
configuration identity.

#### M8b focused verification

```bash
python3 benchmarks/tools/harness.py verify-calibration
python3 tools/check_docs.py
```

M8b is complete only when schemas and locks cover every declared evidence
artifact and the synthetic fixtures reject missing counts, duplicate trial
identities, changed inputs, invalid hashes, incomplete token categories,
missing raw events, unaccounted exclusions, incorrect summaries, and mixed
configurations with stable results.

The gate passes with 18 locked artifacts and 14 stable synthetic outcomes.
M8b collected no agent or performance evidence.

#### Completed submilestone: M8c — Implement the interactive agent runner

Build a fresh locked workspace, enforce the manifest-start gate, invoke only
the selected Codex agent and normal language tools, and record every model,
tool, edit, validation, permission, limit, and process event. Derive edits,
validation attempts, incomplete validations, and repair cycles from the raw
stream using the frozen M2 definitions.

Dry and fake streams must prove stable classification and evidence accounting
for success, failure, timeout, permission violation, input-token limit, and
incomplete evidence. M8c does not run an official trial or implement the later
private correctness replay.

M8c delivered the locked task-workspace and prompt gate, task-derived
least-privilege permissions, isolated Codex and loopback-provider
configuration, complete raw event capture, exact token reconciliation and
limits, process-group termination, deterministic final-source retention, and
M2 edit, validation, incomplete-validation, and repair-cycle accounting. Six
fake/dry terminal outcomes and the boundary checks pass without invoking the
measured agent.

#### M8c focused verification

```bash
python3 benchmarks/tools/harness.py verify-calibration
python3 tools/check_docs.py
```

#### Completed submilestone: M8d — Implement correctness verification and replay

Run the full public and separately held private oracle against the retained
final revision after the agent stops. Verify seeded consumers, protected
artifacts, permissions, revision-bound completion evidence, and functional
replay without exposing private inputs or results to the agent.

M8d must reject incomplete, answer-exposing, revision-mismatched, and seeded
regression results. It may use fake and dry evidence but must not collect
official trials.

M8d delivered a verifier-only private-package boundary, canonical retained
source validation, protected-file and permission enforcement, task-applicable
public/private/seed coverage, revision-bound completion evidence, and
functional replay from a second fresh extraction. Eight fake/dry outcomes and
a campaign-level stale-completion mutation prove that false success is rejected.

#### M8d focused verification

```bash
python3 benchmarks/tools/harness.py verify-calibration
python3 tools/check_docs.py
```

#### Completed submilestone: M8e — Implement performance measurement

Add equivalent per-language adapters for warm-up, readiness, the shared corpus,
monotonic latency and throughput, percentile and variance derivation, process
creation, cold readiness, idle and peak RSS, package identity, dependency
identity, external-access attempts, and functional correctness.

M8e may run one non-official warm and cold pilot per baseline. It must not
freeze the campaign or collect official measurements.

M8e delivered persistent Rust, Go, Python, and TypeScript adapters outside the
frozen M7 checkpoint files plus one shared harness for corpus ordering,
functional and trace comparison, native monotonic samples, warm-up,
throughput, nearest-rank percentiles, integer variance, load, affinity, process
creation, readiness, idle and peak RSS, package and dependency identity,
network-denial events, and safety classification. One explicitly non-official
warm and cold pilot per baseline passed; official counts remain zero.

#### M8e focused verification

```bash
python3 benchmarks/tools/harness.py verify-calibration
python3 tools/check_docs.py
```

#### Completed submilestone: M8f — Readiness and campaign freeze

M8f proved the provider-backed runner with three successful UC-001 pilots
across Python, Rust, and Go. Every request reconciled exactly with provider
usage. Separate Rust and Go UC-003 pilots correctly stopped before exceeding
the 500,000 cumulative-input safety limit. The pinned Codex fake-upstream
integration, all task-start gates, all four warm/cold baseline pairs, the
calibration verifier, and documentation checks pass.

The accepted readiness amendment treated representative provider success plus
an enforced live safety-limit path as the infrastructure gate. The frozen
summary is
[`benchmarks/calibration/readiness/m8f-summary.json`](../benchmarks/calibration/readiness/m8f-summary.json).
Official counts remain zero.

#### Deferred remainder: M8g–M8o

The first M8g launch correctly stopped before any official attempt because the
required task-start gate fails for TypeScript UC-001: its public test output
omits `TODO(UC-001)`. Do not repair or re-freeze this campaign as part of M11
through M17. Resuming M8 requires an explicit maintainer decision, a corrected
freeze, and a new pre-collection verification pass.

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

**Status:** Deferred

Numeric targets are frozen only after maintainers explicitly resume and
complete the M8 statistical campaign. They must be accepted before comparative
AIL benchmark runs, but they do not gate the language contract, compiler
spikes, stack decision, or semantic-oracle implementation.

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

**Status:** Deferred

Illustrative variants are no longer a delivery gate. M11 makes syntax choices
as numbered proposed rules with canonical fixtures, so candidate aesthetics
cannot silently establish semantics.

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

### M11 — Compiler-stack spike contract

**Status:** Complete

#### Scope

- Exactly five language constructs needed to test the compiler architecture
- Canonical grammar and formatting rules for those constructs
- The minimum public-signature, local-inference, and capability checks exercised
  by the spike fixtures
- Numbered proposed language and protocol rules traceable to accepted
  requirements
- Proposed canonical fixtures with expected canonical text, type results, and
  primary diagnostics
- A transport-independent compiler interface for revisions, handles,
  inspection, diagnostics, validated rename, and identity mapping
- A dependency-free checker for rule identifiers, requirement traceability,
  fixture coverage, and expected-result fields

#### Non-scope

- The broader 20–30 construct job-service core
- Full job-service execution or storage-call semantics
- Native code generation
- General concurrency
- Production replay
- General semantic diff and advanced transactional refactors
- A production source tree

#### Focused verification

```bash
python3 specs/tools/core_contract.py check
```

#### Exit criterion

Two independent readers can predict each fixture's canonical text, type result,
and primary diagnostic. The compiler interface can represent revisions,
revision-scoped handles, a validated rename, stale-edit rejection, and identity
mapping without choosing a transport. The checker proves that every fixture is
fully covered by the five-construct contract.

### M12 — Comparable compiler-stack spikes

**Status:** Superseded

[ADR 0004](decisions/0004-rust-compiler-stack.md) records the maintainer's
direct Rust selection. No Rust/TypeScript comparison or disposable compiler
candidate will be built. The M11 fixtures instead constrain the authoritative
Rust implementation beginning in M14.

### M13 — Compiler implementation-stack decision

**Status:** Superseded

The owner made the decision directly in
[ADR 0004](decisions/0004-rust-compiler-stack.md): Rust is authoritative through
the first production backend. M14 is the concrete successor implementation
milestone.

### M14 — Rust lossless syntax and canonical formatter

**Status:** Complete

#### Scope

- A root Cargo workspace and authoritative Rust compiler tree
- A lossless lexer that represents every UTF-8 source byte exactly once
- A typed syntax tree for the five M11 constructs
- Half-open UTF-8 byte spans on tokens and syntax nodes
- Deterministic `AIL.PARSE.EXPECTED_TOKEN` recovery for the M11 fixture
- Canonical formatting for declarations, expressions, literals, and record
  initializer order
- A small `ailc` command for lossless reconstruction, checking, and formatting
- Executable conformance tests backed by the frozen M11 fixtures

#### Non-scope

- Name resolution or type inference
- Capability/effect checking
- Revision storage, inspection, rename, or identity mapping
- Function execution or the broader job-service core
- Native lowering

#### Focused verification

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
python3 specs/tools/core_contract.py check
```

#### Exit criterion

Every M11 source reconstructs byte-for-byte from the lossless token stream.
Every parseable fixture produces the declared canonical source, formatting is
idempotent, and the recovery fixture emits the declared primary diagnostic at
the deterministic insertion point.

### M15 — Rust static semantics and diagnostics

**Status:** Complete

#### Scope

- Top-level and function-local name resolution
- Exact named-type checking
- Local `let` inference
- Record and closed-variant construction checks
- Capability interface input and declared-effect checking
- Structured diagnostics and elaborated type facts for all M11 static fixtures

#### Non-scope

- Revision transactions and rename
- Function execution
- Broader core constructs

#### Focused verification

```bash
cargo test --workspace
python3 specs/tools/core_contract.py check
```

#### Exit criterion

The Rust compiler matches every M11 type result and primary static diagnostic
without fixture-specific behavior.

#### Delivered

- Deterministic top-level and function-local name resolution over the M14
  syntax tree
- Exact named-type checks for records, closed variants, function results,
  local bindings, and capability operation arguments
- Caller-supplied capability interfaces and declared-effect checking
- Revision-scoped elaborated type facts and structured static diagnostics
- Executable coverage for M11 static fixtures, diagnostic causality, closed
  construction, local scope, and primary-diagnostic ordering

### M16 — Rust revision protocol and validated rename

**Status:** Active

#### Scope

- Immutable revisions and revision-scoped handles
- Elaborated inspection
- Atomic validated rename
- Canonical edits, stale-revision rejection, and complete identity mapping
- Deterministic protocol ordering

#### Non-scope

- General semantic diffs or structural refactors
- Function execution
- Transport selection

#### Focused verification

```bash
cargo test --workspace
python3 specs/tools/core_contract.py check
```

#### Exit criterion

The Rust compiler matches the M11 inspection, rename, stale-edit, edit-order,
and identity-map fixtures through a transport-independent API.

### M17 — Deterministic core interpreter

**Status:** Planned

#### Scope

- Accept the next bounded language rules required to execute the reference
  job-service core
- Deterministic tree-walking execution
- Supplied capability instances and ordered observable calls
- Runtime faults and results required by the accepted fixtures
- AIL execution against the shared language-independent job-service corpus

#### Non-scope

- Native code generation
- General concurrency
- Production I/O adapters

#### Focused verification

```bash
cargo test --workspace
python3 benchmarks/tools/harness.py verify --language ail --visibility public
```

#### Exit criterion

The authoritative interpreter executes the accepted AIL reference service and
matches the shared public oracle without weakening the frozen behavior.

## Deferred scaling candidate: architectural regression control

[UC-007](use-cases/UC-007-architectural-regression-control.md), its
[proposed requirements](requirements/architectural-health.md), and the
[architectural health manifest](architecture-health.md) remain a separate
candidate validation track. They are not part of M0–M17 and do not expand a
milestone implicitly.

After the semantic oracle and core protocol exist, maintainers may review
whether UC-007 should be the next scaling use case. Acceptance requires its
starting workspace, cancel-job behavior, project policy, metric catalog,
fixtures, baseline comparison, and budgets to satisfy the gate in its
requirement set. Accepted work must be added as numbered milestones with
explicit dependencies.

Architectural-health implementation should follow the core semantic graph and
revision protocol. It does not block M14 through M17.

## Long-range outlook after M17

This section records intended capability order but is not an operational
roadmap. M14 through M17 own the current compiler path; later work must become
numbered milestones one active at a time.

1. **Broader semantic oracle:** expand the M11 spike subset to the accepted
   20–30 construct job-service core, then implement parser recovery and source
   preservation, canonical formatting, resolution, type and minimal effect
   checking, typed holes, structured diagnostics, deterministic interpretation,
   and the conformance harness.
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
