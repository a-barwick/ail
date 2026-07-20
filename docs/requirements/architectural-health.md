# Proposed architectural-health requirements

Status: **Proposed**

Documentation layer: requirements derived from
[UC-007](../use-cases/UC-007-architectural-regression-control.md). These
requirements are not accepted language or protocol rules and do not select AIL
syntax or the compiler implementation.

## Traceability summary

| Requirement | Primary constraint |
| --- | --- |
| APP-006 | feature work preserves the declared architecture contract |
| LANG-006 | ordinary source preserves complete architectural relationships |
| PROTO-006 | revision-bound architectural health snapshot |
| PROTO-007 | architectural delta and project-policy evaluation |
| NFR-006 | deterministic, bounded, honest analysis |
| NFR-007 | regression-control benchmark |

## Application requirement

### APP-006 — Architecture-preserving feature extension

Status: **Proposed**

Source use case: UC-007.

Requirement: A behaviorally correct feature addition must also preserve the
project's declared dependency directions, capability boundaries, state
ownership, cycle policy, and hotspot no-growth rules. A change that intentionally
violates one of those constraints must carry an authorized, scoped exception;
it must not silently rewrite the policy or accepted baseline.

Rationale and agent change cost: Behavior tests alone permit an agent to place
new work in a central dispatcher or orchestration path. Making architectural
regression part of completion evidence reduces future context discovery,
consequence analysis, repair work, and concentration risk.

Acceptance evidence: The UC-007 centralized implementation passes behavior
tests and fails architecture policy. The valid implementation passes both. The
completion manifest shows that no unauthorized policy, baseline, capability,
state, or dependency change occurred.

Target milestone and scope: A future numbered scaling milestone after the core
semantic graph and revision protocol exist, and only after UC-007 and this
requirement set are accepted. It is not part of M0–M17. Constrains application
completion, compiler policy, benchmark, and governance. It does not prescribe
one module layout.

Dependencies and open questions: Depends on LANG-006, PROTO-006, PROTO-007, and
the frozen UC-007 architecture policy. The exact cancel-job behavior and policy
thresholds remain open.

## Language requirement

### LANG-006 — Complete architectural relationships

Status: **Proposed**

Source use case: UC-007.

Requirement: Ordinary AIL constructs must preserve statically inspectable
relationships for direct calls, dependencies, capability requirements, effects,
state-domain reads and writes, public outcomes, recursion, and module
membership. Features that make any relationship opaque must make the loss of
coverage explicit rather than permitting the compiler to claim a complete
architectural result.

Rationale and why this belongs in the language: A compiler cannot reliably
measure responsibility or enforce architectural boundaries if normal language
features hide the underlying authority, state, or dependency edges. Language
analyzability creates the facts that the compiler protocol and project policy
consume.

Acceptance evidence: Proposed fixtures cover direct and indirect calls,
capability delegation, state access, closed outcomes, recursion, foreign
boundaries, and generated source. Every relationship is either present in the
semantic graph or represented by an explicit incomplete-coverage edge.

Target milestone and scope: A future numbered scaling-language milestone after
acceptance, followed by core-protocol exposure. It is not part of M11 while this
requirement remains proposed. Constrains the language and compiler semantic
model, not metric thresholds or project architecture.

Dependencies and open questions: Depends on the future module, effect,
capability, state, foreign-code, and generated-source rules. The first core may
exclude opaque reflection instead of modeling it.

## Compiler semantic-interface requirements

### PROTO-006 — Architectural health snapshot

Status: **Proposed**

Source use case: UC-007.

Requirement: For a requested source revision, build configuration, policy
revision, and analysis scope, the compiler interface must return a versioned,
deterministically ordered architectural health snapshot. It must include
coverage, metric values, semantic contributors, aggregate scopes, active policy
selectors, baseline matches, exceptions, and unchecked boundaries.

The required initial metrics are defined by the
[architectural health manifest specification](../architecture-health.md). The
interface must report primitive measurements and semantic sets rather than only
an opaque composite score.

Rationale and agent change cost: A structured snapshot lets an agent and
reviewer inspect where responsibility, authority, state, and coupling are
concentrated without reconstructing them from source or interpreting prose
warnings.

Acceptance evidence: Protocol fixtures cover a function, module, dependency
component, and declared architecture group; repeated requests are identical;
semantic contributors resolve at the requested revision; and partial coverage
cannot be rendered as a clean result.

Target milestone and scope: A future numbered scaling-protocol contract and
implementation milestone after acceptance and after the core revision protocol
exists. It is not part of M0–M17. Constrains the compiler protocol and policy
engine, not its transport encoding.

Dependencies and open questions: Depends on LANG-006 and revision-scoped
semantic handles. Metric cost and incremental recomputation strategies remain
implementation questions subject to NFR-006.

