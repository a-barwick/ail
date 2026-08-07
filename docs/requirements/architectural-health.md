# Architectural-health requirements

These requirements come from
[UC-007](../use-cases/UC-007-architectural-regression-control.md). M25 and M26
implement the exact M24 subset. The numbered rules in
[`specs/architecture.md`](../../specs/architecture.md), not this summary, define
compiler behavior.

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

Status: **Accepted**

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

Implemented scope: M24 defines the rules; M25–M26 implement snapshots, deltas,
policy, and atomic publication. The requirement does not prescribe one module
layout.

Dependencies and open questions: Depends on LANG-006, PROTO-006, PROTO-007, and
the UC-007 architecture policy. The exact `CancelJob` behavior and thresholds
are in the [architecture case](../architecture-acceptance.md).

## Language requirement

### LANG-006 — Complete architectural relationships

Status: **Accepted**

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

Acceptance evidence: The broader proposed fixtures cover direct and indirect
calls, capability delegation, state access, closed outcomes, recursion, foreign
boundaries, and generated source. Every relationship is either present in the
semantic graph or represented by an explicit incomplete-coverage edge.

Target milestone and scope: M24 accepted the bounded scaling-language contract,
and M25–M26 expose it through the compiler protocol. It is not part of M11.
Constrains the language and compiler semantic model, not metric thresholds or
project architecture.

Dependencies and open questions: The bounded M24 contract uses explicit module,
capability, state, and typed-edge facts. Applying the requirement beyond that
slice still depends on future general effect, foreign-code, and generated-source
rules. Opaque reflection may remain excluded instead of being modeled.

## Compiler semantic-interface requirements

### PROTO-006 — Architectural health snapshot

Status: **Accepted**

Source use case: UC-007.

Requirement: For a requested source revision, build configuration, policy
revision, and analysis scope, the compiler interface must return a versioned,
deterministically ordered architectural health snapshot. It must include
coverage, metric values, semantic contributors, aggregate scopes, active policy
selectors, baseline matches, exceptions, and unchecked boundaries.

The accepted metric scope is the seven-metric M23 acceptance slice in the
[M23 package](../architecture-acceptance.md), not the entire proposed catalog in
the architectural health manifest. The interface must report primitive
measurements and semantic sets rather than only an opaque composite score.
M24 defines the normative meanings and canonical encoding for that bounded
slice; M25 implements the snapshot and compact rendering.

Rationale and agent change cost: A structured snapshot lets an agent and
reviewer inspect where responsibility, authority, state, and coupling are
concentrated without reconstructing them from source or interpreting prose
warnings.

Acceptance evidence: Protocol fixtures cover a function, module, dependency
component, and declared architecture group; repeated requests are identical;
semantic contributors resolve at the requested revision; and partial coverage
cannot be rendered as a clean result.

Target milestone and scope: M24 accepted the scaling-protocol contract after the
core revision protocol, and M25 implemented this snapshot. It is not part of
M0–M17. Constrains the compiler protocol and policy engine, not its transport
encoding.

Dependencies and open questions: Depends on LANG-006 and revision-scoped
semantic handles. Metric cost and incremental recomputation strategies remain
implementation questions subject to NFR-006.

### PROTO-007 — Architectural delta and policy evaluation

Status: **Accepted**

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

Target milestone and scope: M24 accepted the bounded operation after PROTO-006,
and M26 implemented its delta, policy evaluation, and atomic publication.
Constrains compiler, protocol, project policy, and validation summaries.

Dependencies and open questions: Depends on PROTO-003, PROTO-005, PROTO-006,
identity mapping, and the project-policy revision model. M24 selected the first
accepted bounded policy vocabulary; vocabulary outside that contract remains
proposed.

## Non-functional and benchmark requirements

### NFR-006 — Deterministic and bounded architectural analysis

Status: **Accepted**

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

Target milestone and scope: M24 fixed deterministic graph and output budgets;
M25 and M26 implemented bounded incomplete results. Numerical performance
calibration remains later work. Constrains analysis behavior and evidence, not
a particular graph algorithm.

Dependencies and open questions: Depends on PROTO-006 and PROTO-007. M23 fixes
deterministic graph and encoded-output limits. Numerical wall-time and memory
limits still require later baseline measurement; the acceptance package does
not claim that calibration has occurred.

### NFR-007 — Architectural regression benchmark

Status: **Accepted**

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

Implementation state: the semantic graph and revision protocol exist. The
cross-language benchmark has not been run.

Dependencies and open questions: Depends on NFR-001, PROTO-006, PROTO-007, and
the frozen UC-007 workspace. M23 fixes a zero false/missed-finding allowance.
Trial count, repair-cycle, wall-time, peak-memory, and comparative model-context
envelopes require later baseline calibration.

## Implemented result

The fixtures fix the starting workspace, `CancelJob` behavior, architecture
policy, seven metrics, four scopes, expected classifications, and output budgets.
M25–M26 implement those fixtures. M27 records one repair using the compiler
output. NFR-007 still requires a cross-language benchmark.

Project policy chooses complexity thresholds. The AIL language does not define
one threshold for every program.
