# UC-001 — Request validation and persistence

Status: **Accepted 2026-07-18**

Documentation layer: concrete application scenario. This record is
language-independent and non-normative for AIL syntax or semantics.

## Actor and desired outcome

A caller submits one structured request to create a job. The service validates
the complete request, attempts at most one persistent write through an explicit
datastore capability, and returns one member of a closed result set.

The representative agent task is to implement or repair the handler from this
record and its tests without acquiring undeclared authority or changing the
observable behavior.

## Reference system and system boundary

The reference system is a small logical job-submission service. Its boundary
starts after transport decoding has produced a structured `CreateJobRequest` and
ends when the handler returns a structured `CreateJobResult`.

Inside the boundary:

- validation of every request field;
- construction of a persistent job record;
- one conditional insert through the supplied job-store capability;
- translation of store outcomes into public result variants; and
- the ordering of those operations.

Outside the boundary:

- HTTP, RPC, queue, or command-line transport;
- authentication and authorization;
- allocation of a database connection;
- datastore vendor behavior below the capability contract;
- job scheduling or execution;
- wall-clock time, randomness, logging, and telemetry; and
- process startup, routing, retries, and deployment.

The first reference behavior therefore needs no networking stack or production
database. A deterministic store implementation can exercise the complete
boundary.

## Inputs and required capabilities

The handler receives:

1. one `CreateJobRequest`; and
2. one capability granting conditional insertion into the `jobs` namespace.

No clock, random-number generator, environment, filesystem, network, telemetry,
or ambient datastore access is available. The caller supplies the job
identifier so successful execution does not require hidden nondeterminism.

### Behavior illustration: request and result shapes

The notation below describes data, not AIL source or a required wire encoding.

| Shape | Field | Constraint |
| --- | --- | --- |
| `CreateJobRequest` | `job_id` | ASCII text matching `[A-Za-z0-9][A-Za-z0-9_-]{0,63}` |
| `CreateJobRequest` | `task` | UTF-8 text containing 1–80 Unicode scalar values and no control characters |
| `CreateJobRequest` | `payload` | Opaque byte sequence of 0–4096 bytes |
| `Job` | `job_id` | The validated request identifier |
| `Job` | `task` | The validated request task, unchanged |
| `Job` | `payload` | The validated request payload, unchanged |

`CreateJobResult` is exactly one of:

| Variant | Data | Meaning |
| --- | --- | --- |
| `Created` | the persisted `Job` | The conditional insert succeeded |
| `Invalid` | a non-empty ordered list of `ValidationIssue` values | One or more request fields failed validation |
| `AlreadyExists` | `job_id` | A job with that identifier was already present |
| `PersistenceUnavailable` | no additional data | The store could not determine or perform the insert |

`ValidationIssue` contains a field identity and a closed reason:
`missing`, `invalid_format`, `too_long`, `control_character`, or
`payload_too_large`. Issues are ordered by request field order (`job_id`,
`task`, `payload`) and then by the reason order in this sentence. At most one
issue is returned for each field.

## Nominal behavior

### Behavior illustration: nominal execution trace

Given:

```text
request =
  job_id: "job-1042"
  task: "rebuild-search-index"
  payload: bytes representing {"tenant":"north"}

jobs store initially has no record with job_id "job-1042"
```

the observable trace is:

```text
1. validate job_id
2. validate task
3. validate payload
4. construct Job using the validated values
5. call jobs.insert_if_absent(Job)
6. observe store outcome inserted
7. return Created(Job)
```

The final store contains exactly the returned `Job` at `job-1042`. The handler
does not mutate the request, synthesize additional fields, or perform another
observable effect.

## Domain failures and faults

### Behavior illustration: validation outcomes

| Condition | Result | Store calls | Persistent change |
| --- | --- | ---: | --- |
| all fields satisfy their constraints | continue to insertion | 1 | determined by store outcome |
| `job_id` is empty or malformed | `Invalid(job_id, missing or invalid_format)` | 0 | none |
| `task` is empty | `Invalid(task, missing)` | 0 | none |
| `task` exceeds 80 scalar values | `Invalid(task, too_long)` | 0 | none |
| `task` contains a control character | `Invalid(task, control_character)` | 0 | none |
| `payload` exceeds 4096 bytes | `Invalid(payload, payload_too_large)` | 0 | none |
| multiple fields are invalid | one ordered `Invalid` issue list | 0 | none |

### Behavior illustration: persistence outcomes

| Store outcome | Public result | Required postcondition |
| --- | --- | --- |
| `inserted` | `Created(Job)` | the store contains exactly that job |
| `duplicate` | `AlreadyExists(job_id)` | the pre-existing record is unchanged |
| `unavailable_before_commit` | `PersistenceUnavailable` | no record was inserted |

The capability contract deliberately excludes an ambiguous
“commit status unknown” outcome from this slice. A production adapter that can
observe ambiguous commits must resolve them below this boundary or expose a
future, explicitly modeled result.

Validation failures, duplicates, and known store unavailability are domain
outcomes. They are not unchecked exceptions.

