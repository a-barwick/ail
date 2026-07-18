# UC-003 — Public schema evolution

Status: **Accepted 2026-07-18**

Documentation layer: concrete application scenario. This record is
language-independent and non-normative for AIL syntax or semantics.

## Actor and desired outcome

A maintainer asks an agent to add job priority to the UC-001 reference system.
The agent must identify every affected producer, consumer, test, persistent
shape, and serialization boundary; update them consistently; preserve an
explicit compatibility policy; and provide complete review evidence.

The desired outcome is a revision-safe change whose semantic consequences can be
known before commit and verified after commit without relying on filename
conventions or an agent's unaudited memory.

## System boundary

UC-003 uses the logical job-submission boundary from UC-001 and adds the
repository-level change boundary around it.

Inside the boundary:

- versioned public request and result schemas;
- the handler and its constructed `Job`;
- the job-store capability contract and deterministic implementation;
- persisted job encoding and stored fixtures;
- version adapters used during the compatibility window;
- all statically known constructors, field reads, destructuring operations,
  serializers, deserializers, tests, and invariants for the changed shapes;
- semantic impact analysis at a specific source revision; and
- validation of the complete multi-artifact change.

Outside the boundary:

- deployment rollout and coordination with unknown external clients;
- schema registries and production database migration systems;
- dynamically generated consumers that are absent from the analyzed workspace;
- reflective access that the compiler cannot represent;
- runtime concurrency and job execution; and
- the choice of AIL syntax or compiler implementation.

The impact report must clearly separate complete workspace knowledge from
external dependencies declared by contract.

## Inputs and required capabilities

The change task receives:

1. a fixed source revision implementing UC-001 version 1;
2. the requested version 2 behavior below;
3. the workspace and locked dependency graph;
4. permission to inspect, edit, format, build, and test that workspace; and
5. no permission to alter the task, correctness oracle, compatibility window, or
   measurement policy.

Runtime authority remains the same as UC-001: the handler has only the supplied
jobs-store insertion capability. Adding priority must not add time, randomness,
network, filesystem, environment, telemetry, or ambient datastore authority.

## Schema, serialization, and compatibility

### Behavior illustration: version 1

This structural notation is not AIL source and does not select a wire format.

```text
CreateJobRequestV1 {
  job_id
  task
  payload
}

JobV1 {
  job_id
  task
  payload
}

CreatedV1 {
  job: JobV1
}
```

### Behavior illustration: version 2

```text
Priority = Low | Normal | High

CreateJobRequestV2 {
  job_id
  task
  payload
  priority: Priority
}

JobV2 {
  job_id
  task
  payload
  priority: Priority
}

CreatedV2 {
  job: JobV2
}
```

Version 2 requires an explicit `priority` from version 2 producers. Its three
values and serialized identities are closed:

| Value | Stable serialized identity |
| --- | --- |
| `Low` | `low` |
| `Normal` | `normal` |
| `High` | `high` |

The handler copies the validated priority unchanged into `JobV2`; the store
persists it; and `CreatedV2` returns it.

During the compatibility window:

- a version 1 request is decoded as version 1 and explicitly adapted to version
  2 with `priority = Normal`;
- a version 2 request without `priority` is invalid rather than silently
  defaulted;
- persisted version 1 jobs are explicitly decoded as version 1 and adapted with
  `priority = Normal`;
- newly persisted jobs use version 2 and contain priority; and
- a V1 response projection contains only the V1 job fields and deliberately
  omits the internal priority; and
- version identities are carried by the fixture encoding rather than inferred
  from field presence.

Removing version 1 adapters is a separate change after the compatibility window.

## Nominal evolved behavior

### Behavior illustration: nominal execution trace

Given a valid version 2 request with `priority = High` and an empty store:

```text
1. decode the fixture as CreateJobRequestV2
2. validate job_id
3. validate task
4. validate payload
5. validate that priority is one of Low, Normal, or High
6. construct JobV2 with priority High
7. call jobs.insert_if_absent(JobV2)
8. observe store outcome inserted
9. encode and return CreatedV2(JobV2)
```

Given the equivalent version 1 request:

```text
1. decode the fixture as CreateJobRequestV1
2. adapt it to the version 2 internal request with priority Normal
3. perform the same validation, insert, and result selection
4. persist JobV2 with priority Normal
5. return through the version selected by the fixture contract
```

The chosen response-version rule must be visible in the fixture contract.
The initial rule is symmetric: a version 1 request receives a version 1 response
projection, while a version 2 request receives a version 2 response.

## Domain failures and language/runtime faults

### Behavior illustration: evolved failure table

