# Initial requirements for the job-submission reference slice

Status: **Accepted 2026-07-18**

Documentation layer: requirements derived from
[UC-001](../use-cases/UC-001-request-validation-and-persistence.md) and
[UC-003](../use-cases/UC-003-public-schema-evolution.md). These are accepted
project requirements. They are not language rules and do not select AIL syntax,
memory management, runtime architecture, protocol transport, or compiler
implementation.

## Accepted reference workload

The first reference system is the transport-independent job-submission service
defined by UC-001, evolved by UC-003. The workload and both use cases were
accepted on 2026-07-18.

This slice is selected because it combines:

- explicit public data and closed domain outcomes;
- bounded validation;
- one state-changing capability and a fully ordered effect trace;
- a representative cross-workspace public schema change;
- compatibility and serialization obligations;
- complete semantic-impact analysis; and
- revision-safe completion evidence.

It deliberately avoids concurrency, networking infrastructure, production
datastores, clocks, randomness, and a production runtime. This keeps the first
comparison focused on context discovery, consequence analysis, validation,
repair, and regression control.

## Traceability summary

| Requirement | UC-001 | UC-003 | Primary constraint |
| --- | :---: | :---: | --- |
| APP-001 | yes | inherited | bounded request validation |
| APP-002 | yes | inherited | conditional persistence |
| APP-003 | yes | yes | closed public outcomes |
| APP-004 |  | yes | versioned compatibility |
| APP-005 | yes | yes | review and operational evidence |
| LANG-001 | yes | yes | explicit public contracts |
| LANG-002 | yes | yes | closed errors and exhaustive handling |
| LANG-003 | yes | yes | explicit capabilities and effects |
| LANG-004 | yes | yes | deterministic observable ordering |
| LANG-005 |  | yes | statically knowable schema consequences |
| PROTO-001 | yes | yes | elaborated semantic inspection |
| PROTO-002 | yes | yes | complete impact query |
| PROTO-003 |  | yes | revision-scoped identities |
| PROTO-004 |  | yes | validated structural change |
| PROTO-005 | yes | yes | structured diagnostics and evidence |
| NFR-001 | yes | yes | reproducible benchmark |
| NFR-002 | yes | yes | context budget |
| NFR-003 | yes | yes | repair and regression budget |
| NFR-004 | yes | yes | latency and throughput envelope |
| NFR-005 | yes | yes | startup, memory, and deployment envelope |

## Application and operability requirements

### APP-001 — Bounded validation before effects

Status: **Accepted 2026-07-18**

Source use cases: UC-001; inherited unchanged by UC-003.

Requirement: The reference handler must validate every field against the bounds
in UC-001 before invoking a capability. It must return all applicable validation
issues in the specified deterministic order, at most one issue per field.
Invalid structured input must produce no persistent state change and no
capability call.

Rationale and agent change cost: A complete, local validation contract reduces
context discovery and makes accidental pre-validation effects a directly
testable regression.

Acceptance evidence: The shared fixture corpus covers each bound, every field,
multi-field failure, boundary values, empty payload, and valid non-ASCII task
text. For every fixture, the result, issue order, store-call count, and final
state must match the language-independent oracle.

Target milestone and scope: Accepted as application behavior in M0 and exercised
by the M1 fixture corpus. Constrains the application contract, test oracle, and
later standard-library/runtime adapters; it does not prescribe a validation
syntax or library.

Dependencies, conflicts, and open questions: Depends on LANG-004 and PROTO-005.
Task text is bounded by Unicode scalar values for this reference workload.

### APP-002 — One conditional persistent effect

Status: **Accepted 2026-07-18**

Source use cases: UC-001; inherited unchanged by UC-003.

Requirement: For a valid request, the handler must issue exactly one
`insert_if_absent` operation in the `jobs` namespace and map `inserted`,
`duplicate`, and `unavailable_before_commit` to the specified public result and
postcondition. It must not retry, generate an identifier, or access another
external capability.