Resource exhaustion, a violated capability postcondition, corrupt compiler or
runtime state, and other failures not listed above are language/runtime faults.
This use case requires faults to be distinguishable from domain results but does
not decide whether or how such faults can be caught.

## Observable state and ordered effects

The complete observable behavior is:

```text
(initial jobs state, request, supplied store outcomes)
  -> (final jobs state, returned result, ordered store-call trace)
```

The following ordering is required:

1. Validation completes before any store call.
2. Invalid input produces no store call.
3. Valid input produces exactly one `insert_if_absent` call.
4. The call contains values equal to the validated request values.
5. The handler observes the store outcome before returning.
6. No effect occurs after the result is selected.

There is no concurrency, cancellation, retry, timeout, buffering, recursion, or
unbounded collection growth inside this use case. Input bounds limit the request
and diagnostic list. The handler processes one request at a time; parallel
invocation by a host is outside the logical trace.

## Schema, serialization, and compatibility

The structured shapes and field identities are public contracts. Their concrete
wire encoding is outside UC-001. Tests must nevertheless use a fixed,
language-independent fixture representation so each baseline receives the same
values, including empty payloads and non-ASCII valid task text.

This record describes version 1 of `CreateJobRequest`, `Job`, and
`CreateJobResult`. Evolution of those public and stored shapes is UC-003.

## Representative agent change

Task contract:

> Implement `create_job` from the supplied public data contracts, job-store
> capability contract, and behavioral tests. Do not change public shapes, test
> oracles, capability authority, or effect ordering.

A repair variant starts from a handler that calls the store before validating
`payload`; the agent must find and correct the observable extra effect.

The task is complete only when the agent supplies:

- the changed canonical source or baseline source;
- a passing type/build check and behavior test suite;
- the ordered capability traces for every result variant;
- a machine-readable list of public contracts, errors, and effects used by the
  handler;
- a source and semantic diff suitable for human review; and
- a statement that no additional authority, nondeterministic input, or
  observable effect was introduced.

## Human review and operational evidence

A reviewer must be able to verify without reconstructing model reasoning:

- the full public request and result contracts;
- every validation bound and issue order;
- the handler's datastore authority;
- that invalid input is effect-free;
- the mapping between store and public outcomes;
- the ordered effect trace;
- the final persistent state for each case; and
- the exact source revision and test revision that produced the evidence.

Operational telemetry is excluded from the handler, but the test harness must
report per-case result, store-call count, final store state, and elapsed
measurement separately from functional correctness.

## Baseline implementation and tools

Equivalent behavior will be implemented in:

- Rust using stable Rust, Cargo, rustfmt, Clippy, rust-analyzer, and the standard
  test harness;
- Go using the stable Go toolchain, `gofmt`, `go vet`, `gopls`, and `go test`;
- Python using a supported CPython release, an accepted static type checker,
  formatter/linter, and `pytest`; and
- TypeScript using a supported Node.js release, TypeScript in strict mode, the
  language service, formatter/linter, and a conventional test runner.

Each baseline may use idiomatic representations, but must expose the same
logical contracts and pass the same language-independent fixtures and trace
oracle. Tool and dependency versions, task wording, starting revision,
available tools, retry policy, environment, and measurement procedure are
frozen before agent runs. No baseline is restricted to raw text editing.

## Measurable success criteria

UC-001 is satisfied when:

1. every valid fixture returns the expected `Created` result and final state;
2. every invalid fixture returns the complete ordered issue list with zero store
   calls;
3. duplicate and unavailable fixtures return the specified result with the
   specified unchanged state;
4. all implementations emit the same logical ordered trace for each fixture;
5. static analysis exposes the complete public result set and declared store
   authority;
6. the implementation task stays within the frozen context, repair, regression,
   and runtime envelopes in the derived requirements; and
7. an independent reviewer can match every changed behavior to source and
   machine-produced evidence.

## Derived requirements

- APP-001, APP-002, APP-003
- LANG-001, LANG-002, LANG-003, LANG-004
- PROTO-001, PROTO-002, PROTO-005
- NFR-001, NFR-002, NFR-003, NFR-004, NFR-005

See [the initial reference-slice requirements](../requirements/reference-slice.md).

## Explicit exclusions and unresolved questions

Excluded from this slice:

- transport decoding and response encoding;
- authentication, authorization, rate limiting, and tenancy;
- generated identifiers, timestamps, deadlines, scheduling, and job execution;
- retries or ambiguous datastore commits;
- production database adapters and migrations;
- logging, metrics, and tracing;
- concurrency and cancellation; and
- a choice of AIL syntax, memory strategy, runtime, or compiler implementation.

Accepted decisions:

- The job service is representative enough for the first reference workload.
- The input limits are accepted, including measuring task length in
  Unicode scalar values.
- Shared test cases use the JSON format defined in
  [the job-service fixture rules](../benchmarks/job-service-fixtures.md).
- The first workload keeps the three storage outcomes already defined:
  inserted, duplicate, and unavailable before commit. Ambiguous commits remain
  out of scope.
- Exact tool versions, the container image, and the reference host are recorded
  in a locked run manifest before benchmark work starts.
