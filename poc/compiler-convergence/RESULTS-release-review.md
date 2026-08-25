# Fixture 3 result: release review

The blind control arm won the declared win condition in all seven trials.

- AIL retries to a passing publish, trials 1–7:
  **`[2, 2, 2, 2, 2, 2, 2]`**
- Control retries to a passing publish, trials 1–7:
  **`[1, 1, 1, 1, 1, 1, 1]`**

The medians are 2 against 1 and the worst cases are 2 against 1. Across all 49
AIL/control pairings, control needed fewer retries in 49, more in 0, and tied in
0. No trial failed to converge.

This result belongs only to this fixture. It is not pooled with either earlier
fixture.

## Protocol

- Base: `origin/main` at `7981e32`, after the blind-ledger sealing fix.
- Fixture: `release-review`, a 35-module AIL workspace.
- Runs roots: `ab64-release-review-7981e32/` for trial 1 and
  `ab64-release-review-n7-7981e32/` for trials 2–7.
- Model: all fourteen arms used `gpt-5.6-sol-high`.
- Gate: every arm used the same `ailc publish`; control saw only `PASS` or
  `FAIL`, while the AIL arm saw the compiler's JSON-backed findings.
- Trial 1 control finished before trial 1 AIL started. For trials 2–7, all six
  controls finished before any of the six AIL arms started.
- Every control gate call was sealed while its arm ran. The operator rebuilt
  withheld findings only after the relevant controls and AIL arms had finished.
- Every command log contains only `brief`, `files`, `read`, `write`, `check`,
  and `publish`. No log contains `ailc`, `cargo`, or a private compiler run.
  All fourteen trials are valid.
- The workspace has no `architecture.json`. The harness did not apply
  `expected.type` or any other finding.

The broken workspace already satisfied the task specification. Its active
pipeline passed a `ReviewSignal` to a function declared over the distinct
`StoredSignal` type, then passed that result to a function requiring
`ReviewSignal`. A separate presentation module requested `AuditSink` authority
from an empty capability environment. Every arm repaired the same two source
files and left immutable `contracts.ail` byte-identical.

## Measures

| measure | AIL trials 1–7 | control trials 1–7 |
| --- | --- | --- |
| retries to a passing publish | `[2, 2, 2, 2, 2, 2, 2]` | **`[1, 1, 1, 1, 1, 1, 1]`** |
| failed gate calls before pass | `[1, 1, 1, 1, 1, 1, 1]` | `[0, 0, 0, 0, 0, 0, 0]` |
| gate calls to pass | `[3, 3, 3, 3, 3, 3, 3]` | `[2, 2, 2, 2, 2, 2, 2]` |
| rebreaks | `[0, 0, 0, 0, 0, 0, 0]` | `[0, 0, 0, 0, 0, 0, 0]` |
| fix cycles | `[0, 0, 0, 0, 0, 0, 0]` | `[0, 0, 0, 0, 0, 0, 0]` |
| protocol tokens | `[1646, 1648, 1648, 1648, 1720, 1755, 1648]` | `[2506, 6987, 2541, 7941, 2541, 7672, 3347]` |
| reads | `[4, 4, 4, 4, 6, 7, 4]` | `[34, 34, 35, 45, 35, 39, 36]` |
| source edits | `[2, 2, 2, 2, 2, 2, 2]` | `[2, 2, 2, 2, 2, 2, 2]` |

Every control arm read broadly, repaired both faults before its first check,
received a clean check, then published. Every compiler arm checked the broken
workspace first, received two type findings and one capability finding,
repaired both files, received a clean check, then published. Under the
predeclared metric, that diagnostic check made every compiler arm lose 2
retries to 1.

Tokens are secondary and do not decide the winner. Their medians are 1,648 for
AIL and 3,347 for control. Both arms had zero rebreaks and zero fix cycles in
all seven trials.

## Audit

`self-test` passes 34/34 checks. Trial 1's audit passes 16/16. The trials 2–7
audit passes 126/126. Those audits cover arm order, sealing every control gate
call, rebuilding findings only after controls finished, immutable source, task
specification, canonical published source, no architecture findings, and no
revision written by a check or failed publish.

The committed run ledgers and generated measures are under
`ab64-release-review-7981e32/` and
`ab64-release-review-n7-7981e32/`.
