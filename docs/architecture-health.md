# Architectural health manifest

Status: **Proposed feature specification**

Documentation layer: language and compiler design. This document proposes the
compiler contract for [UC-007](use-cases/UC-007-architectural-regression-control.md)
and its [derived requirements](requirements/architectural-health.md). It is not
yet a normative AIL language or protocol specification.

[ADR 0006](decisions/0006-prepare-architectural-regression-control.md) selects
an M23 acceptance package for this direction. This manifest remains Proposed
until the concrete workspace, behavior, policy, minimal metrics, expected
outputs, baseline comparison, and fixed budgets pass that gate.

## Outcome

The compiler produces a revision-bound, machine-readable account of where
control flow, dependencies, authority, effects, state access, and review context
are concentrated. Given two compatible revisions, it produces a delta and
evaluates the project's declared architecture policy.

The feature is intended to:

- expose architectural concentration before an agent edits a hotspot;
- distinguish unchanged debt from a regression introduced by the current
  change;
- enforce dependency and authority boundaries selected by the project;
- catch superficial function splitting through aggregate analysis;
- provide semantic contributors for diagnosis and validated refactoring; and
- give humans reviewable evidence independent of agent reasoning.

The compiler does not infer the ideal architecture of an application. It
provides complete semantic facts where the language permits completeness and
applies explicit project policy to those facts.

## Design principles

1. **Primitive facts before scores.** The manifest reports measurements and
   contributing semantic sets. It does not reduce maintainability to one opaque
   score.
2. **Deltas before warning floods.** Unchanged accepted debt remains visible but
   does not become a new regression.
3. **Aggregate responsibility matters.** Moving one function into many private
   helpers must not erase concentration at module, dependency-component, or
   declared architecture-group scope.
4. **Policy is separate from semantics.** The compiler defines facts and policy
   evaluation. A project selects thresholds, dependency rules, and enforcement
   dispositions.
5. **Incomplete means incomplete.** Missing source, unsupported constructs, or
   exhausted required analysis budgets cannot produce a clean result.
6. **Every result is actionable and auditable.** Findings identify the semantic
   facts and edges that caused them; prose is a derived rendering.

## Terms

**Executable unit:** a function, method, initializer, handler, closure with an
independent body, or other callable body defined by the language.

**Aggregate scope:** a module, a strongly connected dependency component, or a
project-declared architecture group evaluated as one responsibility boundary.

**Architecture group:** a versioned project-policy selector that assigns
semantic identities to a named responsibility such as `transport`, `domain`,
or `persistence-adapter`. Group names have no universal AIL meaning.

**Snapshot:** architectural facts and policy context for one source revision.

**Delta:** the typed difference between two compatible snapshots.

**Observation:** a reported fact that does not assert a policy failure.

**Regression:** a delta that violates a configured no-growth, baseline, or
change rule.

**Violation:** a current-revision fact that violates an absolute, dependency,
authority, cycle, or ownership rule.

**Hotspot:** a semantic scope selected by a policy threshold, no-growth rule, or
accepted baseline entry. A large unit is not automatically a hotspot unless a
policy classifies it as one.

**Coverage:** the analyzed current-build scope and every boundary that prevents
the compiler from claiming completeness.

## Inputs and identity

An architectural analysis is identified by:

- source revision;
- dependency-lock revision;
- compiler and language version;
- target and build configuration;
- architecture-policy revision;
- accepted-baseline revision, when used;
- analysis scope; and
- manifest schema version.

Every semantic contributor uses a handle scoped to the source revision.
Snapshot comparison uses the compiler's revision identity map. Persistent
schema, ABI, or service identities remain distinct from source-revision
handles.

Snapshots are comparable only when their language semantics, target,
dependency coverage, metric definitions, policy interpretation, and manifest
schema are compatible. An incompatible comparison returns a structured cause;
it must not approximate a delta.

## Analysis scopes

The initial feature analyzes:

- each executable unit;
- each module;
- each strongly connected component in the static dependency graph;
- each architecture group selected by project policy; and
- the complete analyzed build as a summary scope.

The static dependency graph contains typed direct relationships available to
the compiler, including calls, construction, type use, capability delegation,
effect propagation, state-domain access, serialization, adaptation, and
generated-source relationships.

Aggregate values must be computed from the underlying semantic graph. They must
not be calculated by blindly summing per-function values when doing so would
double-count a shared dependency, capability, state domain, or call site.

## Required metric catalog

The first manifest reports the following primitive metrics and sets. Future
schema versions may add metrics, but may not silently redefine an existing
metric.

### Source and control flow

