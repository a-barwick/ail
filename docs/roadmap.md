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

## Successor milestone selection

No completed milestone creates an automatic next phase. Before adding or
activating a successor, the proposal must state:

1. the accepted use case or unresolved thesis uncertainty it addresses;
2. the component of total agent change cost it is expected to reduce;
3. why language, compiler, protocol, runtime, or benchmark work is needed rather
   than ordinary tooling for an existing language;
4. the smallest evidence-producing milestone that can test the mechanism;
5. the strong mainstream baseline plan, baseline evidence, or prerequisite
   evidence needed for a fair comparison, as applicable;
6. correctness and human-auditability admission gates;
7. the result that causes a go, revise, or stop decision; and
8. explicit non-scope, including conventional progress proxies that the
   milestone does not validate.

An enabling milestone may precede official comparative trials when it is needed
to build the comparator or measurement path. It must name the later
discriminating test and must not present implementation completion as proof of
agent efficiency. More syntax, native lowering, LLVM integration, production
packaging, or self-hosting is not a successor rationale without this gate.

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
| M16 | Rust revision protocol and validated rename | Complete | M15 |
| M17 | Deterministic core interpreter | Complete | M16 |
| M18 | Next validation-slice selection | Complete | M17 |
| M19 | UC-003 schema-evolution contract | Complete | M18 |
| M20 | Workspace semantic graph and impact query | Complete | M19 |
| M21 | Atomic schema evolution and completion evidence | Complete | M20 |
| M22 | Post-UC-003 validation-slice selection | Complete | M21 |
| M23 | UC-007 acceptance package | Complete | M22 |
| M24 | Architectural regression contract | Complete | M23 acceptance |
| M25 | Architectural snapshot and agent rendering | Complete | M24 |
| M26 | Architectural delta, policy, and atomic enforcement | Complete | M25 |
| M27 | Non-official architecture-feedback pilot | Complete | M26 |
| M28 | Iterative-evolution acceptance package | Active | M27, ADR 0007 |
| M29 | Strong baseline workspaces and trajectories | Planned | M28 acceptance |
| M30 | AIL gap selection | Planned | M29 |
| M31 | Iterative-workload contract | Planned | M30 |
| M32 | Authoritative compiler implementation | Planned | M31 |
| M33 | Canonical AIL service and cumulative evidence | Planned | M32 |
| M34 | Non-official full-trajectory pilots | Planned | M33 |
| M35 | Comparative campaign readiness | Planned | M29, M34 |
| M36 | Official iterative comparison | Planned | M35 and explicit launch decision |

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

**Status:** Complete

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

#### Delivered

- Immutable canonical source revisions with SHA-256 source digests and
  deterministic revision identifiers
- Deterministic revision-scoped handles for declarations, syntax, and
  expressions, with elaborated inspection results
- Atomic validated rename across resolved M11 references, ordered canonical
  byte edits, and child-revision publication only after parse and static checks
- Rejection of stale bases, stale or non-symbol handles, invalid names, and
  collisions without changing the current revision
- Complete deterministic identity maps that classify every indexed base handle
  and report child-only handles
- Executable coverage for inspection, immutable source retention, canonical
  edit replay, rename validation, stale rejection, and repeated deterministic
  protocol results

### M17 — Deterministic core interpreter

**Status:** Complete

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

#### Delivered

- Accepted nine bounded M17 language, runtime, and protocol rules without
  changing the fixed M11 contract
- Canonical checked AIL source for validation, V1 request and stored-job
  adaptation, priority propagation, one store capability call, and closed
  outcome mapping
- Deterministic revision-scoped tree-walking execution with structured runtime
  values, faults, supplied capabilities, and ordered observed calls
- Focused formatting, static-diagnostic, immutable-revision, capability-order,
  adaptation, outcome, fault, and repeated-result tests
- A host runner that keeps JSON/Base64 decoding and versioned result projection
  outside the AIL service boundary
- A digest-locked AIL verification manifest whose one-case and corpus commands
  pass all 37 frozen public fixtures

### M18 — Next validation-slice selection

**Status:** Complete

#### Scope

- Review the completed M17 semantic-oracle evidence and the remaining accepted
  application and protocol gaps
- Select one next validation slice through the use-case and requirements gate
- Add concrete numbered implementation milestones with explicit dependencies,
  non-scope, focused checks, and exit criteria

#### Non-scope

- Compiler or runtime implementation before the next rules are accepted
- Implicit expansion of UC-007 or its architectural-health proposal
- Native lowering, general concurrency, production I/O, or broad collection
  syntax without an accepted use case and requirements

#### Focused verification

```bash
python3 tools/check_docs.py
```

#### Exit criterion

The repository identifies one accepted next validation slice and a bounded
numbered implementation sequence. An agent can begin the first implementation
milestone without inferring scope from the long-range outlook.

