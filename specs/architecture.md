# M24 architectural regression contract

Status: **Accepted 2026-07-22**

This contract accepts only the architecture facts needed by the M23 package.
It adds no AIL source grammar. In particular, the broader proposed catalog in
`docs/architecture-health.md` and its `ARCH` names are not inherited. The only
accepted metrics are control-flow complexity, direct dependency set, direct
declared capability set, state read set, state write set, dependency-component
size, and minimal-context node count.

## Semantic graph and scopes

A typed edge is a source identity, target identity, and one of `calls`,
`type-use`, `verifies`, `capability-use`, `state-read`, `state-write`, or
`delegates`. Calls and all kinds except `verifies` are direct dependency edges.
Capability and state edges name the declaration or access site directly; no
transitive authority is inferred. Coverage lists every analyzed architecture
group and every unavailable boundary with a reason.

`ArchitectureRequest.analysis_scope` names one executable-unit identity. A
snapshot reports, in order, that unit, its module, its dependency component,
and its architecture group, plus a different unit or group required by a
finding. It does not report every workspace scope. R1 and centralized select
dispatch, valid selects the domain handler, and helper-split selects
cancel_decision. Additional scopes sort by scope-kind precedence
executable-unit, module, dependency-component, architecture-group, then ID.

Facts are reported for an executable unit, module, dependency component, or
architecture group. Module membership is explicit fixture data, never inferred
from identity prefixes: the five role templates map to contracts, transport,
transport, domain, and tests; shared_policy is domain; dispatch and all three
helper-split helpers are transport. Non-unit endpoints map to adapters. Module
and group facts are unions of their member units;
their CFC is the member maximum and sum with ordered contributors. A dependency
component is one deterministic Tarjan strongly-connected component over unit
dependency edges. Every unit belongs to exactly one module and group. Sets and
contributors are deduplicated by semantic identity, so aggregation never
counts a unit or edge twice.

The seven metrics have these exact meanings at every scope: unit CFC is
`E - N + 2`; aggregate CFC reports maximum, sum, and ordered unit/value
contributors. Direct dependencies are unique targets of M23
`DEPENDENCY_KINDS`, excluding same-aggregate targets. Declared capabilities are
direct namespace IDs such as `jobs_store`; operation/access sites remain
contributors and are not capability values. Values are never transitive;
state read and write sets are unique direct targets of their respective edges;
unit component size is its containing SCC size, component size is member count,
and module/group component size is the maximum containing SCC size. Minimal
context is the deduplicated one-hop closure after selecting all aggregate member
units, every directly incident edge endpoint,
direct capability and state site, applicable policy selector, and matching
baseline. It never follows a second hop. A component ID is
`component:sha256:<hex>` over sorted member IDs joined by NUL.

### M24-LANG-001 — Explicit architecture semantics

The typed graph, complete coverage, direct authority facts, seven metrics, and
four aggregate scopes above are preserved from canonical source through the
semantic model. No source grammar is added. Traceability: APP-006, LANG-006,
PROTO-006, NFR-006.

### M24-LANG-002 — Aggregate responsibility cannot be hidden

Module, component, and group aggregation uses semantic membership and unions,
not source layout or helper count; splitting a unit cannot erase authority,
state, dependencies, or contributors. Traceability: APP-006, LANG-006,
PROTO-007, NFR-007.

## Identity, policy, and comparison

An analysis identity is `(workspace_id, revision_id, semantic_model_version,
policy_revision, baseline_revision, analysis_scope)`. Snapshots compare only when workspace and
semantic-model versions match and the delta names the base snapshot digest.
Semantic identities sort by UTF-8 bytes. Findings and diagnostics sort by code
precedence `HOTSPOT_GROWTH`, `AUTHORITY`, `STATE`, `BOUNDARY`, `NEW_UNIT`,
`CYCLE`, `COVERAGE_INCOMPLETE`, `STALE_BASELINE`,
`GOVERNANCE_UNAUTHORIZED`, `ANALYSIS_INCOMPLETE`, then scope, contributor
array, and finding ID. Policies retain M23 order. Scope changes use scope-kind
precedence then identity. Coverage, debt, exceptions, diagnostics, completion
arrays, scope members, and contributors retain declared or identity order.