`canonical_source_lines`
: Number of canonical source lines intersecting the scope's declarations and
  executable bodies, excluding blank lines. This is a presentation metric, not
  a semantic complexity score.

`executable_node_count`
: Number of typed, pre-optimization executable semantic nodes in the scope.
  Generated implicit operations are listed separately and included only when
  their behavior is observable.

`control_flow_complexity`
: For each executable unit, `E - N + 2`, where `E` and `N` are the edge and node
  counts of its connected, single-entry control-flow graph before optimization.
  Constant folding does not remove source branches from this metric. Aggregate
  scopes report the maximum, sum, and ordered contributing units rather than
  constructing an artificial combined control-flow graph.

`decision_point_count`
: Number of semantic control-flow nodes with more than one possible successor,
  plus each additional independently evaluated matching guard. The manifest
  groups counts by construct category.

`maximum_control_nesting`
: Maximum number of nested control-flow regions in an executable unit. Aggregate
  scopes report the maximum and its contributors.

### Dependencies and concentration

`direct_dependency_set`
: Unique semantic identities directly required by the scope, grouped by
  relationship kind.

`direct_dependent_set`
: Unique analyzed semantic identities that directly depend on the scope,
  grouped by relationship kind.

`dependency_component_size`
: Number of semantic identities in the scope's strongly connected dependency
  component. Acyclic identities report one.

`public_entrypoint_reach`
: The set of analyzed public entrypoints whose static dependency paths include
  the scope. The manifest reports the set and count, not only a percentage.

`static_call_site_count`
: Number of typed source call sites in the scope, grouped by local, dependency,
  capability, and foreign target.

### Authority, effects, and state

`declared_capability_set`
: Capability instances or namespaces explicitly required by the scope's public
  contract.

`transitive_capability_set`
: Capability instances or namespaces required by statically reachable work,
  with one or more minimal semantic paths from the scope to each capability.

`declared_effect_set`
: Effects declared by the scope's public contract.

`elaborated_effect_set`
: Complete inferred and explicit effect set for the scope, with contributors.

`state_read_set`
: Statically identifiable state domains read by the scope, with access sites.

`state_write_set`
: Statically identifiable state domains written by the scope, with access
  sites.

`external_operation_site_count`
: Typed call sites that cross a capability or foreign-system boundary, grouped
  by capability and operation identity. This is a static site count, not a
  runtime invocation estimate.

### Outcomes and review context

`declared_outcome_set`
: Closed public success and domain-failure variants returned or propagated by
  the scope.

`recursive_component`
: Ordered identities in any static call-graph component containing recursion,
  plus whether the recursion is direct or mutual and its declared bound status.

`minimal_context_node_count`
: Semantic nodes in the compiler-defined minimal direct context needed to show
  the scope's contract, direct dependencies, capabilities, effects, state
  domains, outcomes, applicable invariants, and policy contributors.

`minimal_context_encoded_bytes`
: Size of the canonical transport-independent encoding of that minimal context.
  Model-token measurements remain benchmark facts because tokenizers are not
  language semantics.

`analysis_gap_set`
: Unsupported, unavailable, generated, foreign, or dynamic boundaries that
  limit completeness, with reason and affected metric categories.

## Required derived changes

A delta reports old and new values or sets for every changed metric. It also
classifies:

- new, enlarged, reduced, unchanged, and removed hotspots;
- added and removed dependency relationships;
- added and removed declared or transitive capabilities;
- added and removed effects;
- added and removed state-domain reads and writes;
- new, enlarged, reduced, and removed dependency or recursion components;
- context growth and reduction;
- coverage growth and loss;
- policy and baseline changes; and
- exception additions, changes, use, and removal.

The compiler must distinguish a semantic change from a measurement change
caused by a metric-schema or configuration change. Incompatible measurements
do not produce an architectural regression classification.

## Project policy model

Project policy is a versioned input to compilation and structural validation.
The transport-independent policy model supports:

- absolute numeric or set thresholds;
- no-growth rules relative to an accepted baseline;
- allowed and forbidden dependencies between selectors or architecture groups;
- allowed and forbidden capabilities, effects, and state domains by selector;
- prohibition or size limits for dependency and recursion components;
- required coverage for selected build profiles;
- dispositions of `record`, `warn`, or `deny`; and
- scoped exceptions.

A selector may identify public semantic identity, module, semantic kind,
architecture group, or a deterministic combination of those fields. Selectors
must not depend solely on unstable source line numbers.

Policy evaluation is deterministic. When multiple rules match, every result is
reported. A more permissive rule does not silently override a stricter rule
unless the policy explicitly declares and validates that precedence.