#### Delivered

- Reviewed the locked M17 interpreter, runtime fixtures, focused tests, and
  37-case public-corpus evidence
- Identified the remaining accepted UC-003 gaps in stable schema identity,
  semantic impact, atomic structural change, semantic diff, and completion
  evidence
- Selected compiler-guided UC-003 priority evolution in
  [ADR 0005](decisions/0005-next-validation-slice.md)
- Bounded the work as M19 through M21 without activating UC-007 or adding
  unrelated runtime, concurrency, I/O, collection, or lowering behavior

### M19 — UC-003 schema-evolution contract

**Status:** Complete

#### Scope

- Accept the smallest numbered language and protocol rules needed for the
  selected R1-to-R2 priority evolution
- Define stable schema identities separately from revision-scoped handles
- Define one immutable ordered source-set revision and its deterministic digest
- Define typed semantic relationship kinds and deterministic traversal order
- Define the `must_change`, `review`, and `unchecked` impact-query contract
- Define the atomic candidate-change request, semantic diff, validation
  summary, failure diagnostics, and completion-evidence shapes
- Freeze canonical R1 and R2 job-service workspaces containing every in-scope
  UC-003 semantic role
- Freeze positive and rejecting conformance fixtures, including stale base,
  missed consumer, incompatible schema identity, effect growth, and failed
  behavior validation
- Extend the dependency-free contract checker over every new rule, shape, and
  fixture

#### Non-scope

- Rust implementation of the new rules
- A general package, module, import, serialization, migration, or code-generation
  system
- A fixture-specific priority operation
- General semantic context, architectural-health metrics, or policy evaluation
- Native lowering, concurrency, production I/O, or new external authority

#### Focused verification

```bash
python3 specs/tools/core_contract.py check
python3 tools/check_docs.py
```

#### Exit criterion

Two independent readers can predict the complete ordered impact report and
every commit or rejection result for the frozen R1-to-R2 fixtures. Every new
behavior is traceable to accepted UC-003 requirements, and M20 can implement
the contract without choosing identity, traversal, coverage, or ordering
semantics.

#### Delivered

- Accepted three language rules and ten protocol rules for explicit schema
  identities, ordered source-set revisions, semantic relationships, categorized
  impact, atomic candidate validation, semantic diff, and completion evidence
- Fixed canonical identity grammar, source-set digest encoding, relationship
  kinds, traversal order, impact bounds, and stable rejection codes
- Froze five-path canonical R1 and R2 workspaces containing the request,
  persistence, handler, capability, adapter, projection, fixture, and evidence
  roles required by UC-003
- Froze an exact 12-entry `must_change` report, two-entry `review` report, and
  explicit unavailable external-client boundary
- Froze one successful whole-workspace transaction and stale, missed-consumer,
  incompatible-identity, effect-growth, and behavior-mismatch rejections
- Extended the dependency-free checker across 13 rules, 16 protocol shapes,
  seven fixture scenarios, deterministic digests, rule coverage, and five
  rejection mutations

### M20 — Workspace semantic graph and impact query

**Status:** Complete

#### Scope

- Implement immutable ordered multi-source revisions and canonical source-set
  digests while retaining parent revisions
- Implement the accepted stable schema identities without conflating them with
  revision-scoped handles
- Index the accepted declaration, construction, field, function, match,
  capability, effect, and source-artifact relationship kinds
- Implement deterministic revision-bound semantic inspection over those facts
- Implement the accepted pre-edit impact query and exact coverage categories
- Match every M19 inspection, graph, impact, stale-request, and incomplete-
  coverage fixture

#### Non-scope

- Committing source changes other than the existing M16 rename
- Inferring external consumers or claiming completeness beyond analyzed inputs
- General modules, imports, dependency resolution, reflection, generated code,
  or architectural-health metrics
- New runtime behavior

#### Focused verification

```bash
cargo test --workspace --test m20_impact
python3 specs/tools/core_contract.py check
python3 tools/check_docs.py
```

#### Exit criterion

At R1, the compiler returns every and only the frozen `must_change` locations,
keeps bounded reasoned entries in `review`, lists unavailable consumers in
`unchecked`, and reports unchanged storage authority and effect ordering.
Repeated requests and retained parent-revision requests are identical.

#### Delivered

- Added contextual schema identity syntax to records, variants, fields, and
  cases without reserving `identity` outside those positions
- Added immutable canonical ordered source-set revisions with the accepted
  digest encoding and path validation
- Kept persistent schema identities separate from revision-scoped handles and
  rejected missing, malformed, or duplicate schema identities
- Built the twelve-kind deterministic semantic relationship graph from checked
  declarations, signatures, constructions, field reads, matches, capabilities,
  effects, adapters, projections, verification functions, and declared source
  artifacts
