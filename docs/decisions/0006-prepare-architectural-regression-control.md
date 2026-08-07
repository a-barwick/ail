# ADR 0006: Enforce architecture on atomic changes

- Status: Accepted and complete
- Date: 2026-07-21
- Owners: project maintainers
- Documentation layer and scope: M23–M27 architecture behavior

## Context

M21 could prove that a multi-file schema change was complete and behaviorally
correct. It could not reject a change that passed every test while growing a
central dispatcher, moving store authority into transport, or hiding the same
responsibility in private helpers.

UC-007 defines three behavior-equivalent `CancelJob` candidates: domain-owned,
centralized, and helper-split. The compiler needs facts and policy at both
function and aggregate scopes to distinguish them.

## Decision

Add revision-bound architecture analysis to the existing atomic transaction:

- derive primitive metrics and contributors from the semantic graph;
- report function, module, dependency-component, and configured-group scopes;
- compare compatible base and candidate revisions;
- apply explicit dependency, capability, state, cycle, hotspot, baseline, and
  exception rules;
- render compact text from the same structured result; and
- publish no child when policy denies the change or required analysis is
  incomplete.

The compiler reports facts. Project policy chooses thresholds. AIL does not
infer the best architecture or emit one maintainability score.

## Consequences

- The domain-owned candidate publishes.
- The centralized candidate is rejected for dispatch growth and transport-owned
  dependencies, jobs-store authority, and jobs state.
- The helper-split candidate is rejected at aggregate transport scope even when
  each helper stays below individual thresholds.
- Existing hotspot debt remains visible but does not fail a no-growth rule until
  the candidate enlarges it.
- Budget or coverage failure returns `incomplete`, not success.
- M27 records one agent using the output to repair the centralized candidate.

## Alternatives considered

### Function complexity only

Rejected because helper splitting evades it without changing responsibility.

### Automatic architecture inference

Rejected because business boundaries and acceptable debt are project choices.
The compiler can measure semantic relationships but cannot infer product
ownership from names.

### Add concurrency or native execution

Rejected for this work. Neither addresses behaviorally correct architecture
regressions, and both require separate semantics.

## Validation

`python3 specs/tools/architecture_acceptance.py check` derives the three case
classifications and rejects 37 mutations. The M25 and M26 tests must match the
structured and compact results, reject every denied or incomplete candidate,
and preserve the base revision on failure.