Rationale and agent change cost: A narrow effect and outcome contract bounds
consequence analysis. It lets an agent and reviewer distinguish intended state
change from incidental or ambient authority.

Acceptance evidence: A deterministic capability double records arguments,
order, outcome, and state. Nominal, duplicate, and unavailable fixtures each
produce the exact UC-001 trace and postcondition. Static evidence lists no
additional capability requirement.

Target milestone and scope: Accepted as application behavior in M0, exercised
by M1 fixtures, and used as semantic input to M11. Constrains the application
contract, capability adapter contract, and test harness, not the datastore
technology.

Dependencies, conflicts, and open questions: Depends on LANG-003 and LANG-004.
Ambiguous commit status is explicitly excluded and must become a separate
requirement if introduced.

### APP-003 — Closed public result contract

Status: **Accepted 2026-07-18**

Source use cases: UC-001 and UC-003.

Requirement: The handler's public result must contain exactly the success and
domain-failure variants declared by the active schema version. Validation,
duplicate, and known pre-commit persistence failures must be returned as data,
not delivered through an unchecked exception or an undocumented sentinel.

Rationale and agent change cost: A closed result set reduces hidden control-flow
reconstruction and makes producers, propagators, and handlers mechanically
checkable after a change.

Acceptance evidence: Static analysis enumerates the full result set. Fixtures
exercise every variant. Removing a handler for any variant from a seeded
consumer causes a stable static failure rather than a runtime-only miss.

Target milestone and scope: Accepted as application behavior in M0 and specified
as language error semantics in M11. Constrains application contracts and
language error semantics; does not decide the syntax of results or the treatment
of language/runtime faults.

Dependencies, conflicts, and open questions: Depends on LANG-002 and PROTO-001.
The catchability and process behavior of non-domain faults remain unresolved.

### APP-004 — Explicit public and persisted schema compatibility

Status: **Accepted 2026-07-18**

Source use cases: UC-003.

Requirement: The V2 priority change must use stable version and field
identities. V2 producers must provide `Low`, `Normal`, or `High` explicitly.
During the declared compatibility window, V1 requests and V1 persisted jobs
must be recognized by version and adapted explicitly to `Normal`; missing or
unknown V2 priority must not be silently treated as V1.

Rationale and agent change cost: Explicit compatibility policy prevents an agent
from inferring migration behavior from field presence or scattered conventions,
reducing consequence analysis and regression risk.

Acceptance evidence: Golden V1 and V2 request, response, and stored-record
fixtures exercise all priority identities, missing and unknown priority,
version selection, and V1 adaptation. Round trips preserve declared identities.
A semantic diff reports the compatibility addition and eventual adapter removal
as distinct changes.

Target milestone and scope: Accepted as application behavior in M0, exercised
by M1 fixtures, and specified as schema semantics in M11. Constrains
public/stored data contracts, fixture encoding, and compatibility tests. It does
not select a general migration framework or wire format.

Dependencies, conflicts, and open questions: Depends on LANG-001, LANG-005,
PROTO-002, and
[the accepted JSON fixture format](../benchmarks/job-service-fixtures.md). The
response-version policy is fixed: V1 requests receive V1 responses, and V2
requests receive V2 responses.

### APP-005 — Complete human-review and operational evidence

Status: **Accepted 2026-07-18**

Source use cases: UC-001 and UC-003.

Requirement: Completion of either representative change must produce evidence
that identifies the exact source revision, public contract, domain outcomes,
capability authority, ordered observable effects, final test state, and all
semantic and textual changes. UC-003 evidence must additionally state impact
coverage, compatibility obligations, changed identities, and unanalyzed
external dependencies.

Rationale and agent change cost: Review artifacts externalize consequence and
validation work. They lower the risk that correctness depends on inaccessible
agent reasoning or on a reviewer reconstructing a semantic change from raw
files.

Acceptance evidence: A machine-checkable completion manifest contains every
required field and references passing results for the same revision. An
independent reviewer can account for each changed semantic identity and
observable behavior solely from canonical source, the manifest, and referenced
evidence.