- Implemented revision-bound exact impact categorization with explicit analyzed
  paths, unchecked boundaries, unchanged authority facts, and incomplete-
  coverage rejection
- Preserved retained parent snapshots and every M11–M17 single-source behavior
- Added six focused M20 tests for digests, identities, graph coverage and order,
  exact impact, revision retention, repeatability, stale requests, and coverage
  gaps

### M21 — Atomic schema evolution and completion evidence

**Status:** Complete

#### Scope

- Implement the accepted whole-workspace candidate-change transaction against
  the current immutable revision
- Apply ordered canonical per-path edits and validate the complete candidate
  source set before publication
- Return the complete cross-revision identity map and accepted semantic diff
- Return structured rejection diagnostics and machine-readable validation and
  completion summaries
- Execute the accepted static, impact, effect, and public behavior checks for
  the same candidate revision
- Match every M19 transaction fixture and preserve M11 rename behavior

#### Non-scope

- A fixture-specific `add_priority` operation or automatic product-policy
  inference
- Move, extraction, parameter addition, or general refactoring primitives
- Hidden or official agent benchmark runs
- General migrations, production adapters, native lowering, or concurrency

#### Focused verification

```bash
cargo test --workspace --test m21_schema_transaction
python3 benchmarks/tools/harness.py verify --language ail --visibility public
python3 specs/tools/core_contract.py check
python3 tools/check_docs.py
```

#### Exit criterion

The accepted complete R1 priority change commits exactly one R2 whose canonical
source, identities, semantic diff, validation summary, and behavior match the
M19 fixtures and all 37 public cases. Stale, incomplete, incompatible,
effect-changing, or behaviorally failing candidates publish no revision and
return the declared structured cause.

#### Delivered

- Added whole-source-set candidate validation against the current immutable
  revision and exact pre-edit impact report
- Canonicalized and statically checked the complete candidate before exposing
  it to the accepted public-behavior oracle or publishing a child revision
- Derived ordered per-path canonical edits, a complete indexed-handle identity
  map, persistent identity classifications, and the frozen seven-change
  semantic diff
- Bound parse, type, capability, impact, and public-behavior evidence to the
  committed child revision while retaining the base impact and unchecked
  boundary
- Returned stable structured rejections with empty edits for stale bases,
  missed consumers, incompatible identity reuse, effect growth, behavior
  mismatch, incomplete source sets, and invalid candidates
- Added four focused M21 tests covering the accepted R1-to-R2 result, every
  frozen rejection, additional atomicity boundaries, and repeated deterministic
  results
- Preserved the M11 rename protocol and all 37 public job-service results

### M22 — Post-UC-003 validation-slice selection

**Status:** Complete

#### Scope

- Review the completed M19–M21 UC-003 evidence and the remaining accepted
  application, language, runtime, and protocol gaps
- Select exactly one next validation slice through the use-case and requirements
  gate, or identify the bounded acceptance work required before selection
- Add concrete numbered contract and implementation milestones with explicit
  dependencies, non-scope, focused checks, and exit criteria

#### Non-scope

- Compiler or runtime implementation before the next slice and rules are
  accepted
- Implicit activation of UC-007 or the deferred M8 campaign
- Native lowering, production I/O, general concurrency, or broad language-core
  expansion without accepted use-case and requirement support

#### Focused verification

```bash
python3 tools/check_docs.py
```

#### Exit criterion

The repository identifies one selected direction, states any acceptance work
required before implementation, and records a bounded numbered delivery
sequence. An agent can start the first new milestone without inferring scope
from the long-range outlook.

#### Delivered

- Selected architectural regression control as the next scaling direction in
  [ADR 0006](decisions/0006-prepare-architectural-regression-control.md)
- Kept UC-007 and its requirements proposed until their concrete workspace,
  behavior, policy, fixtures, comparison, and budgets pass acceptance
- Bounded the work as M23 acceptance preparation, M24 contract, M25 and M26
  implementation, and M27 non-official usability validation
- Made deterministic compact agent rendering an explicit product surface
  derived from the compiler's structured facts
- Recorded Amp as an optional versioned operator for authoring or a later
  non-official pilot, not as language semantics or official evidence
- Preserved a build-heavy effort bias after one short acceptance gate

### M23 — UC-007 acceptance package

**Status:** Complete

#### Scope

- Freeze the exact mature job-service starting workspace and explain or amend
  the proposed scale before acceptance
- Freeze transport-independent `CancelJob` inputs, outcomes, state changes,
  capability use, effect order, and behavior fixtures
- Freeze one valid change, one behaviorally correct centralized regression, and
  one superficial helper-splitting change
- Freeze architecture groups, allowed dependencies, capability and state
  boundaries, the accepted hotspot baseline, and the smallest metric set that
  distinguishes the three changes
- Freeze expected structured results and compact agent-facing text for the
  starting revision and every candidate
