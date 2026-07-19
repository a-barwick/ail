# UC-001 implementation task

Task contract version: 1

## Task text

Implement `create_job` in the supplied version-one job-service workspace.

Use the supplied public request, result, and job-store contracts and the
available normal language tools. Validate the complete request before invoking
the store. Invalid input must return every applicable issue in the declared
field and reason order, make no store call, and leave stored state unchanged.
Valid input must make exactly one `insert_if_absent` call with the validated
values and map inserted, duplicate, and unavailable-before-commit outcomes to
the declared closed result and postcondition.

Do not change public shapes, fixture oracles, capability authority, effect
ordering, task limits, tool configuration, or benchmark files. Do not add
transport, networking, a production datastore, retries, clocks, randomness,
telemetry, concurrency, or deployment behavior.

Use the supplied tests and any normal compiler, static-analysis,
language-server, formatter, search, and build tools available in the locked run
manifest. The task is complete only when every public and hidden correctness
check passes for the final source revision and the required completion evidence
names that same revision.