Target milestone and scope: M2 benchmark contract and M11 diagnostic and
protocol contract. Compiler implementation belongs to numbered milestones added
after M13. Constrains protocol, benchmark, and governance surfaces; it does not
require one report presentation.

Dependencies, conflicts, and open questions: Depends on PROTO-001 through
PROTO-005. The long-term signing, retention, and provenance policy is deferred.

## Language requirements

### LANG-001 — Explicit, stable public contracts

Status: **Accepted 2026-07-18**

Source use cases: UC-001 and UC-003.

Requirement: AIL must require every cross-unit request, result, error,
capability, and persisted schema boundary in the reference slice to have an
explicit statically checkable contract with stable semantic identities.
Implementations may infer local intermediate facts only when the compiler can
expose the complete elaborated result.

Rationale and why this belongs in the language: The completeness of boundaries
must hold for all conforming programs, not only projects that adopt a linter or
annotation convention. This reduces context discovery and makes downstream
dependence a semantic fact.

Acceptance evidence: Proposed fixtures lacking a required public field type,
result type, domain outcome, or capability contract fail with a stable
diagnostic. Valid fixtures expose the complete contract through PROTO-001.
Renaming a display name does not silently change a stable serialization
identity.

Target milestone and scope: M11 proposed normative semantics and fixture
contract. Constrains the language and compiler protocol, not concrete syntax.

Dependencies, conflicts, and open questions: Depends on the future module,
visibility, schema-identity, and type rules. Which boundaries require persistent
identities beyond source revisions remains open.

### LANG-002 — Closed domain errors and exhaustive consumption

Status: **Accepted 2026-07-18**

Source use cases: UC-001 and UC-003.

Requirement: Domain failures in the reference slice must be represented by a
closed statically known set in the public result contract. A consumer that
selects behavior by result variant must either handle every current variant or
use an explicit propagation or intentional catch-all construct whose semantic
effect is visible.

Rationale and why this belongs in the language: Only the language can guarantee
that adding or removing a domain outcome creates a complete, checkable change
surface rather than a convention-dependent runtime path. This reduces missed
consumers and repair cascades.

Acceptance evidence: Conformance candidates cover complete handling,
propagation, and a seeded missing variant. The missing case produces one stable
primary diagnostic and appears in the UC-003 impact report.

Target milestone and scope: M11 error and matching semantics. Constrains the
language and compiler; it does not decide fault handling or exact syntax.

Dependencies, conflicts, and open questions: Depends on closed data types and
matching semantics. Whether intentional catch-all handling is permitted at
public boundaries remains open.

### LANG-003 — Explicit capability authority and public effects

Status: **Accepted 2026-07-18**

Source use cases: UC-001 and UC-003.

Requirement: Code in the reference handler may invoke the jobs store only
through an explicitly supplied capability whose accessible instance or
namespace is statically identifiable. Its public contract must expose the
resulting persistent effect. Time, randomness, network, filesystem,
environment, telemetry, and other ambient authority must be absent unless added
to the public contract.

Rationale and why this belongs in the language: Authority and effects must be
unavoidable semantic properties for consequence analysis to be complete.
Tooling alone cannot reliably recover them from ambient APIs or hidden global
state.

Acceptance evidence: A valid fixture declares and uses only jobs insertion.
Seeded attempts to read a clock, access another store namespace, or call an
effectful operation from an undeclared context fail with stable diagnostics.
PROTO-001 returns the elaborated capability instance and effect path.

Target milestone and scope: M11 minimal capability and effect semantics.
Constrains the language and protocol, not capability-passing syntax or runtime
representation.

Dependencies, conflicts, and open questions: Depends on module boundaries and
the initial non-polymorphic effect model. Delegation, revocation, and effect
polymorphism are deferred.

### LANG-004 — Deterministic observable evaluation and effect order

Status: **Accepted 2026-07-18**