| Condition | Result | Store calls | Compatibility evidence |
| --- | --- | ---: | --- |
| V2 priority is `Low`, `Normal`, or `High` | normal UC-001 processing | 1 for otherwise valid input | V2 round-trip fixture |
| V2 priority is absent | `Invalid(priority, missing)` | 0 | negative V2 fixture |
| V2 priority has an unknown identity | fixture decode failure at the public boundary | 0 | stable decode diagnostic |
| V1 request during compatibility window | normal processing with `Normal` internally | 1 for otherwise valid input | V1 request and response golden fixtures |
| V1 persisted job during compatibility window | decode as `JobV1`, then adapt to `JobV2(Normal)` | 0 handler calls | V1 stored golden fixture |
| duplicate V1 or V2 job identifier | `AlreadyExists(job_id)` in the request's response version | 1 | unchanged pre-existing record |
| persistence unavailable | version-appropriate `PersistenceUnavailable` | 1 | unchanged state |

An unknown priority identity is a boundary decode failure because no valid
structured `CreateJobRequestV2` exists. The transport-independent fixture oracle
must distinguish this from a valid structured request that fails domain
validation.

All structured-request validation, duplicate, and known persistence outcomes
remain the closed domain results defined by UC-001. UC-003 adds no unchecked
domain failure. Resource exhaustion, violated adapter or capability
postconditions, corrupt compiler/runtime state, and other failures outside those
closed results remain language/runtime faults. This use case requires them to be
distinguishable in evidence but does not decide whether they are catchable.

## Observable state and ordered effects

The observable behavior remains the UC-001 tuple of final state, returned
result, and ordered store-call trace, extended with:

- selected request and response schema versions;
- any explicit V1-to-V2 adaptation;
- stored schema version;
- serialized priority identity; and
- the revision-scoped impact and semantic-diff reports.

Adaptation and validation occur before the one permitted store call. Invalid or
undecodable priority produces no store call. There is no concurrency,
cancellation, retry, timeout, or unbounded growth.

## Affected producer and consumer inventory

The frozen reference workspace must contain at least these semantic roles:

| Role | Version 2 change |
| --- | --- |
| V2 request constructor or client fixture | supplies an explicit priority |
| V1 request adapter | selects `Normal` explicitly |
| request decoder | recognizes the V2 field and closed identities |
| request validator | rejects missing or unknown V2 priority |
| `create_job` handler | propagates priority into the job |
| `Job` constructor | requires priority |
| job-store capability contract | accepts the evolved stored job |
| deterministic store | preserves priority exactly |
| persisted-job encoder | writes the V2 version and priority identity |
| V1 persisted-job decoder/adapter | supplies `Normal` explicitly |
| created-result constructor and projection | preserves or intentionally omits priority by response version |
| field readers or destructuring sites | handle the added field under the language's rules |
| behavior, compatibility, and golden-fixture tests | cover V1 and V2 success and failure |
| human review manifest | reports every consequence and explicit non-change |

The fixture may distribute these roles across files or combine them. Impact
completeness is evaluated over semantic roles and dependencies, not file count.

## Expected semantic-impact report

### Behavior illustration: pre-edit impact

A conforming semantic query at source revision `R1` should report, in a stable
machine-readable order:

```text
changed contract:
  CreateJobRequest: add required field priority: Priority
  Job: add required field priority: Priority
  Created result payload: Job changes transitively

new closed contract:
  Priority: Low | Normal | High

directly affected:
  all constructors of CreateJobRequest and Job
  request and persisted-job serializers/deserializers
  handler propagation from request to Job
  store capability signatures and implementations
  response projections
  complete-record patterns or destructuring sites

compatibility obligations:
  V1 request -> V2 internal request uses Normal
  V1 stored Job -> V2 Job uses Normal
  V1 response projection remains stable
  V2 fixtures require explicit priority

tests and fixtures:
  nominal Low, Normal, and High
  missing and unknown V2 priority
  V1 request compatibility
  V1 persisted-record compatibility
  duplicate and unavailable outcomes in both request versions

semantic non-changes:
  datastore authority unchanged
  effect set unchanged
  store-call count and ordering unchanged
  UC-001 non-priority validation unchanged

external coverage:
  current build and available locked dependency source: complete
  unavailable dependency source and external clients: listed as unchecked
```

The report has two action lists. `must_change` contains only locations that
definitely need an edit and must be exact. `review` contains locations that may
need attention and may include at most two unnecessary entries in this first
workload. External systems that the compiler cannot inspect are listed
separately. Renames must not make a dependency disappear from the report.

## Representative agent change

Task contract:

