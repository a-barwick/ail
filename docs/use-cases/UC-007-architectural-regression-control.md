# UC-007 — Add an operation without degrading architecture

Status: **Accepted 2026-07-22**

## Task

Add `CancelJob` to a 24-operation job service. The operation may conditionally
update the `jobs` state domain through `jobs_store.cancel_if_active`. It may not
use clock, network, or telemetry authority.

Malformed requests make no store call. Queued and running jobs become cancelled
after exactly one store call. Missing jobs return `NotFound`; completed or
already-cancelled jobs return `NotCancellable`. Those cases make one call and no
state change.

The implementation must not grow the existing dispatch hotspot, create a
forbidden dependency, move jobs-store authority or jobs state into transport,
or create a dependency cycle.

## Why behavior tests are insufficient

All three test candidates pass the six behavior cases:

1. **Domain-owned:** transport forwards to a domain handler. This passes.
2. **Centralized:** dispatch performs decoding, decisions, and store access.
   Behavior passes, but architecture policy rejects the change.
3. **Helper split:** transport delegates the same responsibilities to private
   helpers. Individual functions remain small, but aggregate transport authority,
   state, and dependencies still violate policy.

The third case is the reason architecture must be measured at function, module,
dependency-component, and configured-group scopes.

## Compiler behavior

For the base revision, the compiler returns a revision-bound snapshot containing
coverage, metric values, contributors, policy, baseline matches, and unchecked
boundaries.

For a candidate, it returns the compatible delta, changed hotspots,
dependencies, capabilities, state access, cycles, policy results, and exact
contributors. Results use deterministic ordering and stable diagnostic codes.

A denied result publishes no child revision. Missing source, unsupported input,
or a required analysis that exceeds its budget returns `incomplete`; it cannot
be reported as success. An exception must match the exact rule, scope,
contributors, policy revision, and review boundary.

## Implemented metrics and policy

The accepted M24 contract covers seven primitive metrics or sets:

- control-flow complexity;
- direct dependencies;
- directly held or used capabilities;
- state reads;
- state writes;
- dependency-component size; and
- minimal context node count.

Policy covers allowed group dependencies, empty transport capability and state
sets, no growth of the existing dispatch hotspot, limits for new units, no new
cycles, complete coverage, and unchanged policy/baseline/exception inputs.

The compiler reports facts and contributors. The project chooses thresholds and
which findings deny publication. AIL does not define one universal complexity
limit.

## Completion result

A valid change provides passing behavior, the semantic impact and effect trace,
base and candidate snapshots, the architecture delta, canonical text edits,
semantic changes, complete coverage, and no denied result.

The M25 and M26 tests exercise repeated snapshots, all three candidates,
unchanged existing debt, hotspot growth, forbidden edges, stale baselines,
exceptions, budget failures, and atomic rollback. M27 recorded one agent repair
of the centralized candidate using the compact and structured diagnostics.

That recorded repair is one usability result, not a cross-language comparison.
Wall time, memory, repair counts, and model context still need measurement
against Rust, Go, Python, and TypeScript.

## Requirements

This use case owns `APP-006`, `LANG-006`, `PROTO-006`, `PROTO-007`, `NFR-006`,
and `NFR-007` in
[the architecture requirements](../requirements/architectural-health.md).
Exact fixtures and budgets are in
[the architecture case](../architecture-acceptance.md) and
[`specs/architecture.md`](../../specs/architecture.md).