Source use cases: UC-001 and UC-003.

Requirement: AIL must define enough evaluation and effect-ordering semantics for
the reference handler that identical initial state, structured input, and
supplied capability outcomes produce the same returned result, final state, and
ordered capability trace on every conforming implementation.

Rationale and why this belongs in the language: Deterministic tests and portable
diagnoses cannot depend on an implementation's incidental evaluation order.
The guarantee reduces validation and repair work across targets.

Acceptance evidence: Conformance candidates cover validation order, zero effects
on invalid input, one effect on valid input, result selection after the store
outcome, and no trailing effect. Repeated interpreter and candidate
implementation runs match the logical trace oracle.

Target milestone and scope: M11 dynamic-semantics contract followed by the
numbered semantic-oracle milestones added after M13. Constrains the language and
conformance suite. Bit-reproducible build output, allocator behavior, and
production scheduling are separate concerns.

Dependencies, conflicts, and open questions: Depends on the future observable
determinism contract and evaluation-order rules. It must not accidentally define
unobservable internal order.

### LANG-005 — Statically knowable schema consequence graph

Status: **Accepted 2026-07-18**

Source use cases: UC-003.

Requirement: AIL constructs used to define, construct, destructure, serialize,
adapt, store, or expose the reference public schemas must preserve semantic
relationships that allow a compiler to enumerate every statically present
affected workspace consumer of the priority change. Ordinary language features
must not permit hidden reflective or generated access that invalidates a claim
of completeness without making that loss of analyzability explicit.

Rationale and why this belongs in the language: A complete impact query requires
language-level analyzability. A protocol cannot manufacture missing edges when
the language makes dependencies dynamically opaque.

Acceptance evidence: A fixture workspace contains at least the semantic roles
listed by UC-003. The compiler's dependency graph reaches every seeded role,
including aliases and renamed display symbols. Any intentionally opaque
boundary is represented as an explicit incomplete-coverage edge.

Target milestone and scope: M11 schema, static-semantics, and impact-query
contract. Compiler implementation belongs to numbered milestones added after
M13. Constrains language analyzability and protocol exposure, not source file
layout or code generation strategy.

Dependencies, conflicts, and open questions: Depends on modules, generics,
serialization identities, and rules for generated or foreign code. The first
slice may exclude general reflection rather than model it.

## Compiler semantic-interface requirements

### PROTO-001 — Revision-bound elaborated semantic inspection

Status: **Accepted 2026-07-18**

Source use cases: UC-001 and UC-003.

Requirement: For a requested handler, public schema, result, or capability at a
specific source revision, the compiler interface must return its explicit and
inferred types, closed variants, effects, capability instances, direct semantic
dependencies, serialization identities, and relevant invariants in a stable
machine-readable form.

Rationale and agent change cost: Elaborated inspection lets agents retrieve
semantic facts directly instead of loading implementation files and
reconstructing inference, reducing context and diagnostic work.

Acceptance evidence: Protocol examples return every listed fact for the
reference fixtures, use stable field categories and ordering, identify the
source revision, and agree with static checking and execution. A human-readable
rendering can be derived without adding semantic information.

Target milestone and scope: Minimal contract in M11; implementation in numbered
core-protocol milestones added after M13. Constrains the transport-independent
protocol schema and compiler, not JSON-RPC or another transport.

Dependencies, conflicts, and open questions: Depends on LANG-001 through
LANG-005. Budgeting and transitive context expansion beyond direct dependencies
remain separate requirements.

### PROTO-002 — Complete impact report

Status: **Accepted 2026-07-18**

Source use cases: UC-001 and UC-003.

Requirement: Given a proposed public data change at revision R1, the compiler
must return three lists:

- `must_change`: every location in the current build that definitely needs an
  edit, with no unnecessary entries;
- `review`: locations that may need attention, with a reason for each; and
- `unchecked`: external systems or unavailable dependency source that the
  compiler cannot inspect.

