# AIL use cases

Use cases define application behavior and representative changes. They do not
define AIL syntax.

## Implemented workloads

| Use case | What the system must do | Compiler capability exercised |
| --- | --- | --- |
| [UC-001](UC-001-request-validation-and-persistence.md) | Validate a create-job request, make at most one jobs-store call, and return a closed result | types, variants, effects, deterministic interpretation |
| [UC-003](UC-003-public-schema-evolution.md) | Add priority to public and stored schemas and update every affected consumer | schema identity, impact query, atomic multi-file validation |
| [UC-007](UC-007-architectural-regression-control.md) | Add `CancelJob` without growing dispatch or moving store authority into transport | architecture snapshots, deltas, policy, rollback |

UC-001 and UC-003 were accepted on 2026-07-18. UC-007 was accepted on
2026-07-22. Their exact requirements are under
[`docs/requirements/`](../requirements/README.md).

## Proposed workload

[UC-008](UC-008-iterative-service-evolution.md) asks whether fresh agents can
apply a sequence of product changes without cumulative context, repair, and
architecture costs exploding. The workload and requirement sequence are not
defined, and no work on it is active. The old M29–M36 plan is retained only in
delivery history.

## Candidate behaviors

The next compiler increment should start from one executable behavior the
current language cannot express:

- an outbound call with explicit timeout, cancellation, authority, and typed
  failure;
- bounded fan-out with fixed result order and child-failure behavior;
- an event worker with idempotency and deterministic acknowledgement; or
- replay of supplied time, randomness, configuration, filesystem, or network
  values.

These are options, not queued work.

## Required contents

A use case must state:

- system boundary and actor;
- input, output, state, and capability authority;
- exact success, domain-error, and runtime-fault behavior;
- effect order, concurrency, cancellation, and resource bounds;
- schema and compatibility rules;
- the change an agent must make;
- executable correctness checks; and
- what a human must inspect to approve the result.

Use language-independent tables, traces, and data shapes first. An AIL example
does not become required behavior until a numbered specification and fixture say
so.