### PROTO-007 — Architectural delta and policy evaluation

Status: **Proposed**

Source use case: UC-007.

Requirement: Given compatible architectural snapshots for `R1` and `R2`, the
compiler must return a revision-bound delta containing changed metric values and
sets; new, enlarged, reduced, and removed hotspots; changed dependency,
capability, effect, state, and cycle relationships; policy classifications; and
related semantic contributors.

Policy results must distinguish:

- an `observation`, which records a fact without asserting a problem;
- a `regression`, which violates a configured delta or baseline rule; and
- a `violation`, which breaks a configured absolute or boundary rule.

Each result has a stable code and project-selected disposition of `record`,
`warn`, or `deny`. A denied result prevents validation from reporting the
revision as accepted unless an authorized exception matches exactly.

Rationale and agent change cost: Revision deltas focus the agent on newly
introduced architectural risk instead of flooding it with unchanged debt.
Explicit project policy turns selected facts into enforceable constraints
without pretending the compiler can infer product architecture.

Acceptance evidence: Protocol fixtures cover a new hotspot, enlarged baseline
hotspot, reduced hotspot, prohibited capability edge, new dependency cycle,
unchanged accepted debt, stale baseline, authorized exception, expired or
inapplicable exception, and unauthorized policy edit.

Target milestone and scope: A future numbered scaling-protocol milestone after
acceptance and after PROTO-006 is available. Constrains compiler, protocol,
project policy, and validation summaries.

Dependencies and open questions: Depends on PROTO-003, PROTO-005, PROTO-006,
identity mapping, and the project-policy revision model. The first accepted
policy vocabulary remains to be selected from the proposed feature
specification.

## Non-functional and benchmark requirements

### NFR-006 — Deterministic and bounded architectural analysis

Status: **Proposed**

Source use case: UC-007.

Requirement: Architectural snapshots, deltas, policy results, and coverage
claims must be deterministic for identical source, dependency, compiler,
configuration, and policy revisions. The compiler must expose analysis time,
peak working memory attributable to the analysis where measurable, manifest
size, truncation, and unavailable-source coverage. A configured budget may
limit advisory detail but must not silently omit facts required to evaluate a
build-blocking rule.

Rationale and agent change cost: An unbounded or unstable health report merely
moves architectural work into output filtering and diagnosis. Honest budgets
keep the feature usable on large workspaces without weakening enforced policy.

Acceptance evidence: Repeated and incrementally recomputed reports compare
equal after normalization. Budget fixtures prove that required policy facts are
returned or the evaluation is explicitly `incomplete`; no incomplete result is
classified as a pass.

Target milestone and scope: The future numbered scaling-protocol implementation
and performance milestone created after acceptance. Constrains analysis behavior
and evidence, not a particular graph algorithm.

Dependencies and open questions: Depends on PROTO-006 and PROTO-007. Numerical
analysis-time, memory, and manifest-size limits require UC-007 baseline
calibration.

### NFR-007 — Architectural regression benchmark

Status: **Proposed**

Source use case: UC-007.

Requirement: A comparative benchmark must measure whether agents add the
UC-007 operation without enlarging the seeded hotspot or violating the frozen
architecture policy. It must separately report behavior correctness,
architecture correctness, context consumed, repair cycles, false findings,
missed findings, elapsed agent time, and any policy or baseline edits.

AIL targets must be frozen only after equivalent Rust, Go, Python, and
TypeScript workspaces are measured with their normal compiler,
language-server, refactoring, testing, and static-analysis tools.

Rationale and agent change cost: The project cannot claim to resist god
functions merely because the compiler emits metrics. The benchmark must show
that agents act on accurate evidence and avoid or repair architectural
regressions at lower total change cost.

Acceptance evidence: The benchmark contains a behaviorally correct centralized
implementation, a superficial helper-extraction variant, and a policy-compliant
implementation. Hidden checks verify the final semantic graph rather than
source layout alone.

Target milestone and scope: A future numbered scaling benchmark after the core
semantic graph and revision protocol exist, and only after UC-007 is accepted.
It is not part of M0–M17. Constrains benchmark and governance.

Dependencies and open questions: Depends on NFR-001, PROTO-006, PROTO-007, and
the frozen UC-007 workspace. Trial count, false-positive allowance, and
performance envelopes require baseline calibration.

## Acceptance gate

These requirements are ready for acceptance only when:

1. UC-007 fixes the starting workspace, operation behavior, and architecture
   policy;
2. the feature specification defines every required metric and policy result
   unambiguously;
3. proposed fixtures demonstrate both symbol-level and aggregate analysis;
4. baseline tools and comparison rules are identified; and
5. two independent readers classify every seeded change the same way.

Acceptance would authorize normative protocol rules and conformance fixtures.
It would not make a project-selected complexity threshold part of universal AIL
language semantics.