> At revision R1, add the required closed `priority` field and version 1
> compatibility behavior defined by UC-003. Identify the complete semantic
> impact before editing. Update all affected workspace artifacts, preserve the
> UC-001 effect contract, and return completion evidence for revision R2.

The agent may use normal structured compiler, language-server, build, formatter,
and test tools. It must not be given a manually curated file list containing the
answer.

### Behavior illustration: completion evidence

The task is complete only when evidence for R2 includes:

```text
pre-edit:
  impact report bound to R1

edit:
  changed semantic identities
  R1 -> R2 identity map
  formatted textual diff

post-edit:
  no unresolved affected workspace consumers
  public-contract and effect semantic diff
  build and static checks pass
  UC-001 regression suite passes
  V1 compatibility suite passes
  V2 priority suite passes
  golden request, response, and stored-record fixtures pass
  authority and ordered-effect trace match the declared non-changes
```

If the edit is offered as a structural transaction, the transaction must either
produce the validated R2 revision or leave R1 unchanged. A sequence of ordinary
baseline edits is permitted, but only its final validated revision counts as
task completion.

## Human review and operational evidence

A reviewer must be able to see:

- the original and evolved public and stored shapes;
- whether each new field is required, defaulted, or projected away, and where;
- stable serialized identities and version-selection rules;
- every affected semantic role and its post-edit disposition;
- any dependency that could not be analyzed;
- changes to public API, errors, effects, authority, state, and compatibility;
- the revision identity for every report and diagnostic;
- the exact test and golden-fixture results; and
- any source change not explained by the requested evolution.

The semantic report supplements rather than replaces the canonical text diff.

## Baseline implementation and tools

UC-003 uses the Rust, Go, Python, and TypeScript baseline families and normal
tooling listed in UC-001. Each starts from behaviorally equivalent version 1
source and receives the same task contract, fixture corpus, workspace roles, and
compatibility oracle.

Before agent runs, freeze:

- source and dependency revisions;
- task wording and initially supplied context;
- available compiler, language-server, search, format, build, and test tools;
- model and agent versions;
- correctness and regression tests, including hidden tests;
- runtime environment and performance envelope;
- token and tool-output accounting;
- timeout, retry, and permitted repair policy; and
- the definition of an affected consumer and a regression.

Baseline tools may report less semantic information than AIL eventually
requires. That limitation is a measured result, not grounds to withhold normal
tools or manually simplify a baseline.

## Measurable success criteria

UC-003 is satisfied when:

1. the pre-edit `must_change` list finds every seeded location that needs an
   edit, contains no unnecessary entries, and makes no claim beyond the current
   build;
2. all V2 constructors and fixtures provide priority explicitly;
3. every priority value round-trips through request, handler, store, and V2
   result without alteration;
4. V1 requests and stored records select `Normal` only through the declared
   adapters and preserve the V1 response projection;
5. missing or unknown V2 priority produces the specified zero-effect failure;
6. no UC-001 validation, result, authority, or ordering regression occurs;
7. all post-edit checks are bound to R2 and stale R1 handles are rejected or
   remapped explicitly;
8. the run records complete context and repair measurements under the frozen
   benchmark rules; and
9. a reviewer can account for every semantic and textual change using the
   supplied evidence.

## Derived requirements

- APP-003, APP-004, APP-005
- LANG-001, LANG-002, LANG-003, LANG-004, LANG-005
- PROTO-001, PROTO-002, PROTO-003, PROTO-004, PROTO-005
- NFR-001, NFR-002, NFR-003, NFR-004, NFR-005

See [the initial reference-slice requirements](../requirements/reference-slice.md).

## Explicit exclusions and unresolved questions

Excluded from this slice:

- automatic negotiation with unknown deployed clients;
- a general migration language or schema registry;
- online database rollout and rollback;
- removing V1 support;
- compatibility across dynamically loaded or reflective consumers;
- changes to effects, authority, concurrency, scheduling, or resource policy;
- a normative AIL syntax for records, enums, versions, adapters, or queries; and
- a requirement that all baseline tools expose AIL-equivalent impact data.

Accepted decisions:

- Existing requests and stored jobs receive `Normal` priority through an
  explicit version adapter.
- A V1 request receives a V1 response. A V2 request receives a V2 response.
- Shared cases use JSON. Each request, response, and stored record carries its
  version explicitly.
- The compiler's complete-results promise covers all application source, tests,
  fixtures, checked-in schemas, generated code used by the build, and locked
  dependency source available to the compiler.
- When generated code is affected, the report points to both the generated
  output and the editable generator input or schema. Unavailable dependency
  source and external services are listed as unchecked.
- The impact report separates `must_change` from `review`. `must_change` must be
  exact. `review` may contain at most two unnecessary entries in this workload,
  each with a reason.
