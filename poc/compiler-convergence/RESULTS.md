# Results: seven trials per arm on the merged tree

Run on 2026-08-24 against `main` at `5fd65db`, which carries the structured
findings from [PR #5](https://github.com/a-barwick/ail/pull/5): located
findings, a `requirement` line per error stating a constraint and never an edit,
the `AIL.MODULE.INACCESSIBLE_DECLARATION` module fix, and `ailc check --json`.
Fourteen agent runs, seven per arm, same model, same broken fixture, same
`ailc publish` gate, same reference material. Every ledger was built from
`ailc check --json`.

Measures in the order that decides the outcome:

1. **Retries to a passing publish** is the win condition.
2. **Rebreak** is whether the change was actually safe.
3. **Tokens** are extra and decide nothing on their own.

## 1. Retries to a passing publish (the win condition)

```
ail      n=7  retries [2, 2, 2, 2, 3, 3, 3]        median 2   worst 3   total 17
control  n=7  retries [2, 2, 3, 12, 13, 13, 14]    median 12  worst 14  total 59

paired comparisons (49, every ail trial against every control trial):
  control needed more retries in 32, fewer in 6, equal in 11
```

Both arms converged in all seven trials; neither hit the 40-call limit.

**The `ail` arm wins the win condition: median 2 retries against 12, worst case
3 against 14.** It is not a clean sweep. Three control trials landed at 2, 2,
and 3, matching or beating every `ail` trial, so an agent that gets the whole
repair right before its first gate call does not need the compiler. The other
four control trials needed 12 to 14.

The distribution is bimodal, and that is the finding. A blind arm either guesses
the whole repair correctly on the first try, or it starts bisecting, and
bisecting costs an order of magnitude more. The compiler arm has no such split:
every trial took 2 or 3 retries.

## 2. Rebreak: did a later edit break an earlier check

```
ail      rebreak events 0, across 0 of 7 trials
control  rebreak events 6, across 3 of 7 trials
         fix cycles: ail 3 events in 3 trials, control 18 events in 5 trials
```

**The `ail` arm wins this outright again: zero rebreaks in seven trials.** No
`ail` trial ever saw a diagnostic reappear after fixing it. Note the median
hides this, because four of seven control trials had none; the totals and the
per-trial column are the honest view.

The mechanism repeated exactly what the pre-#5 run showed. Four control trials
bisected by gutting modules, and all four concluded that `contracts.ail` needs
an executable function because a types-only `contracts.ail` failed while a
gutted workspace passed. That conclusion is false. Five `ail` trials and three
control trials published with `contracts.ail` byte-identical to the fixture.

Two of those four shipped the invented fix as dead code:

| trial | added to `contracts.ail` | called anywhere |
| --- | --- | --- |
| `control-t6` | `fn keep(value: Text) -> Text` | no |
| `control-t7` | `fn is_blank`, `fn has_control` | no |

The other two put a real decision helper there, which is a legitimate design
choice, and so did `ail` and `ail-t2`. The distinction matters: moving the
store-outcome match into the contract group is a defensible shape, while adding
`fn keep` to satisfy a coverage rule that does not exist is a false belief
written into published source.

## 3. Tokens (extra)

```
ail      median 8490   range 8043 to 8857
control  median 11430  range 8725 to 11904
```

Median about 35 percent lower for `ail`, and the ranges barely overlap: the most
expensive `ail` trial cost less than the cheapest control trial but one. Tokens
agree with the other two measures rather than deciding anything. The structured
findings did not make the `ail` arm more expensive: its median rose from 8305
pre-#5 to 8490, about 2 percent, while carrying locations, snippets, and a
requirement per finding.

Protocol tokens exclude an agent's private reasoning tokens, which this
environment does not expose. They are a lower bound, measured identically for
both arms.

## Full table

```
measure                                             ail      ail-t2      ail-t3      ail-t4      ail-t5      ail-t6      ail-t7     control  control-t2  control-t3  control-t4  control-t5  control-t6  control-t7
1 RETRIES to passing publish                          2           3           2           2           2           3           3           2           2          12          13           3          13          14
  converged                                        True        True        True        True        True        True        True        True        True        True        True        True        True        True
2 REBREAKS (fixed, then broken again)                 0           0           0           0           0           0           0           0           0           2           2           0           0           2
  fix cycles (new breakage after an edit)             0           1           0           0           0           1           1           0           0           4           5           1           5           3
  attempts failing the task specification             0           1           0           0           0           1           1           0           0           9          10           1          11          11
3 TOKENS total (protocol, extra)                   8490        8658        8449        8449        8830        8857        8043        8725        8859       11430       11725        9340       11850       11904
- source edits                                        5           5           4           4           4           4           4           4           4          24          23           7          24          23
- god-method rejections                               1           1           1           1           1           1           1           1           1           3           3           1           7           2
- worst dispatch control-flow complexity              7           7           7           7           7           7           7           7           7           7           7           7           7           7
- ledger built from                                json        json        json        json        json        json        json        json        json        json        json        json        json        json
```

## The magnitude is unstable; the direction was not

The control arm's information is identical before and after #5: `PASS` or `FAIL`
and nothing else. Its median retries still moved from 4 in the pre-#5 run to 12
here. That is run-to-run variance in blind search across two samples of seven,
not an effect of the compiler change, and it is the strongest caution in this
report. **Do not quote the 6x median ratio as a stable effect size.** Across
both runs of the experiment the `ail` arm's median was 2 retries with a worst
case of 4, and the control arm was worse at the median both times, but how much
worse swung by a factor of three under an unchanged condition.

## God methods

All fourteen trials submitted a `transport.dispatch` measuring control-flow
complexity 7 against the policy budget of 4, and the architecture checker denied
it in both arms. No trial published an over-budget dispatch, so the gate
prevented the god method regardless of visibility. Visibility changed the cost
of learning why: one denial and one edit for every `ail` trial, reading
`facts.candidate_cfc=7` and the requirement `control-flow complexity must stay
at most 4 and minimal review context at most 12; the candidate measured 7 and
6`, against up to seven denials of the same rule for `control-t6`.

## What actually separated the arms

Only one of the six faults was hard to see by reading. Every trial in both arms
found the missing import, the `Int` in a `Text` field, the unknown capability,
and the non-exhaustive match from the source and the reference material. The
`ail` ledgers reached a median of one diagnostic class:
`AIL.ARCH.HOTSPOT_GROWTH`. That is where the compiler earned its keep, and a
control agent could substitute arithmetic for it, since `architecture.json`
states the budget and `specs/architecture.md` gives the metric as `E - N + 2`.
Three control trials did that arithmetic before their first gate call and
converged as fast as the `ail` arm. Four did not, and paid 12 to 14 retries.

## Threats to validity

1. Seven trials per arm, one fixture, one model, and a control distribution
   whose median moved 3x between two runs of an unchanged condition. Treat the
   direction as repeatable and the magnitude as not.
2. The control arm can read the policy file and the metric definition, so most
   compiler facts in this fixture are re-derivable by hand. That is realistic
   and it caps how large the gap can be.
3. The fixture is five modules and six faults, small enough to hold in context
   entirely. The thesis is about scale; this does not test scale.
4. Both arms used the same strong model. The compiler's facts should matter more
   for a weaker or cheaper model. Not tested.
5. The gate is `ailc publish` plus a task specification matched against
   normalized source text. There is no behavior test in this language fragment,
   so an arm can satisfy the letter of the specification while losing behavior.
6. Both arms are told which task-specification rules fail. That is symmetric,
   but it hands both arms one of the six faults with no compiler involvement,
   and every bisecting control trial used the specification list as a free
   oracle while probing.
7. Nothing physically prevented a control agent from reconstructing the
   workspace from `runs/<arm>/state.json` and compiling it itself. The brief
   forbids it and the committed command logs show none did, but this is a trust
   assumption, not an enforcement.

## What would make this a real measurement

- Enough trials to put error bars on a bimodal distribution, and a weaker model
  as a second condition.
- A fixture too large to hold in context, where the agent must locate the
  relevant module before it can reason about it.
- A fault whose fix is not derivable from readable policy.
- Behavior verification in the gate, so specification text cannot stand in for
  working code.

## Audit of all fourteen final workspaces

`architecture.json` byte-identical to the fixture in all fourteen. Zero
task-specification violations in every final source. No comments used to satisfy
a requirement in any final source. No arm emptied or weakened policy, and the
harness never applied a checker fact to source.

## Earlier runs kept for the record

- `runs-pre-pr5/`, `report-pre-pr5/`: seven trials per arm against the one-line
  diagnostic format, before #5 merged. Superseded by this run; kept because the
  `contracts.ail` phantom appears in both, under two different compilers.
- `runs-v1-gate-defect/`: the first two trials, invalidated by a harness bug
  where `check` reported `FAIL` even when `ailc check` passed. The control arm
  could not see through that label and spent 18 gate calls on a workspace that
  had been correct since its second attempt.

## Harness defects found and fixed

1. The gate reported `FAIL` for a passing `check`. Fixed; `check` now reports
   its own outcome in both arms.
2. The ledger dropped `AIL.ARCH.ANALYSIS_INCOMPLETE` lines, which carry no rule
   field. Unrecognized output is now recorded rather than vanishing.
3. The task specification required only that a function named `summarize` exist,
   so an arm could hollow it out. It now pins the four outcome labels.
4. AIL has `//` and `/* */` comments and the specification matched raw source,
   so commenting a requirement out satisfied it. Normalization strips comments
   and the self-test asserts it.
5. The line-format ledger parser could not read #5's findings and would have
   recorded zero diagnostics for every attempt. The gate now reads
   `ailc check --json` and falls back to text when the compiler has no JSON
   view, recording which it used.
