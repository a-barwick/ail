# Results: seven trials per arm on current main

Run on 2026-08-24 against `main` at `36636f8`, which carries #5's structured
findings (squash `5fd65db`) plus the repair that renumbered the findings record
to ADR 0016 and unpinned the old `AIL.ARCH.BOUNDARY` header assertion. Fourteen
agent runs, seven per arm, same model, same broken fixture, same `ailc publish`
gate, same reference material. Every ledger was built from `ailc check --json`.

Measures in the order that decides the outcome:

1. **Retries to a passing publish** is the win condition.
2. **Rebreak** is whether the change was actually safe.
3. **Tokens** are extra and decide nothing on their own.

## 1. Retries to a passing publish (the win condition)

```
ail      n=7  retries [1, 2, 2, 2, 3, 3, 3]        median 2   worst 3   total 16
control  n=7  retries [1, 8, 10, 11, 11, 13, 14]   median 11  worst 14  total 68

paired comparisons (49, every ail trial against every control trial):
  control needed more retries in 42, fewer in 6, equal in 1
```

Both arms converged in all seven trials; neither hit the 40-call limit.

**The `ail` arm wins the win condition: median 2 retries against 11, worst case
3 against 14.** It is not a clean sweep. `control-t4` needed 1 retry, beating
every `ail` trial, because it worked out the whole repair, including the
complexity budget arithmetic, before spending a gate call. That is the only
control trial that did so, and it is why the paired comparison is 42-6-1 rather
than 49-0-0.

Every other control trial fell into bisection and needed 8 to 14 retries.

## 2. Rebreak: did a later edit break an earlier check

```
ail      rebreak events 0, across 0 of 7 trials
control  rebreak events 9, across 6 of 7 trials
         fix cycles: ail 3 events in 3 trials, control 23 events in 6 trials
```

**The `ail` arm wins this outright: zero rebreaks in seven trials.** No `ail`
trial ever saw a diagnostic reappear after fixing it. Six of seven control
trials did, nine times in total. The only control trial with no rebreak is
`control-t4`, the one that never had to search.

The mechanism repeated for a third time, now under three different compiler
versions. Six control trials bisected by gutting modules, and every one of them
concluded that `contracts.ail` needs an executable function because a types-only
`contracts.ail` failed while a gutted workspace passed. That conclusion is
false. All seven `ail` trials published with `contracts.ail` byte-identical to
the fixture.

Five control trials shipped the invented fix, four of them as dead code:

| trial | added to `contracts.ail` | called anywhere |
| --- | --- | --- |
| `control` | `fn keep` | no |
| `control-t3` | `fn request_value` | no |
| `control-t5` | `fn keep` | no |
| `control-t6` | `fn keep` | no |
| `control-t7` | `fn keep` | no |
| `control-t2` | `fn decide` | yes |
| `control-t4` | `fn cancel_decision` | yes |

`fn keep(value: Text) -> Text { value }` in four published workspaces is the
cost of blind search made concrete: a function that exists only because an agent
could not see why the compiler said no.

## 3. Tokens (extra)

```
ail      median 8600   range 8405 to 8922
control  median 11124  range 8552 to 12226
```

Median about 23 percent lower for `ail`. The one cheap control trial (8552) is
`control-t4` again. Tokens agree with the other two measures rather than
deciding anything.

## Full table

```
measure                                             ail      ail-t2      ail-t3      ail-t4      ail-t5      ail-t6      ail-t7     control  control-t2  control-t3  control-t4  control-t5  control-t6  control-t7
1 RETRIES to passing publish                          2           3           2           1           3           3           2          13          11          14           1          10           8          11
2 REBREAKS (fixed, then broken again)                 0           0           0           0           0           0           0           1           1           3           0           2           1           1
  fix cycles (new breakage after an edit)             0           1           0           0           1           1           0           4           4           5           0           4           3           3
  attempts failing the task specification             0           1           0           0           1           1           0          11           9          12           0           7           5           8
3 TOKENS total (protocol, extra)                   8447        8922        8449        8405        8615        8600        8784       11843       11319       12226        8552       10681       10357       11124
- source edits                                        4           4           4           4           4           4           4          23          22          24           4          18          15          21
- god-method rejections                               1           1           1           1           1           1           1           3           5           4           0           3           3           3
- worst dispatch control-flow complexity              7           7           7           7           7           7           7           7           7           7        None           7           7           7
- ledger built from                                json        json        json        json        json        json        json        json        json        json        json        json        json        json
```

