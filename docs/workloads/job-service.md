# Job-service workload

The accepted reference workload is a transport-independent job service with an
explicit jobs-store capability. Its public JSON fixtures and schema are under
[benchmarks/](../../benchmarks/README.md).

## Create a job

`create_job` validates a request before effects, performs at most one conditional
store insert, and returns a closed result. Invalid input makes zero store calls;
duplicate and unavailable outcomes preserve the specified state. The fixture
corpus is the executable behavior authority.

## Evolve priority

The priority change adds a required closed priority to version-two request and
stored-job shapes. Version-one requests and stored jobs adapt explicitly to
`Normal`; version-one responses omit priority. The compiler must report exact
known edits, explicit review items, and unchecked boundaries, then validate the
complete candidate atomically.

The corresponding compiler contracts are [runtime](../../specs/runtime.md) and
[evolution](../../specs/evolution.md). This workload does not define AIL syntax,
production migrations, unknown-client rollout, or a completed agent comparison.
