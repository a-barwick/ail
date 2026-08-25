# Proof of concept: when does compiler output help an agent?

Three tests. Same harness, same model, same publish gate.

Each test hands two agents the same broken AIL workspace and the same success
gate, `ailc publish`. One arm sees the compiler's output. The other arm gets
`PASS` or `FAIL` and nothing else. The measure that decides the outcome is
retries: gate calls spent before the publish that passed, whether each one
failed or passed. The first two tests ran on 2026-08-24 against the same `ailc`
binary. The third ran later, after the arms were staggered and the blind ledger
was sealed.

The three tests use different fixtures and are separate experiments. Their
numbers do not combine.

## Test 1: a rule you cannot see by reading the files

Fixture `cancel-dispatch`, seven trials per arm, in
[RESULTS.md](../poc/compiler-convergence/RESULTS.md). One of its six faults is
an architecture complexity budget. `transport.dispatch` measures control-flow
complexity 7 against a policy budget of 4. The budget is written in
`architecture.json`. The 7 is a number the compiler computes, and no file the
arms could read states it.

The compiler arm needed about two tries: retries [1, 2, 2, 2, 3, 3, 3], median
2, worst 3.

The blind arm needed about eleven: retries [1, 8, 10, 11, 11, 13, 14], median
11, worst 14.

The blind arm also kept breaking its own fixes. Counting diagnostics that were
fixed and then came back: 0 events in the compiler arm, 9 in the blind arm.

One blind trial finished in 1 retry, matching the best compiler-arm trial and
beating the other six. It read the budget out of `architecture.json` and did the
metric arithmetic before spending a gate call. The metric definition is written
down, so a careful reader can recompute what the compiler measures. One blind
trial in seven did. The other six needed 8 to 14 retries.

## Test 2: type mistakes sitting in the source

Fixture `label-batch`, seven trials per arm, in
[RESULTS-label-batch.md](../poc/compiler-convergence/RESULTS-label-batch.md).
Seven faults, all of them type, capability, and effect errors visible at their
own site in six files of about ninety lines. This workspace has no
`architecture.json`.

The blind arm was faster: retries [1, 1, 1, 1, 1, 1, 1], median 1. The compiler
arm ran [1, 2, 2, 2, 2, 2, 2], median 2.

Both arms wrote the same repair either way. Every trial in both arms edited
exactly the same three files, `batch.ail`, `classify.ail`, and `report.ail`, and
created no new module. `batch.ail` and `classify.ail` came out byte-identical
across every trial in both arms. The only variation anywhere is what an arm
named the match bindings in `report.ail`.

The extra compiler-arm retry is the opening check. Six of seven compiler-arm
trials ran `check` on the broken workspace to read its thirteen findings, wrote
the three files, ran `check` again to confirm the workspace was clean, then
published. The blind arm ran the same trajectory without the opening probe. The
one compiler-arm trial that skipped the probe tied the blind arm at 1 retry.

A second condition, three trials per arm, told both arms that gate calls are
expensive. Every trial in both arms wrote the three files and published on its
first gate call, 0 retries, and the compiler arm never looked at a diagnostic at
all. So the finding here is not that compiler output costs a retry. On this
fault set the probe bought nothing, and an arm told to economize skipped it.

One caveat on this fixture. The trials ran concurrently, so a compiler-arm state
file holding full compiler output sat on the same machine while the blind arms
worked. The prompts forbade reading it and nothing enforced that. Treat the
blind arm's 1 retry as an upper bound that a staggered run should confirm. The
number cannot settle it either way, because 1 retry is the floor for a
check-then-publish strategy, so a peeking arm and an honest arm would report the
same number.

## Test 3: the same kind of mistake, in a bigger folder

Fixture `release-review`, seven trials per arm, in
[RESULTS-release-review.md](../poc/compiler-convergence/RESULTS-release-review.md).
Thirty-five modules. Both arms used `gpt-5.6-sol-high`. The arms were staggered
and sealed: the blind trials finished first, and the blind arm never saw
compiler findings while it ran.

The compiler arm needed two tries: retries [2, 2, 2, 2, 2, 2, 2], median 2.

The blind arm needed one: retries [1, 1, 1, 1, 1, 1, 1], median 1. Across all
49 pairings, the blind arm needed fewer retries in 49. Neither arm rebroke a
fix.

The extra compiler-arm retry is the opening check, same as fixture 2. Every
compiler trial checked the broken workspace first, repaired two files, checked
again, then published. Every blind trial repaired both files before its first
check.

Size showed up as reads, about 34 against 4, and as tokens, where the compiler
arm used fewer. That is not the scale test. This is fixture 2 with a bigger
folder.

Every command log used only the harness commands. No trial ran a private
compiler.

## What it shows

The compiler helps when the rule is hidden. It does not help when the bug is
already on the page. A bigger folder of the same kind of bug does not change
that.

## What it does not show

This is not a proof at scale. One model, three fixtures. Test 3 is fixture 2
with a bigger folder, not a scale test. None of the gates execute the program.

## Full results

- [poc/compiler-convergence/RESULTS.md](../poc/compiler-convergence/RESULTS.md)
  — fixture `cancel-dispatch`, including the full tables and threats to
  validity.
- [poc/compiler-convergence/RESULTS-label-batch.md](../poc/compiler-convergence/RESULTS-label-batch.md)
  — fixture `label-batch`, including the full tables, the secondary condition,
  and threats to validity.
- [poc/compiler-convergence/RESULTS-release-review.md](../poc/compiler-convergence/RESULTS-release-review.md)
  — fixture `release-review`, including the full tables and the sealed-run
  protocol.