### Behavior illustration: policy model

This illustrates data, not an accepted configuration syntax:

```text
rule ARCH-TRANSPORT-AUTHORITY:
  select group transport
  forbid capabilities jobs_store, clock, outbound_network
  disposition deny

rule ARCH-DISPATCH-NO-GROWTH:
  select semantic identity service.dispatch
  compare accepted baseline
  forbid growth in:
    control_flow_complexity
    transitive_capability_set
    state_write_set
    minimal_context_node_count
  disposition deny

rule ARCH-NEW-UNIT-COMPLEXITY:
  select new executable units
  require control_flow_complexity <= calibrated_limit
  disposition warn
```

## Accepted baseline

A baseline records policy-relevant facts for known debt at a specific compatible
revision. Its purpose is to permit an unchanged hotspot while preventing
unreviewed growth.

A baseline entry contains:

- policy rule code;
- semantic selector and resolved identity at the baseline revision;
- relevant metric values or sets;
- source, dependency, compiler, configuration, policy, and schema revisions;
- rationale;
- approval identity; and
- identity-map history needed to match later revisions.

Normal source editing cannot rewrite the accepted baseline. Baseline creation,
replacement, or broadening is a distinct governance change shown in the
architectural delta and completion evidence.

If an identity cannot be mapped safely, the rule result is `stale_baseline`.
The compiler must not treat an unmatched entry as permission for a new hotspot.

## Exceptions

An exception is a versioned project-policy artifact, not a source comment or
compiler flag supplied implicitly by an agent.

Each exception contains:

- stable policy rule code;
- exact semantic selector;
- permitted metric value, set difference, or relationship;
- rationale;
- approval identity;
- policy revision in which it was introduced; and
- explicit review boundary such as a later policy revision or release
  identity.

An exception applies only to the matching rule, scope, and permitted difference.
It cannot suppress unrelated findings or incomplete analysis. Creating,
broadening, using, narrowing, or removing an exception appears in the delta.

## Results and diagnostics

The manifest contains all observations. Results selected by policy additionally
produce structured diagnostics.

Every diagnostic includes:

- stable code;
- classification: `regression`, `violation`, `incomplete`, or
  `policy_invalid`;
- disposition: `record`, `warn`, or `deny`;
- source and policy revisions;
- primary semantic identity and location;
- old and new facts when applicable;
- threshold, boundary, or relationship evaluated;
- contributing semantic identities and edges;
- aggregate scopes affected;
- matching baseline or exception;
- coverage limitations;
- related diagnostics and causal chain; and
- supported structural-operation categories, when known.

Prose such as “function too complex” is never the authoritative result.

## Compilation and validation behavior

Architectural analysis does not change ordinary program semantics.

For every compilation profile that enables the feature:

1. semantic analysis produces architectural facts;
2. the compiler emits a compact summary in the normal compilation result;
3. the full snapshot is available through the protocol or requested artifact;
4. a compatible base revision produces a delta;
5. policy is evaluated after semantic facts are available;
6. `warn` results do not fail compilation;
7. unexcepted `deny` results prevent the revision from being reported as
   successfully validated; and
8. incomplete facts required by a `deny` rule produce a denied `incomplete`
   result rather than a pass.

Compilation without an architecture policy may still produce a fact-only
snapshot. Universal AIL conformance must not depend on a project's chosen
thresholds.

If ordinary semantic errors prevent complete architectural analysis, the
compiler returns a partial snapshot with explicit coverage. It does not issue a
clean architectural completion summary.

## Agent workflow and structural operations

Before editing a classified hotspot, an agent can request:

- the matching policy and baseline;
- the metrics and semantic contributors;
- the smallest complete context for the relevant finding;
- changed entrypoints and downstream consumers;
- capability, effect, state, and dependency paths; and
- structural operation categories supported by the compiler.

Candidate operation categories include function extraction, move, dependency
inversion through an existing contract, capability narrowing, and operation
registration. A recommendation is advisory: the compiler validates the
resulting complete revision rather than assuming that smaller functions imply a
better architecture.

After editing, the agent validates the full architecture delta together with
semantic, behavior, and test evidence. A structural transaction cannot commit a
revision containing an unexcepted denied result.

## Determinism, ordering, and budgets

The manifest uses stable field ordering and semantic-identity ordering defined
by the eventual protocol specification. Set ordering must not depend on hash
iteration, filesystem order, process identity, or parallel scheduling.

Identical inputs produce byte-equivalent canonical manifest encodings where the
selected encoding permits it.

