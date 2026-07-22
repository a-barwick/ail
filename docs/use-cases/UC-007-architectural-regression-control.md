# UC-007 — Architectural regression control

Status: **Proposed**

Documentation layer: concrete application scenario. This record is
language-independent and non-normative for AIL syntax or semantics.

[ADR 0006](../decisions/0006-prepare-architectural-regression-control.md)
selects an M23 acceptance package for this use case. Its status remains
Proposed until the concrete behavior, workspace, policy, metrics, examples,
baseline comparison, and budgets pass that gate.

## Actor and desired outcome

A maintainer asks an agent to add one operation to a service that already has
enough handlers, capabilities, and shared policies for a central dispatcher or
orchestration function to become an attractive shortcut.

The agent must implement the operation without increasing an existing
architectural hotspot, bypassing a declared dependency boundary, or granting
unrelated authority to an existing unit. Compilation must produce
machine-readable evidence that distinguishes architectural facts, regressions,
and enforced policy violations.

The desired outcome is not a universally preferred module layout. It is a
revision-safe change that preserves the architecture contract selected by the
project and makes any intentional exception explicit.

## Reference system and system boundary

The proposed reference workspace extends the job-service domain after the
UC-001 and UC-003 slice is stable. It contains:

- at least 24 logical operations;
- transport-independent operation declarations;
- request decoding and response encoding boundaries;
- separate domain handlers;
- datastore, clock, network, and telemetry capability contracts;
- shared authentication and request-policy components;
- tests and fixtures for every operation;
- one existing decision-heavy dispatch or orchestration hotspot recorded in an
  architectural baseline; and
- a versioned project policy describing allowed dependency directions,
  authority boundaries, thresholds, and no-growth rules.

Inside the change boundary:

- the requested operation and its public contract;
- its handler, tests, fixtures, and required capability contract;
- operation registration or discovery;
- every semantic dependency changed by the task;
- compiler-produced architectural snapshots for the starting and final
  revisions;
- the architectural delta and project-policy evaluation; and
- any explicit, scoped architectural exception.

Outside the boundary:

- a production HTTP server or deployment platform;
- dynamically loaded operations absent from the current build;
- external consumers the compiler cannot inspect;
- a universal rule for how every AIL service must be layered; and
- the internal representation of the compiler's semantic graph.

## Inputs and authority

The task receives:

1. a fixed starting revision `R1`;
2. the requested operation and behavior fixtures;
3. the complete current build and locked dependency source available to the
   compiler;
4. the project architecture policy and accepted hotspot baseline;
5. permission to inspect, structurally edit, format, build, and test the
   workspace; and
6. no permission to weaken the policy, rewrite the baseline, or add an
   exception unless the task explicitly authorizes that governance change.

The new operation receives only the capability instances required by its
contract. Transport adaptation must not gain datastore, clock, network, or
telemetry authority merely because the new operation uses one of those
capabilities in its domain handler.

## Representative change

### Behavior illustration: add a cancel-job operation

The proposed task is:

> Add a `CancelJob` operation with its closed success and domain-failure
> outcomes. It may conditionally update the jobs namespace and emit no other
> external effect. Preserve the existing dependency and authority boundaries,
> do not enlarge the recorded dispatch hotspot, and return architectural,
> semantic, textual, and test evidence for the final revision.

The exact request, result, and store behavior will be frozen before this use
case is accepted. The architecture task is deliberately constructed so that a
behaviorally correct implementation can still be architecturally invalid.

One seeded invalid implementation adds decoding, authorization, datastore
mutation, result mapping, and telemetry decisions directly to the existing
dispatcher. Its behavior fixtures pass, but it:

- increases the dispatcher's control-flow complexity;
- adds direct datastore authority to a transport-facing unit;
- increases the number of state domains coordinated by the dispatcher; and
- enlarges the semantic context required to review that unit.

A valid implementation adds the operation through the workspace's declared
extension boundary, keeps domain behavior behind its explicit contract, and
does not enlarge the accepted hotspot.

## Architectural policy

The frozen project policy must cover at least:

- permitted dependency directions between transport, domain, and capability
  adapter groups;
- capabilities permitted for each group;
- control-flow and semantic-context thresholds for new units;
- no-growth rules for recorded hotspots;
- prohibition of new dependency cycles;
- aggregate checks at symbol, module, dependency-component, and declared
  architecture-group scope; and
- the allowed disposition of observations, regressions, and violations.

The policy must not use source comments as hidden suppressions. Any exception is
a versioned project artifact with a stable rule code, semantic scope, rationale,
and review boundary.