The current build includes application source, tests, fixtures, checked-in
schemas, generated code used by the build, and locked dependency source
available to the compiler. A generated-code result must also identify its
editable generator input or schema.

Rationale and agent change cost: Complete categorized impact directly reduces
context discovery, consequence analysis, and missed downstream updates. Stable
categories are cheaper to consume and audit than prose or a raw reference list.

Acceptance evidence: Before the UC-003 edit, `must_change` contains every seeded
location that needs an edit and nothing else. `review` contains no more than two
unnecessary entries. `unchecked` lists known external consumers and unavailable
source without claiming they were analyzed. The report also confirms that
storage authority and call ordering do not change.

Target milestone and scope: Contract in M11; implementation in numbered
core-protocol milestones added after M13. Constrains protocol and compiler
analysis. It does not require mainstream baseline tools to expose equivalent
data.

Dependencies, conflicts, and open questions: Depends on LANG-005 and PROTO-003.
Larger workloads may set a different `review` limit, but must do so before their
benchmark results are inspected.

### PROTO-003 — Revision-scoped handles and identity mapping

Status: **Accepted 2026-07-18**

Source use cases: UC-003.

Requirement: Every semantic query, diagnostic, and edit must identify its source
revision. Node and source-symbol handles from R1 must not be silently applied to
R2. A successful edit must return an identity map classifying relevant R1
identities as surviving, replaced, removed, or unmapped and identifying new R2
identities; persistent schema identities remain distinct from revision handles.

Rationale and agent change cost: Revision discipline prevents stale inspection
and edits from producing plausible but invalid changes, reducing repair work and
review ambiguity.

Acceptance evidence: Protocol examples cover a surviving identity, changed
schema, removed identity, new priority type, stale-handle rejection, and
explicit remapping. Every UC-003 completion artifact is bound to R1 or R2 as
appropriate.

Target milestone and scope: Minimal contract in M11; implementation in numbered
core-protocol milestones added after M13. Constrains compiler protocol and
structural edit clients, not persistent source-control identifiers.

Dependencies, conflicts, and open questions: Depends on the workspace revision
model. Handle lifetime, revision garbage collection, and concurrent client
policy remain open.

### PROTO-004 — Atomic validated structural change

Status: **Accepted 2026-07-18**

Source use cases: UC-003.

Requirement: The compiler interface must be capable of expressing the priority
evolution as a structural change against R1 and validating the complete
resulting workspace before commit. If validation fails or R1 is stale, the
operation must not publish a partial revision. Success returns R2, its canonical
text edits, identity map, semantic diff, diagnostics, and validation summary.

Rationale and agent change cost: Atomic validation replaces coordination of
independent text edits with one checkable change unit, reducing partial-state
diagnostics, repair cascades, and regression risk.

Acceptance evidence: One example commits a valid UC-003 change. Seeded examples
with a missed constructor, incompatible serializer, failing fixture, and stale
base revision each leave R1 unchanged and return categorized causes.

Target milestone and scope: Complete transaction contract shape in M11;
implementation in numbered core-protocol milestones added after M13. Constrains
protocol transactions and canonical text generation, not the internal
refactoring algorithm or user approval policy.

Dependencies, conflicts, and open questions: Depends on PROTO-002, PROTO-003,
PROTO-005, formatter rules, and workspace validation semantics. The smallest
initial set of structural edit primitives remains open.

### PROTO-005 — Structured diagnostics and completion evidence

Status: **Accepted 2026-07-18**

Source use cases: UC-001 and UC-003.

Requirement: Static, validation, impact, and structural-edit failures must be
reported with a stable code, revision, semantic location, category, expected and
actual facts where applicable, related identities, minimal causal chain, and
cascade relationship. The interface must also return a machine-readable
completion summary for successful validation.

Rationale and agent change cost: Stable diagnostics localize failures and make
repair policy automatable. Structured success evidence prevents the absence of
diagnostics from being mistaken for proof of complete validation.