- Identify equivalent Rust, Go, Python, and TypeScript baseline tools and
  comparison rules
- Set the false-finding allowance, analysis budget, and manifest-size budget
  before inspecting implementation results
- Add dependency-free checks for the acceptance package and decide whether to
  accept UC-007 and `APP-006`, `LANG-006`, `PROTO-006`, `PROTO-007`, `NFR-006`,
  and `NFR-007`

#### Non-scope

- Rust compiler, policy-engine, or text-renderer implementation
- Treating any proposed policy syntax, metric, threshold, or diagnostic as
  accepted before the gate passes
- General architecture inference, a composite quality score, or automatic
  refactoring
- Agent trials, official benchmark evidence, M8 resumption, native lowering,
  production I/O, or concurrency

#### Focused verification

```bash
python3 specs/tools/architecture_acceptance.py check
python3 tools/check_docs.py
```

#### Exit criterion

Two independent readers can use only the frozen behavior, project policy,
baseline, and expected facts to classify the valid, centralized, and
helper-splitting changes identically. The checker proves complete traceability,
deterministic ordering, fixed budgets, and no hidden acceptance choice. If the
gate passes, UC-007 and its requirements are accepted and M24 is eligible for
explicit activation; otherwise the rejection or required amendment is recorded
before new work.

#### Delivered

- Froze an exact 24-operation R1 workspace and complete `CancelJob` operation
  roles for the valid, centralized, and helper-splitting candidates
- Froze six transport-independent behavior cases with one atomic jobs-store
  operation and no clock, network, or telemetry effect
- Froze five architecture groups, eight policy rules, one accepted dispatch
  baseline, seven primitive metrics or sets, and complete/unchecked coverage
- Froze exact structured results and bounded compact text that accept the valid
  candidate and reject both behaviorally correct architecture regressions
- Identified pinned Rust, Go, Python, and TypeScript comparison tools and fixed
  zero false/missed findings plus graph and encoded-output budgets
- Added a dependency-free checker that derives every result from semantic
  facts, verifies the digest graph and review records, and rejects 37 mutations
- Recorded two independent approvals of fixture-set digest
  `ab362d96d89cbba779743dd8a3050b2bd4452ff6daddf3e7ae65109207f7e3ed`
  and review-subject digest
  `fcb454729e6c2c228802d471d97c1eecd7abc793da407b0ced2bbd76fe9624cf`
- Accepted UC-007 and its six requirements without defining M24 protocol rules
  or implementing compiler behavior

### M24 — Architectural regression contract

**Status:** Complete

#### Scope

- Accept the smallest numbered language and protocol rules required by M23
- Freeze metric meanings, aggregate scopes, policy and baseline governance,
  coverage, ordering, budgets, diagnostics, and completion evidence
- Freeze canonical snapshot and delta encodings plus compact deterministic
  agent renderings for all M23 candidates
- Freeze rejecting cases for boundary, authority, hotspot, cycle, stale
  baseline, unauthorized governance, and incomplete-analysis failures

#### Non-scope

- Rust implementation
- Universal architecture policy or default thresholds for every project
- Automatic repair or official agent evidence

#### Focused verification

```bash
python3 specs/tools/architecture_contract.py check
python3 tools/check_docs.py
```

#### Exit criterion

Two independent readers can predict every structured fact, policy result,
compact text summary, and commit or rejection outcome without using incidental
implementation behavior.

#### Delivered

- Accepted two language and six protocol rules traced to all six M23
  requirements, without adding AIL grammar or compiler behavior
- Froze exactly seven metrics, four aggregate scopes, 23 transport-independent
  shapes, ten diagnostic codes, M23 governance, ordering, and fixed budgets
- Froze four exact operation results plus one read-only incomplete result and
  23 executable scenarios,
  including module and dependency-component aggregate evidence
- Added a standalone dependency-free checker that digest-locks and re-derives
  the M23 inputs and rejects 36 malformed or weakened contract mutations
- Accepted the M24 contract while retaining **Active** as the handoff marker;
  this run stops before M25 and does not activate implementation

### M25 — Architectural snapshot and agent rendering

**Status:** Complete

#### Scope

- Implement the accepted revision-bound facts, contributors, coverage, and
  aggregate scopes for the M23 starting revision
- Implement deterministic budgets and explicit incomplete results
- Render the bounded compact agent summary from the same structured facts
- Match repeated snapshot and rendering fixtures exactly

#### Non-scope

- Cross-revision regression classification or denied transaction enforcement
- Metrics, relationships, or language constructs outside the M24 contract

#### Focused verification

```bash
cargo test --workspace --test m25_architecture_snapshot
python3 specs/tools/architecture_contract.py check
python3 tools/check_docs.py
```

#### Exit criterion

The compiler returns the frozen structured snapshot and compact text for the
starting revision, including exact contributors, policy context, coverage, and
budgets. Repeated requests are identical and incomplete analysis cannot render
as clean.