An observation reports a fact without failure. A regression is growth relative
to the named compatible baseline. A violation breaches current policy. An
incomplete result means a required fact was unavailable or a budget exhausted
and can never pass. Dispositions are `record`, `warn`, and `deny`; only `deny`
blocks commit. M23's dispatch baseline is accepted debt, not a violation.
All eight policies are explicit records containing selector, classification,
disposition, comparison, and exact value. Exceptions match exactly policy
revision, rule, scope, contributor set, and a non-expired review boundary;
there is no wildcard matching. A missing/incompatible baseline
is stale. Policy, baseline, and exception changes require separately authorized
governance evidence and cannot be supplied by the candidate under analysis.
Trusted authorization is not a caller boolean: it binds candidate revision and
graph digest, policy and baseline revisions, exception IDs, rule, scope, and
review boundary, including the governance change `kind`. Exactly one
`GovernanceChange` is permitted per authorization and request. An ordinary
candidate requests zero governance changes; more than one is unauthorized.
`validate_architecture_change` receives an
`ArchitectureEvaluationInput`: the candidate-owned `request` carries complete
`GovernanceChange` records and one `authorization_id`, while trusted
`governance_authorizations`, active exceptions, and active policy and baseline
revisions are separate fields. The evaluator resolves that exact ID and rejects
an unknown ID or a mismatch in any binding. Contributor and exception-ID arrays
are matched as sets and emitted deduplicated in UTF-8 sort order.

An authorized governance change does not override architecture findings. It may
commit only when the resulting candidate has no other denied finding. By
contrast, a pre-existing trusted exception that exactly matches the active
policy, finding rule, scope, contributor set, and review boundary needs no new
change authorization: it is already governance context, not a candidate-owned
governance mutation.

### M24-PROTO-001 — Revision-bound snapshot and delta

Snapshot and delta shapes expose analysis identity, all four scopes, coverage,
budgets, ordered findings with contributors, dispositions, and exact base and
candidate digests. Traceability: PROTO-006, PROTO-007, NFR-006.

`architecture_snapshot` is read-only. Its `ArchitectureSnapshotInput` names one
revision and analysis scope plus trusted active governance context. Success is
exactly `ArchitectureSnapshotResponse`: `status`, `snapshot`, and
`snapshot_digest`. It has no delta, publication, commit, or completion evidence.
`validate_architecture_change` is the transactional operation. It consumes
`ArchitectureEvaluationInput` and returns `ArchitectureSuccess`,
`ArchitectureFailure`, or `ArchitectureIncompleteFailure`.

### M24-PROTO-002 — Deterministic canonical encoding

Canonical structured output is UTF-8 JSON, two-space indented, with insertion
key order fixed by the fixtures, ordered arrays, no duplicate object keys, and
one final LF. Compact output is derived only from that structured output and is
also exactly fixture-locked and LF-terminated. Traceability: PROTO-006, NFR-006,
NFR-007.

The canonical candidate graph digest is lowercase `sha256:<hex>` over a
canonical two-space-indented JSON object plus final LF with keys `units` then
`edges`. Each unit is projected with keys in exact order `id`, `module`,
`group`, `cfg` (`nodes`, `edges`), `capabilities`, `state_reads`, and
`state_writes`; module is explicit semantic membership, not an ID-prefix
inference. Units sort by UTF-8 `id`, and every set-valued unit array is sorted
and deduplicated. Each edge is projected as `[source, target, kind]`; duplicate
unit identities and duplicate edge triples are rejected before digesting. Edges
sort by UTF-8 source, then UTF-8 target, then typed
edge kind precedence `calls`, `type-use`, `verifies`, `capability-use`,
`state-read`, `state-write`, `delegates`. Reordering or duplicating equivalent
set-valued unit facts therefore cannot change authorization binding.