Analysis reports:

- elapsed analysis work in implementation-independent units where defined and
  wall time as benchmark evidence;
- cache reuse status;
- analyzed semantic-node and edge counts;
- peak analysis memory where measurable;
- manifest encoded size;
- configured budgets; and
- any truncated advisory detail.

An implementation may truncate optional explanatory paths after a configured
limit. It may not truncate the facts necessary to evaluate a `deny` rule and
then report success. If those facts cannot be completed within the budget, the
result is `incomplete`.

## Manifest shape

### Behavior illustration: transport-independent structure

This is a field model, not a selected wire encoding:

```text
ArchitectureHealthManifest {
  schema_version
  analysis_identity {
    source_revision
    dependency_revision
    compiler_version
    language_version
    target
    build_configuration
    policy_revision
    baseline_revision?
  }
  scope
  coverage
  metrics_by_semantic_scope[]
  dependency_components[]
  policy_rules[]
  baseline_matches[]
  exceptions[]
  observations[]
  diagnostics[]
  analysis_cost
}

ArchitectureHealthDelta {
  schema_version
  base_analysis_identity
  candidate_analysis_identity
  compatibility
  identity_map
  metric_changes[]
  relationship_changes[]
  hotspot_changes[]
  coverage_changes[]
  policy_changes[]
  baseline_changes[]
  exception_changes[]
  diagnostics[]
}
```

## Proposed initial diagnostic codes

These codes are proposed for the first protocol version. They become stable only
after acceptance in a numbered protocol specification.

| Code | Meaning |
| --- | --- |
| `ARCH.NEW_HOTSPOT` | a new scope crosses a configured threshold |
| `ARCH.HOTSPOT_GROWTH` | an accepted or existing hotspot grows in a prohibited fact |
| `ARCH.DEPENDENCY_BOUNDARY` | a forbidden semantic dependency is present |
| `ARCH.AUTHORITY_BOUNDARY` | a capability, effect, or state domain is outside policy |
| `ARCH.DEPENDENCY_CYCLE` | a new or enlarged prohibited component exists |
| `ARCH.CONTEXT_BUDGET` | review context crosses a configured absolute or no-growth rule |
| `ARCH.COVERAGE_LOSS` | the candidate loses required analyzability |
| `ARCH.ANALYSIS_INCOMPLETE` | required facts cannot be completed within coverage or budget |
| `ARCH.STALE_BASELINE` | accepted debt cannot be mapped safely |
| `ARCH.INVALID_POLICY` | policy is ambiguous, incompatible, or references invalid selectors |
| `ARCH.UNAUTHORIZED_GOVERNANCE_CHANGE` | policy, baseline, or exception changed outside task authority |

## Conformance evidence

Before this feature becomes normative, proposed fixtures must cover:

1. repeated deterministic snapshot generation;
2. a new decision-heavy function;
3. growth of an accepted hotspot;
4. unchanged accepted debt;
5. superficial extraction into private helpers;
6. aggregate module and dependency-component concentration;
7. forbidden dependency and capability edges;
8. new and enlarged cycles;
9. context growth;
10. complete, partial, and lost coverage;
11. compatible and incompatible snapshot comparison;
12. identity mapping across rename, move, replace, and removal;
13. baseline creation, match, staleness, and unauthorized rewrite;
14. exception creation, exact use, over-broad use, and removal;
15. `record`, `warn`, and `deny` dispositions;
16. denied structural transaction rollback; and
17. bounded analysis that returns `incomplete` instead of a false pass.

The UC-007 benchmark must additionally show that an agent can use this evidence
to avoid or repair a seeded architectural regression.

## Non-goals

This feature does not:

- prove that a program has the best possible architecture;
- assign semantic meaning to a universal line-count or complexity threshold;
- replace behavior tests, performance benchmarks, or human review;
- make a large unit invalid in every project;
- guarantee architectural properties beyond analyzed source;
- infer business responsibilities from names or model judgment;
- treat source-file splitting as sufficient modularity; or
- authorize automatic refactoring without validating the complete result.

## Open decisions

Before normative rules are accepted:

1. freeze the exact metric names and semantic graph relationship kinds;
2. decide which metrics are required in the first protocol version;
3. define canonical manifest encoding and ordering;
4. define architecture-group selector semantics;
5. decide how policy and baseline governance authority is represented;
6. calibrate default advisory thresholds, if any, from strong baselines;
7. set analysis and manifest budgets for UC-007;
8. specify generated and foreign-source coverage;
9. define incremental invalidation requirements; and
10. decide which structural operations the first agent protocol advertises.