## Pooled with the 5fd65db run

`36636f8` touches only `docs/decisions`, `docs/language.md`, and one test
assertion. `git diff 5fd65db 36636f8 -- compiler/ail-compiler/src` is empty and
cargo had nothing to rebuild, so the compiler binary and therefore the measured
condition are identical. The two runs are poolable:

```
ail      n=14  retries [1, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3]        median 2   worst 3
control  n=14  retries [1, 2, 2, 3, 8, 10, 11, 11, 12, 13, 13, 13, 14, 14] median 11  worst 14
```

Fourteen trials per arm on one compiler. The `ail` arm never exceeded 3 retries
in fourteen attempts. The control arm is bimodal: four of fourteen trials landed
at 1 to 3 retries by getting the whole repair right before the first gate call,
and the other ten needed 8 to 14. That bimodality, not the median, is the honest
description of what withholding compiler output does.

The earlier pre-#5 run is not pooled here, because it ran on a different
compiler. It is kept in `runs-pre-pr5/` and showed the same direction with a
smaller gap.

## God methods

Thirteen of fourteen trials submitted a `transport.dispatch` measuring
control-flow complexity 7 against the policy budget of 4, and the architecture
checker denied it in both arms. `control-t4` was the exception: it computed the
budget in advance and never submitted an over-budget dispatch. No trial
published one, so the gate prevented the god method regardless of visibility.
Visibility changed the cost of learning why: one denial and one edit for every
`ail` trial, reading `facts.candidate_cfc=7` and the requirement `control-flow
complexity must stay at most 4 and minimal review context at most 12; the
candidate measured 7 and 6`, against up to five denials of the same rule for
`control-t2`.

## What actually separated the arms

Only one of the six faults was hard to see by reading. Every trial in both arms
found the missing import, the `Int` in a `Text` field, the unknown capability,
and the non-exhaustive match from the source and the reference material. The
`ail` ledgers reached a median of one diagnostic class:
`AIL.ARCH.HOTSPOT_GROWTH`. That is where the compiler earned its keep, and a
control agent can substitute arithmetic for it, since `architecture.json` states
the budget and `specs/architecture.md` gives the metric as `E - N + 2`. One
control trial in seven did that arithmetic correctly up front. Six did not, and
paid 8 to 14 retries, and five of them shipped an invented fault.

## Threats to validity

1. Seven trials per arm here, fourteen pooled on one compiler. The control
   distribution is bimodal, so its median is a poor summary and its mean is
   worse. Report the distribution.
2. The control arm can read the policy file and the metric definition, so most
   compiler facts in this fixture are re-derivable by hand. That is realistic
   and it caps how large the gap can be; `control-t4` is what the cap looks
   like.
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

- A weaker model as a second condition, and enough trials to characterize the
  control arm's bimodality rather than its median.
- A fixture too large to hold in context, where the agent must locate the
  relevant module before it can reason about it.
- A fault whose fix is not derivable from readable policy, which is the one
  route `control-t4` used to win.
- Behavior verification in the gate, so specification text cannot stand in for
  working code.

## Audit of all fourteen final workspaces

`architecture.json` byte-identical to the fixture in all fourteen. Zero
task-specification violations in every final source. No comments used to satisfy
a requirement. No arm emptied or weakened policy, and the harness never applied a
checker fact to source.

## Runs kept for the record

- `runs/`, `report/`: this run, against `main` at `36636f8`.
- `runs-pr5-5fd65db/`: seven per arm against `5fd65db`, the same compiler
  binary. Poolable with this run.
- `runs-pre-pr5/`: seven per arm against the one-line diagnostic format, before
  #5. Different compiler, not pooled.
- `runs-v1-gate-defect/`: the first two trials, invalidated by a harness bug
  where `check` reported `FAIL` even when `ailc check` passed.

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
5. The line-format ledger parser could not read #5's findings. The gate now
   reads `ailc check --json` and falls back to text when the compiler has no
   JSON view, recording which it used.
