# Fixture 3 result: release review

The blind control arm won the declared win condition. It needed **1 retry to a
passing publish**; the compiler arm needed **2**.

This is one trial per arm. It is evidence for this fixture, not a pooled result
with either earlier fixture.

## Protocol

- Base: `origin/main` at `7981e32`, after the blind-ledger sealing fix.
- Fixture: `release-review`, a 35-module AIL workspace.
- Fresh runs root: `ab64-release-review-7981e32/`.
- Arm order: control finished before the compiler arm started.
- Model: both arms used `gpt-5.6-sol-high`.
- Gate: both arms used the same `ailc publish`; control saw only `PASS` or
  `FAIL`, while the AIL arm saw the compiler's JSON-backed findings.
- Every control gate call was sealed while the arm ran. Before the compiler arm
  started, the completed control state contained no `compiler_output`,
  `diagnostics`, `rebuilt_at`, or `AIL.` code. The operator rebuilt its
  findings only after both arms finished.
- The workspace has no `architecture.json`. The harness did not apply
  `expected.type` or any other finding.

The broken workspace already satisfied the task specification. Its active
pipeline passed a `ReviewSignal` to a function declared over the distinct
`StoredSignal` type, then passed that result to a function requiring
`ReviewSignal`. A separate presentation module requested `AuditSink` authority
from an empty capability environment. The repair changed two source files and
left immutable `contracts.ail` byte-identical.

## Measures

| measure | AIL | control |
| --- | ---: | ---: |
| retries to a passing publish | 2 | **1** |
| failed gate calls before pass | 1 | 0 |
| gate calls to pass | 3 | 2 |
| rebreaks | 0 | 0 |
| fix cycles | 0 | 0 |
| protocol tokens | 1,646 | 2,506 |
| reads | 4 | 34 |
| source edits | 2 | 2 |
| distinct findings reached | 3 | 0 |

The control arm read broadly, repaired both faults before its first check,
received a clean check, then published. The compiler arm checked the broken
workspace first, received two type findings and one capability finding,
repaired both files, received a clean check, then published. Under the
predeclared metric, that extra diagnostic check made the compiler arm lose
2 retries to 1.

Tokens do not reverse the result: they are secondary. The compiler arm used 860
fewer protocol tokens because it read 4 files while control read 34, but control
still won retries to a passing publish. Both arms had zero rebreaks and zero fix
cycles.

## Audit

`self-test` passes 34/34 checks. The committed run audit passes 16/16 checks,
including arm order, sealing every control gate call, rebuilding findings only
after control finished, immutable source, task specification, canonical
published source, no architecture findings, and no revision written by a check
or failed publish.

The run ledgers and generated measures are under
`ab64-release-review-7981e32/`.