Acceptance evidence: Candidate protocol examples include invalid request
construction, missing result handling, undeclared capability use, stale handle,
missed schema consumer, and failed compatibility fixture. Snapshot tests verify
stable structured fields and causality independently of prose wording.

Target milestone and scope: M11 diagnostic contract followed by numbered
core-protocol implementation milestones added after M13. Constrains
compiler/protocol diagnostics and validation summaries; human prose remains a
derived rendering.

Dependencies, conflicts, and open questions: Depends on the initial diagnostic
taxonomy and revision model. Security redaction and long-term compatibility of
diagnostic codes remain open.

## Non-functional and benchmark requirements

The first run is a calibration pilot. It measures the four baseline languages
before setting AIL targets. Correctness rules and basic resource limits are
fixed now. The targets used to judge AIL will be recorded after the baseline
results are available and before any AIL benchmark run starts.

### NFR-001 — Reproducible comparative benchmark

Status: **Accepted 2026-07-18**

Source use cases: UC-001 and UC-003.

Requirement: Before a run starts, its locked manifest must record the starting
source and dependency revisions, task text, tests, model and agent versions,
initial context, available tools, tool versions, container image, reference
host, time limit, retry policy, and definition of success. Rust, Go, Python,
TypeScript, and AIL receive the same behavior and their normal development
tools.

Rationale and agent change cost: The project claim is not falsifiable if
differences in context, tooling, retries, or correctness oracles explain the
result. Reproducibility makes total change cost reviewable.

Acceptance evidence: Each run emits a manifest containing all frozen values,
raw event/timing records, final revision, and correctness result. A second
operator can replay the functional oracle from the manifest and obtain the same
pass/fail classification.

Target milestone and scope: M2 run-manifest contract and M7 locked benchmark
configuration, before M8 agent runs. Constrains benchmark and governance, not
the language or runtime.

Dependencies, conflicts, and open questions: Uses the shared JSON fixture
format. Exact tool versions, container digest, host details, and model sampling
settings must be filled in before the first run. Model output may not be exactly
repeatable, so the manifest records the inputs and settings needed to reproduce
the conditions rather than promising identical generated text.

### NFR-002 — Measure all context used

Status: **Accepted 2026-07-18**

Source use cases: UC-001 and UC-003.

Requirement: For each task, measure every model input token after the task
starts. Break the total into initial context, source reads, compiler or language
server results, diagnostics, build and test output, and other tool output. Run
at least 10 independent successful trials for each task and language. A pilot
run has a 100,000-input-token safety limit, but that limit is not the eventual
AIL success target.

Rationale and agent change cost: This tests context efficiency across the whole
correct-change loop instead of rewarding short source that requires expensive
discovery or repair.

Acceptance evidence: Token-accounting logs use the actual model tokenizer,
deduplicate no repeated context unless the model did not receive it, and publish
median, range, and category totals for successful and failed runs separately.
Task correctness is a prerequisite; an incomplete low-token run does not count.

Target milestone and scope: M8 baseline calibration and M9 AIL target decision,
followed by the later empirical suite. Constrains measurement and agent protocol
output budgeting, not source token count.

Dependencies, conflicts, and open questions: Depends on NFR-001 and a stable
agent execution harness. After the four baseline results are recorded, the
maintainers set the AIL context target before any AIL benchmark run begins.

### NFR-003 — Measure repairs and forbid missed changes

Status: **Accepted 2026-07-18**

Source use cases: UC-001 and UC-003.

Requirement: Count a repair whenever an agent edits the code and the next
compiler, static check, or behavior test shows that the task is still
incomplete. Record the count and failure reason for every run. No successful
UC-003 run may miss a seeded location that needed a change or pass while a
hidden UC-001 regression remains.

Rationale and agent change cost: Repair work and regressions are dominant terms
in the project thesis. Counting only first-pass source generation would hide the
cost AIL is intended to reduce.

Acceptance evidence: The harness records each edit/validation cycle and failure
category. Hidden tests seed affected consumers and UC-001 invariants. Reports
separate incomplete runs, detected repairs, escaped regressions, and final
success.

