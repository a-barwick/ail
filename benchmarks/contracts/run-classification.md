# Benchmark run and repair classifications

Status: **Frozen by M2**

These definitions apply to all baseline and later AIL benchmark runs.

## Validation attempt

A validation attempt is one agent-requested compiler, static-analysis, build, or
behavior-test action whose result can establish that the task is complete or
incomplete. Multiple diagnostics emitted by one action remain one validation
attempt.

Read-only inspection and formatting without a correctness check are not
validation attempts.

## Edit

An edit occurs when the agent changes durable workspace content that can affect
the task result. A formatter change caused by the agent's requested edit is part
of that edit. Reverting or repairing prior content is another edit.

Changes made before the task starts, by benchmark packaging, or by the harness
are not agent edits.

## Repair cycle

A repair cycle is counted when:

1. the agent has made at least one edit since the preceding validation attempt;
2. the next validation attempt runs; and
3. that attempt shows the task is still incomplete.

One failing attempt counts one repair cycle even when it reports several
causes. Further checks without another edit do not add repair cycles. After the
agent edits again, the next incomplete validation begins another repair cycle.
Pre-edit checks establish the starting state and do not count as repairs.

## Successful run

A run is successful only when it finishes within its locked limits and all
locked public and hidden correctness checks pass for the final source revision.
The response, final state, ordered store calls, seeded consumers, UC-001
regressions, compatibility behavior, permissions, and completion evidence must
all pass. A low-context or fast incomplete run is not successful.

## Failed run

A run is failed when it terminates without satisfying the complete success
definition. Stable harness classifications distinguish at least:

- `nonzero_exit`;
- `malformed_result`;
- `result_schema_invalid`;
- `manifest_mismatch`;
- `missing_case`;
- `unexpected_case`;
- `response_mismatch`;
- `final_state_mismatch`;
- `store_calls_mismatch`;
- `seeded_regression`;
- `permission_violation`; and
- `incomplete_evidence`.

The first deterministic classification in corpus order is primary. All observed
classifications remain available in raw evidence.

## Timed-out run

A run is timed out when the agent or functional runner exceeds the applicable
locked wall-time limit and the harness terminates it. Timeout is a failed-run
classification, recorded separately from a normal non-zero exit. Partial output
cannot make a timed-out run successful.

## Manifest-start gate

A run has not started until the harness has:

1. verified the external manifest-lock digest;
2. validated every required run-manifest field;
3. verified every locked artifact digest; and
4. confirmed the selected runner and visibility are allowed.

An incomplete manifest, changed manifest, or changed locked artifact is a
configuration rejection, not a failed agent run. The implementation subprocess
must not start.