### M24-PROTO-003 — Governed policy evaluation

The eight M23 policies, accepted dispatch baseline, exact exceptions, three
dispositions, stale-baseline behavior, and immutable governance described above
are the complete accepted policy vocabulary. There are no universal default
thresholds. Traceability: APP-006, PROTO-007, NFR-007.

### M24-PROTO-004 — Complete bounded analysis

The fixed limits are 512 semantic nodes, 2,048 typed edges, 65,536 structured
UTF-8 bytes, 2,048 compact bytes, 12 compact lines, and no required-contributor
truncation. Semantic nodes count unique units plus non-unit endpoints in the
complete candidate; typed edges count every complete-candidate edge.
`structured_bytes` is the canonical snapshot byte length with that field zero;
compact counts include the final LF. `exhausted` is null or the first exhausted
budget-field name. The complete operation envelope also fits 65,536 bytes.
Coverage loss or any exhausted limit returns the same bounded
`ArchitectureIncompleteFailure`, never a full snapshot, delta, failure, or
pass. Exhaustion is selected in fixed order `semantic_nodes`, `typed_edges`,
`structured_bytes`, `compact_bytes`, `compact_lines`. The failure preserves
the actual known graph counts and exact computed output counts; prospective
full output may be constructed only to measure its exact size before it is
discarded. Coverage loss emits `AIL.ARCH.COVERAGE_INCOMPLETE`; a budget
exhaustion emits `AIL.ARCH.ANALYSIS_INCOMPLETE`. Limits are fixed before analysis and are never expanded or retried to turn
an incomplete candidate into a pass. `ArchitectureIncompleteFailure` is the
bounded output: it contains identity, coverage, budget use, ordered diagnostics,
no edits, the unchanged current revision, and no published child. It does not
contain a partial snapshot or delta. Traceability: PROTO-006, NFR-006, NFR-007.

### M24-PROTO-005 — Stable actionable diagnostics

This slice accepts only `AIL.ARCH.HOTSPOT_GROWTH`, `AIL.ARCH.AUTHORITY`,
`AIL.ARCH.STATE`, `AIL.ARCH.BOUNDARY`, `AIL.ARCH.NEW_UNIT`,
`AIL.ARCH.CYCLE`, `AIL.ARCH.COVERAGE_INCOMPLETE`,
`AIL.ARCH.STALE_BASELINE`, `AIL.ARCH.GOVERNANCE_UNAUTHORIZED`, and
`AIL.ARCH.ANALYSIS_INCOMPLETE`. Each identifies rule, scope, classification,
disposition, and all ordered contributors. Traceability: APP-006, PROTO-006,
PROTO-007, NFR-007.

### M24-PROTO-006 — Atomic validation and completion evidence

Architecture evaluation is part of the M19/M21 whole-candidate transaction.
Only a complete result without a denied finding publishes one child revision.
Denial, stale governance, or incomplete analysis publishes no revision and no
edits. Success evidence binds base, child, snapshot, delta, policy, baseline,
coverage, budgets, behavior validation, and commit outcome to that child
revision; evidence from another revision cannot complete the change.
Traceability: APP-006, PROTO-006, PROTO-007, NFR-006.

The transaction evaluates the complete candidate graph and behavior evidence
against the separately trusted policy, baseline, exceptions, and authorization
records. Candidate semantic changes may alter graph, control-flow, and coverage
facts, but cannot supply or mutate trusted governance records.

Snapshot and delta digests are lowercase `sha256:<hex>` over the canonical JSON
bytes of the object, which contains no self-digest. Ordered delta scope changes
bind the full base and candidate records by those records' canonical digests.
`ArchitectureFailure` contains the unchanged current revision, full snapshot
and delta, ordered diagnostics, `edits: []`, and no child. Completion evidence
binds base and child, both snapshot digests, delta digest, policy and baseline,
coverage, budgets, behavior evidence, and commit.

## Boundaries

This contract specifies no Rust/compiler/runtime implementation, repair,
official agent evidence, universal policy, additional metric, or M25 behavior.