Target milestone and scope: M2 repair-cycle contract, M8 baseline measurement,
and M9 AIL target decision. Constrains benchmark and protocol diagnostic
quality; it does not mandate a particular repair strategy.

Dependencies, conflicts, and open questions: Depends on NFR-001, PROTO-002, and
PROTO-005. The run manifest defines how pre-edit agent checks are counted. After
the baseline results are recorded, the maintainers set the AIL repair target
before any AIL benchmark run begins.

### NFR-004 — Measure handler latency and throughput

Status: **Accepted 2026-07-18**

Source use cases: UC-001 and UC-003.

Requirement: On the locked reference environment, one process using the
in-memory test store must run the shared warm-state cases. Measure throughput
and p50, p95, and p99 handler latency. Exclude process startup and fixture file
I/O; include validation, version conversion, job construction, store mutation,
and result construction. Results and store calls must be correct before the
performance result counts. A pilot corpus run must finish within 30 seconds as a
basic safety limit.

Rationale and agent change cost: The slice must not obtain semantic or context
advantages by abandoning credible service execution. This requirement states an
observable envelope without choosing ownership, garbage collection, regions,
reference counting, or a backend.

Acceptance evidence: At least 30 measured runs follow a documented warm-up,
corpus, clock, affinity, load, and summary procedure. Report median throughput,
p50/p95/p99 latency, variance, host/container identity, and profiler evidence
for outliers for AIL and every baseline.

Target milestone and scope: M8 baseline calibration and M9 AIL target decision,
followed by a production-runtime milestone added after M13. Constrains benchmark
procedure, not the initial semantic oracle interpreter's production
performance.

Dependencies, conflicts, and open questions: Depends on NFR-001 and the shared
corpus. After baseline measurement, the maintainers set the production AIL
target before measuring an AIL production runtime. Network database latency and
concurrent load are deferred.

### NFR-005 — Measure startup and memory

Status: **Accepted 2026-07-18**

Source use cases: UC-001 and UC-003.

Requirement: On the locked reference environment, measure cold startup, memory
when ready, and peak memory while running the cases. The calibration runner must
start within 2 seconds and stay below 512 MiB peak resident memory. These are
safety limits, not production AIL targets. The runner uses one non-privileged
process with no network access, external service, or dependency installation
after packaging.

Rationale and agent change cost: Startup, memory, and deployment predictability
are application constraints that must remain visible while implementation
mechanisms stay open. A simple package also limits environmental diagnosis and
repair work.

Acceptance evidence: At least 30 cold starts and corpus runs record readiness
time, idle and peak RSS, exit status, package manifest, dependency lock, and
attempted external access. Functional results and traces must match before
resource measurements count.

Target milestone and scope: M8 baseline calibration and M9 AIL target decision,
followed by a production-runtime milestone added after M13. Constrains packaging
and measurement, not static versus dynamic linking, memory management, artifact
format, or baseline idioms.

Dependencies, conflicts, and open questions: Depends on NFR-001 and a defined
readiness signal. After baseline measurement, the maintainers set the production
AIL startup and memory targets before measuring an AIL production runtime.
Package size and bit-reproducibility limits are deferred until the compilation
and deployment target is selected.

## Acceptance and sequencing

These requirements were accepted on 2026-07-18. They authorize the next
artifacts, in order:

1. a frozen language-independent fixture and trace corpus;
2. a baseline benchmark protocol and equivalent Rust, Go, Python, and
   TypeScript reference implementations;
3. baseline calibration runs and frozen AIL success targets;
4. two or three explicitly labeled **Illustrative AIL** renderings evaluated
   against these requirements; and
5. selection of a narrow semantic slice, proposed language rules, and proposed
   fixtures.

These requirements do not authorize a production source tree or settle the
compiler implementation stack. The common compiler spikes remain gated by
[the stack-evaluation prerequisites](../stack-evaluation.md).
