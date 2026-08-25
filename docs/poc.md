# Proof of concept: when does compiler output help an agent?

Two tests. Same harness, same model, same publish gate.

Each test hands two agents the same broken AIL workspace and the same success
gate, `ailc publish`. One arm sees the compiler's output. The other arm gets
`PASS` or `FAIL` and nothing else. The measure that decides the outcome is
retries: failed gate calls before the publish that passed. Seven trials per arm
per test. Both tests ran on 2026-08-24 against the same `ailc` binary.

## Test 1: a rule you cannot see by reading the files

Fixture `cancel-dispatch`, in [RESULTS.md](../poc/compiler-convergence/RESULTS.md).
One of its six faults is an architecture complexity budget. `transport.dispatch`
measures control-flow complexity 7 against a policy budget of 4. The budget is
written in `architecture.json`. The 7 is a number the compiler computes and no
file states.

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

Fixture `label-batch`, in
[RESULTS-label-batch.md](../poc/compiler-convergence/RESULTS-label-batch.md).
Seven faults, all of them type, capability, and effect errors visible at their
own site in six files of about ninety lines. This workspace has no
`architecture.json`.

The blind arm was faster: retries [1, 1, 1, 1, 1, 1, 1], median 1. The compiler
arm ran [1, 2, 2, 2, 2, 2, 2], median 2.

Both arms wrote the same repair either way: the same three files, `batch.ail`,
`classify.ail`, and `report.ail`, with `types.ail` left byte-identical to the
fixture. `batch.ail` and `classify.ail` came out byte-identical across every
trial in both arms.

The extra compiler-arm retry is the opening check. Six of seven compiler-arm
trials called `check` on the broken workspace to read the findings, then wrote
the three files, then published. The one trial that skipped that probe tied the
blind arm at 1 retry.

One caveat on this fixture. The trials ran concurrently, so a compiler-arm state
file holding full compiler output sat on the same machine while the blind arms
worked. The prompts forbade reading it and nothing enforced that. Treat the
blind arm's 1 retry as an upper bound that a staggered run should confirm.

## What it shows

The compiler helps when the rule is hidden. It does not help when the bug is
already on the page.

## What it does not show

This is not a proof at scale. One model, two fixtures. Both workspaces are
small enough to read in full, and neither gate executes the program.

## Full results

- [poc/compiler-convergence/RESULTS.md](../poc/compiler-convergence/RESULTS.md)
  — fixture `cancel-dispatch`, including the full tables and threats to
  validity.
- [poc/compiler-convergence/RESULTS-label-batch.md](../poc/compiler-convergence/RESULTS-label-batch.md)
  — fixture `label-batch`, including the secondary condition where both arms
  published on the first gate call.