#### Delivered

- Added a read-only Rust snapshot API over validated revision-bound semantic
  graph, policy, baseline, coverage, and exception facts
- Derived all seven accepted metrics for executable-unit, module,
  dependency-component, and architecture-group scopes with exact ordered
  contributors and deterministic component identities
- Matched the frozen R1 structured response, compact text, and snapshot digest
  exactly and returned identical results for repeated requests
- Accounted for all five fixed budgets in precedence order and returned bounded
  incomplete results for coverage or budget exhaustion without a snapshot,
  edits, or clean classification
- Preserved every prior compiler and 37-case public runtime result; added no
  syntax, runtime behavior, cross-revision comparison, policy enforcement, or
  transaction rollback
- Retained **Active** as the handoff marker and stopped before M26

### M26 — Architectural delta, policy, and atomic enforcement

**Status:** Complete

#### Scope

- Implement compatible revision comparison and identity mapping for accepted
  architectural facts
- Implement the accepted no-growth, dependency, capability, state, cycle,
  baseline, exception, and governance rules
- Match the valid, centralized, and helper-splitting candidate classifications
- Integrate denied and incomplete results with atomic candidate rollback and
  revision-bound completion evidence
- Render actionable compact explanations with exact contributors and next
  inspection handles

#### Non-scope

- Automatic architecture design or repair
- Unaccepted policy vocabulary, broad refactors, or official agent evidence

#### Focused verification

```bash
cargo test --workspace --test m26_architecture_delta
python3 specs/tools/architecture_contract.py check
python3 tools/check_docs.py
```

#### Exit criterion

The valid change commits, while the centralized and superficial-splitting
changes return their exact structured and compact causes and publish no
revision. Unchanged accepted debt remains visible without becoming a new
failure, and unauthorized policy or baseline changes cannot create a pass.

#### Delivered

- Added compatible four-scope snapshot comparison, canonical identity mapping,
  exact snapshot and delta digests, and deterministic ordered scope changes
- Evaluated all eight accepted project rules from trusted policy, baseline,
  exception, authorization, coverage, graph, and behavior facts
- Matched the valid, centralized, and helper-splitting responses plus all 23
  rejection, governance, exception, and budget scenarios byte-for-byte
- Kept existing dispatch debt visible without classifying it as new growth and
  applied exact exception matching without suppressing stale or unauthorized
  governance failures
- Published exactly one valid child revision and retained the unchanged base
  for every denied, incomplete, invalid-behavior, or governance-invalid request
- Bound completion evidence to the exact base, committed child, snapshot and
  delta digests, policy and baseline, coverage, budgets, behavior, and outcome
- Preserved all prior compiler behavior and all 37 public runtime cases without
  adding syntax, runtime effects, dependencies, automatic repair, or M27 work

### M27 — Non-official architecture-feedback pilot

**Status:** Complete

#### Scope

- Run a small usability pilot against the locked M23 task after M26 passes
- Record the exact agent, version, mode, prompt, permissions, tools, thread or
  run identity, compiler outputs, edits, validations, and repairs
- Test whether an operator can use compact output and structured drill-down to
  avoid or repair the seeded regression
- Permit Codex, Amp, or another explicitly named operator configuration

#### Non-scope

- Official comparative evidence, statistical claims, or AIL success targets
- Changing fixtures, policy, baselines, prompts, or expected results after
  observing a run
- Resuming M8 or selecting a measured agent by convenience

#### Focused verification

```bash
python3 benchmarks/tools/harness.py verify-architecture-pilot
python3 tools/check_docs.py
```

#### Exit criterion

The repository contains at least one complete, replayable, explicitly
non-official run showing whether the feedback was actionable, with every input,
output, edit, check, repair, and limitation accounted for. The result supports
a later go, revise, or stop decision but makes no comparative project claim.

#### Delivered

- Retained one Amp `0.0.1785901465-g80049c` medium-mode run with the exact
  prompt, threads, orb environment, permissions, restrictions, authorized
  toolchain change, tools, original outputs, edit, checks, and limitations
- Kept the compact centralized rejection as the first input and recorded the
  four structured findings and their exact dispatch, capability, and state
  contributors used for drill-down
- Preserved the repaired candidate byte-for-byte at SHA-256
  `8bfd3f223b2b15f4eca72eba215e25450d61ded2cd22ae0c6d3223fa25df567d`
  after final comparison
- Verified that the domain handler owns cancellation, jobs-store access, and
  jobs state while transport only registers, adapts, and forwards the request
- Replayed the retained candidate through the M26 transaction, passed all six
  behavior cases, published `arch-r2-valid`, and retained `arch-r1`
- Recorded that the repair differs from the locked valid candidate only by an
  empty `changed_units` field and one redundant domain-to-contract `type-use`
  edge, without editing it after comparison