The compiler reports measurements even when no threshold is configured. The
project policy, not universal language semantics, decides whether a measurement
is informational, advisory, or build-blocking.

## Observable compiler behavior

For `R1`, compilation or an equivalent semantic query returns an architectural
snapshot containing:

- analyzed scope and coverage;
- metric values and contributing semantic identities;
- existing hotspot classifications;
- active policy rules and exceptions; and
- deterministic ordering and schema version information.

For a proposed or completed revision `R2`, the compiler additionally returns:

- metric changes from `R1`;
- new, enlarged, reduced, and removed hotspots;
- boundary and authority changes;
- new or enlarged dependency cycles;
- policy results with stable diagnostic codes;
- analysis gaps and unchecked external boundaries; and
- related structural operations that the compiler can validate.

The final build may succeed only if all build-blocking policy results are
satisfied or covered by an authorized exception. A partial or failed analysis
must not be reported as a clean architectural result.

## Why aggregate analysis is required

Function size alone is insufficient. An agent can replace one large function
with many private helpers while leaving the same module responsible for all
authority, state, and dispatch.

The reference policy therefore evaluates both individual executable units and
larger semantic scopes. A refactor is not considered an improvement merely
because source lines or per-function decision counts decrease. The final
manifest must also show the resulting capability, state, dependency, cycle, and
context concentration.

## Representative agent workflow

The measured workflow is:

1. compile or query `R1`;
2. inspect the requested operation contract, architectural snapshot, relevant
   policy, and bounded semantic context;
3. request the architectural and semantic impact of the proposed operation;
4. make ordinary or structural edits;
5. compile or validate `R2`;
6. inspect the architectural delta and policy results;
7. repair any behavior or architecture failure; and
8. return completion evidence bound to the accepted final revision.

Agents may use normal search, compiler, language-server, formatter, refactoring,
test, and static-analysis tools. Baseline languages receive their normal tools
and the same project policy expressed in the strongest practical form available
to them.

## Completion evidence

The task is complete only when the final revision includes:

- passing public and hidden behavior tests;
- the expected semantic impact and effect trace;
- architectural snapshots for `R1` and the final revision;
- the revision-bound architectural delta;
- no unhandled build-blocking policy result;
- no unauthorized policy or baseline change;
- a list of new, enlarged, reduced, and removed hotspots;
- a list of boundary, capability, state, and dependency changes;
- any unchecked source or external system;
- the canonical source diff and semantic diff; and
- any authorized exception with its exact scope and rationale.

## Measurable success criteria

UC-007 is ready for acceptance when the fixtures and benchmark can prove:

1. the compiler reports the same architectural snapshot for repeated analysis
   of the same revision and configuration;
2. the seeded centralized implementation passes behavior tests but fails with
   the expected architectural policy codes;
3. the valid implementation passes the same behavior tests and architecture
   policy;
4. splitting the seeded dispatcher into helpers inside the same responsibility
   boundary does not erase aggregate concentration findings;
5. a new prohibited dependency, capability, or cycle is attributed to the
   semantic edges that caused it;
6. an unchanged pre-existing hotspot does not fail a no-growth policy;
7. enlarging that hotspot does fail without rewriting the baseline;
8. incomplete analysis is reported as incomplete rather than clean;
9. every diagnostic and manifest is bound to the correct source and policy
   revisions; and
10. an independent reviewer can explain the architectural consequence of the
    change without reconstructing it from raw source.

Benchmark targets for context, repairs, false findings, analysis time, and
manifest size must be calibrated against strong baselines before acceptance.

## Derived proposed requirements

- APP-006
- LANG-006
- PROTO-006
- PROTO-007
- NFR-006
- NFR-007

See [the proposed architectural-health requirements](../requirements/architectural-health.md).

## Explicit exclusions and unresolved questions

This use case does not require:

- one universal complexity threshold;
- a single composite maintainability score;
- automatic inference of the project's desired architecture;
- mandatory rejection of every large function;
- a particular service framework or registration mechanism;
- automatic refactoring without validation and review; or
- architectural guarantees for source the compiler cannot inspect.

Before acceptance, decide:

1. the exact starting workspace and cancel-job behavior;
2. which architectural groups and dependency directions are frozen;
3. which initial hotspot and baseline values are seeded;
4. which metrics are thresholded and which use no-growth policies;
5. the maximum acceptable false-positive count;
6. how baseline tools express equivalent constraints; and
7. the analysis-time and manifest-size envelopes.