- Added a schema, external manifest lock, semantic verifier, one acceptance
  test, and eight rejection tests covering changed bytes or digests, missing
  metadata, noncanonical JSON, changed compact or behavior evidence, official
  claims, transport-owned repair, and unrecorded post-comparison changes
- Recorded the initial missing-Cargo result and later explicit authorization to
  install Rust 1.87.0 before the required focused test passed; no failed step or
  changed restriction was omitted
- Classified the pilot as one non-official run supporting no comparative,
  statistical, or broader project claim

### M28 — Iterative-evolution acceptance package

**Status:** Active

#### Scope

- Freeze the language-independent boundary and operation inventory for the
  proposed mature job service
- Freeze six to eight cumulative product requirements, each starting from the
  accepted revision produced by the previous change
- Define public and held-back behavior, state, ordered effects, compatibility,
  architecture expectations, and human-review evidence for every change
- Define fresh-context task envelopes that do not carry conversation memory or
  answer-bearing summaries between changes
- Define per-change and cumulative measurements for context, queries, edits,
  validation, repair, regressions, elapsed work, and review evidence
- Freeze the scale hypothesis and language-independent pressure criteria without
  enforcing source-line or file quotas
- Identify normal Rust, Go, Python, and TypeScript tools and the equivalence
  rules M29 must implement
- Record preliminary AIL feasibility risks without selecting syntax, semantics,
  or implementation; M30 remains the only gap-selection milestone
- Freeze go, revise, and stop conditions, fixed budgets, exact expected
  classifications, and two independent digest-bound reviews
- Add a dependency-free package checker and rejection mutations covering the
  complete acceptance evidence
- Accept, revise, or reject UC-008 and its candidate requirements only after the
  package passes

#### Non-scope

- AIL grammar, specification, compiler, interpreter, or protocol implementation
- Treating direct calls, modules, iteration, collections, or additional
  capabilities as accepted requirements before the gate
- Building the four complete baseline workspaces or collecting agent evidence
- Enforcing approximately 2,000 lines or five to eight files
- Choosing one universal architecture or source layout
- Native lowering, LLVM, production deployment, self-hosting, or M8 resumption
- Official comparative evidence or an agent-efficiency claim

#### Focused verification

```bash
python3 benchmarks/tools/iterative_evolution.py check
python3 tools/check_docs.py
```

#### Exit criterion

Two independent readers can use only the frozen service, ordered requirements,
reference trajectory, bounded rejecting mutations, oracles, architecture policy,
task envelopes, measurement rules, and baseline plan to predict every expected
behavior, architecture, package-integrity, and cumulative classification. The
checker proves complete traceability, deterministic ordering and encoding,
digest closure, fixed budgets, scale-hypothesis non-enforcement, and rejection
of weakened or incomplete packages. The result explicitly accepts, revises, or
rejects UC-008 before M29 starts.

### M29 — Strong baseline workspaces and trajectories

**Status:** Planned

Conditional on M28 acceptance.

#### Scope

- Build idiomatic Rust, Go, Python, and TypeScript starting services using their
  normal compiler, language-server, formatter, linter, test, refactoring, and
  static-analysis tools
- Implement and freeze one reference revision for every accepted cumulative
  requirement in each language
- Prove cross-language behavior, state, ordered-effect, compatibility, and
  architecture-policy parity at every revision
- Record observed natural source and file scale without changing the workload
  to fit the planning range
- Stage implementation: calibrate one idiomatic baseline first, record a go,
  revise, or stop result for service class and maintenance pressure, and only
  then complete the other three
- Freeze answer-free task starts, public and held-back checks, source and tool
  manifests, and deterministic reference results

#### Non-scope

- AIL language or compiler changes
- Agent trials or comparative measurements
- Forcing identical source architecture across languages
- Changing M28 behavior or architecture expectations after implementations are
  observed without returning through its revision gate

#### Focused verification

```bash
python3 benchmarks/tools/iterative_evolution.py verify-baselines
python3 tools/check_docs.py
```

#### Exit criterion

All four idiomatic baseline trajectories satisfy every frozen revision oracle,
their answer-free starts expose equivalent task information, and their normal
tools and manifests are digest locked. M29 records an explicit go, revise, or
stop result for natural service class and maintenance pressure. Padding,
artificial file splitting, or a material workload revision returns to M28 rather
than being hidden in a language-specific implementation.

### M30 — AIL gap selection

**Status:** Planned

Conditional on M29.

#### Scope

- Compare the accepted workload with the authoritative compiler's actual
  syntax, static semantics, interpreter, revisions, graph, and architecture
  protocol
- Identify the smallest language and compiler gaps that prevent a natural AIL
  service and cumulative sequence
- Tie every proposed gap to one accepted requirement change and one component
  of agent work or correctness risk
- Select a bounded contract and implementation sequence, revise the workload,
  or stop if useful evidence requires broad conventional feature parity first

#### Non-scope

- Implementing syntax or compiler behavior
- Selecting features because mainstream languages conventionally have them
- Native lowering or production runtime work

#### Focused verification

```bash
python3 tools/check_docs.py
```

#### Exit criterion

The repository records one bounded accepted direction for M31, or an explicit
revise or stop result. Every proposed language or protocol behavior names the
accepted workload step it enables and the later cumulative comparison that can
discriminate its value.

### M31 — Iterative-workload contract

**Status:** Planned

Conditional on M30.

#### Scope

- Accept only the grammar, formatting, static and runtime semantics,
  diagnostics, semantic facts, protocol behavior, and canonical fixtures
  selected by M30
- Define deterministic behavior, coverage, ordering, revision identity, and
  rejection results sufficiently for independent implementation
- Extend dependency-free contract checking over every accepted behavior

#### Non-scope

- Rust compiler implementation
- Features not required by the accepted sequence
- Native code generation or production adapters

#### Focused verification

```bash
python3 specs/tools/iterative_contract.py check
python3 tools/check_docs.py
```

#### Exit criterion

Two independent readers can predict canonical source, static results, runtime
observations, semantic relationships, and primary diagnostics for every new
fixture without relying on incidental compiler behavior.

### M32 — Authoritative compiler implementation

**Status:** Planned

Conditional on M31.

#### Scope

- Implement the accepted M31 syntax, formatting, checking, diagnostics,
  semantic relationships, protocol facts, and deterministic interpretation
- Preserve immutable revisions, complete coverage, ordered effects, structured
  faults, architecture analysis, and prior accepted behavior
- Add executable checks for every delivered rule and rejection

#### Non-scope

- Unaccepted workload features
- Optimizing or replacing the interpreter
- Native lowering, LLVM, JIT, production runtime, or deployment

#### Focused verification

```bash
cargo test --workspace
python3 specs/tools/iterative_contract.py check
python3 tools/check_docs.py
```

#### Exit criterion

The authoritative compiler matches every M31 fixture without fixture-specific
behavior, all prior compiler and workload checks pass, and the accepted
semantics are exposed through revision-safe structured compiler facts.

### M33 — Canonical AIL service and cumulative evidence

**Status:** Planned

Conditional on M32.

#### Scope

- Build the natural canonical AIL starting service and every accepted reference
  revision
- Keep this frozen reference trajectory separate from later measured
  trajectories; only the common initial revision may start a measured run
- Freeze behavior, state, ordered effects, compatibility, semantic impact,
  architecture snapshots and deltas, completion evidence, and task starts for
  the complete trajectory
- Extend the shared verifier so every revision is checked through the same
  language-independent oracle as the strong baselines

#### Non-scope

- Agent trials
- Retuning the workload to improve AIL results
- Native execution or production packaging

#### Focused verification

```bash
python3 benchmarks/tools/iterative_evolution.py verify-ail
cargo test --workspace
python3 tools/check_docs.py
```

#### Exit criterion

The AIL starting workspace and every cumulative reference revision pass the
shared oracle and accepted compiler contracts, all task starts are answer-free,
and the complete trajectory is digest locked before any agent pilot.

### M34 — Non-official full-trajectory pilots

**Status:** Planned

Conditional on M33.

#### Scope

- Run a small number of initiated complete or terminal trajectories with a
  named agent configuration
- Start every change in a fresh context from the actual accepted revision
  produced by the previous change in that same trajectory, never from the
  corresponding reference revision
- Record prompts, tools, context, queries, edits, validations, repairs,
  compiler evidence, final revisions, and limitations
- Retain failures and timeouts as terminal outcomes with later changes marked
  not reached; makeup runs receive new identities and never replace originals
- Use failures to revise task wording, compiler output, measurement, or evidence
  before campaign freeze

#### Non-scope

- Official comparison, statistics, or AIL success claims
- Changing locked expectations without returning through the owning milestone
- Selecting a favorable run as representative

#### Focused verification

```bash
python3 benchmarks/tools/iterative_evolution.py verify-pilots
python3 tools/check_docs.py
```

#### Exit criterion

Every initiated pilot trajectory is replayable through completion or its
terminal outcome and accounts for every input, reached change, validation,
repair, revision, not-reached change, and limitation. The project records a go,
revise, or stop decision for campaign readiness without making a comparative
claim.

### M35 — Comparative campaign readiness

**Status:** Planned

Conditional on M29 and M34.

#### Scope

- Freeze the measured agent, model, prompts, fresh-context protocol,
  environments, permissions, tools, task order, trajectory order, trial counts,
  safety limits, target-calculation formula, exclusions, replay, terminal-run
  accounting, and evidence closure
- Freeze the rule for computing baseline-derived AIL targets and the separate
  continuation decision required after official baseline evidence, without
  assigning target values before that evidence exists
- Prove fake and dry success, failure, timeout, limit, malformed-evidence, and
  changed-input paths
- Require an explicit launch decision after readiness passes

#### Non-scope

- Official trials
- Retuning targets after AIL results
- Claims from readiness infrastructure

#### Focused verification

```bash
python3 benchmarks/tools/iterative_evolution.py verify-readiness
python3 tools/check_docs.py
```

#### Exit criterion

The complete campaign can be executed and independently verified without
changing locked inputs or exposing held-back answers, and maintainers record an
explicit launch, revise, or stop decision.

### M36 — Official iterative comparison

**Status:** Planned

Conditional on M35 and an explicit launch decision.

#### Scope

- Run the locked AIL, Rust, Go, Python, and TypeScript trajectories at the
  frozen trial counts and ordering in two phases
- Phase one: collect and independently verify all official Rust, Go, Python,
  and TypeScript trajectories, including every terminal result
- Compute and digest-lock AIL targets from phase-one evidence using only the
  preregistered formula, then require an explicit continue or stop decision
- Phase two: only after continuation, collect the locked AIL trajectories
- Report raw per-change and cumulative correctness, context, repair,
  regression, elapsed-work, architecture, and review-evidence facts
- Apply the frozen go, revise, and stop decision rule without changing targets

#### Non-scope

- Changing task inputs, tools, targets, or exclusions after results are seen
- Inferring causality beyond the locked experiment
- Treating source brevity, native performance, or one successful trajectory as
  the project result

#### Focused verification

```bash
python3 benchmarks/tools/iterative_evolution.py verify-campaign
python3 tools/check_docs.py
```

#### Exit criterion

The repository contains complete independently verifiable evidence for every
initiated trajectory and reports the frozen factual and decision outcomes. A
stop after phase one preserves all official baseline evidence and collects no
AIL result. Only a completed valid two-phase campaign may support a bounded
comparative claim about cumulative agent change cost.

## Completed scaling direction: architectural regression control

[UC-007](use-cases/UC-007-architectural-regression-control.md), its
[accepted requirements](requirements/architectural-health.md), and the proposed
[architectural health manifest](architecture-health.md) form a separate scaling
track. They are not part of M19–M21 and do not expand a milestone implicitly.

ADR 0006 selected UC-007 acceptance preparation as M23. Its starting workspace,
cancel-job behavior, project policy, minimal metric set, fixtures, baseline
comparison, and budgets passed the gate, so UC-007 and its requirements are
accepted. M24 accepted its bounded contract, M25 delivered the single-revision
snapshot, M26 delivered cross-revision enforcement, and M27 completed one
bounded non-official usability pilot. ADR 0007 subsequently selected iterative
service evolution as the next conditional scaling direction and activated only
its M28 acceptance package.

Architectural-health implementation should follow the core semantic graph and
revision protocol. It does not block M19 through M21.

## Selected direction after M27

[ADR 0007](decisions/0007-prepare-iterative-service-evolution.md) selects the
proposed [UC-008 iterative service evolution](use-cases/UC-008-iterative-service-evolution.md)
as the next acceptance direction. The north-star comparison applies six to
eight cumulative requirements to a naturally medium-sized service, starts each
change in a fresh agent context, and compares all initiated complete or terminal
trajectories against strong normal Rust, Go, Python, and TypeScript tooling.

Only M28 is active. UC-008 and its requirements remain proposed, M29 through
M36 are conditional, and no AIL syntax or implementation is authorized until
the acceptance and gap-selection gates pass.

## Other candidate directions

This is an unordered set of possible capabilities, not an operational roadmap.
Every candidate must pass the successor-selection gate; none is active:

- **Representative comparative validation:** fresh held-out agent changes,
  strong mainstream toolchains, locked correctness and review rules, and an
  explicit go, revise, or stop decision.
- **Broader semantic oracle:** only the constructs required by a selected use
  case, with canonical source, static and dynamic semantics, diagnostics,
  deterministic interpretation, and conformance evidence.
- **Agent protocol expansion:** semantic queries or validated operations that
  remove measured context, consequence, validation, or repair work on a
  representative change.
- **Safety and controlled execution:** memory, concurrency, cancellation,
  resource, nondeterminism, and replay behavior selected through accepted use
  cases rather than general feature parity.
- **Production execution:** lowering, runtime packaging, debugging, and foreign
  boundaries only when a concrete comparison or deployment envelope requires
  them. No backend, including LLVM, is selected here.
- **Ecosystem work:** packages, supply-chain policy, deployment, and compatible
  target emission only when they enable an accepted workload or remove measured
  agent work.

Self-hosting may eventually exercise compiler-shaped workloads, but it is not a
project objective or maturity gate by itself.
