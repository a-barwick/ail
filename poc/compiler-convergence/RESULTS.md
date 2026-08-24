# Results: seven trials per arm, same broken workspace

## Superseded: these numbers measure the pre-#5 diagnostic format

All fourteen trials below ran against this branch before
[PR #5](https://github.com/a-barwick/ail/pull/5) merged, so the `ail` arm saw
the old one-line diagnostic format plus the architecture facts this branch added.
PR #5 replaces that with located structured findings, a `requirement` line per
error, and `ailc check --json`, and a follow-up on that branch cut the two
requirements that prescribed an edit rather than stating a constraint. The `ail`
arm's information therefore changes when #5 lands, so **both arms must be re-run
against the merged tree before any number here is quoted as the result.**

Nothing about the control arm changes: it never sees compiler output. The
comparison has to be re-run anyway, because an arm measured against a different
compiler is not comparable to one measured against this branch.

The numbers stay in this file because they are real, auditable, and the
mechanism they exposed is unlikely to reverse. Treat them as the pre-#5
baseline, not the result.

The harness is already ported: it reads its ledger from `ailc check --json` when
the compiler offers it and falls back to parsing text when it does not, so the
same measures come out of either tree. `- ledger built from` in the report says
which was used.

## The pre-#5 baseline

Run on 2026-08-24. Fourteen agent runs, seven per arm, same model, same broken
fixture, same `ailc publish` gate, same reference material. The only difference
between arms is whether the agent may see compiler output.

Measures are reported in the order that decides the outcome:

1. **Retries to a passing publish** is the win condition.
2. **Rebreak** is whether the change was actually safe.
3. **Tokens** are extra and decide nothing on their own.

## 1. Retries to a passing publish (the win condition)

```
ail      n=7  retries [1, 2, 2, 2, 2, 3, 4]   median 2  worst 4   total 16
control  n=7  retries [1, 2, 3, 4, 9, 12, 16] median 4  worst 16  total 47

paired comparisons (49, every ail trial against every control trial):
  control needed more retries in 33, fewer in 9, equal in 7
```

Both arms converged in all seven trials; neither hit the 40-call limit.

**The `ail` arm wins the win condition on median and on worst case, and it is
not a clean sweep.** Median 2 retries against 4. Worst case 4 against 16. Total
16 against 47, so the blind arm spent roughly three times the retries overall.
In 33 of 49 paired comparisons the control trial needed more retries, in 9 it
needed fewer, and in 7 the two tied.

The overlap is real and worth naming: `control` and `control-t4` needed 1 and 2
retries, matching the best `ail` trials. A lucky first-pass repair does not need
the compiler. What the compiler changes is the tail. Three of seven control
trials needed 9, 12, and 16 retries; no `ail` trial needed more than 4.

## 2. Rebreak: did a later edit break an earlier check

```
ail      rebreak events 0, across 0 of 7 trials
control  rebreak events 9, across 4 of 7 trials
         fix cycles: ail 3 events in 2 trials, control 14 events in 4 trials
```

**The `ail` arm wins this outright: zero rebreaks in seven trials.** No `ail`
trial ever saw a diagnostic reappear after fixing it. Four of seven control
trials did, nine times in total.

A rebreak is recorded when a diagnostic that was fixed at some earlier gate call
comes back at a later one. Identity is the diagnostic's code plus its detail or
fact payload, never its span, because spans move on every edit. The harness runs
the real compiler in the control arm too and records the diagnostics it refuses
to show, so both ledgers are built by the same checker.

The mechanism is visible in the ledgers. With binary feedback, the rational move
is to break working code on purpose: gut a module, watch the bit, infer. Three
control trials bisected the workspace that way, and it cost them. `control-t5`,
`control-t6`, and `control-t7` all reduced modules to trivial bodies, hit
`AIL.ARCH.ANALYSIS_INCOMPLETE base architecture is incomplete` because a group
had lost all its units, and concluded the original `contracts.ail` needed an
executable function to satisfy coverage. It did not. Four `ail` trials and two
control trials published with `contracts.ail` byte-identical to the fixture,
types only. Those three trials spent retries fixing a fault they had created and
then wrote it into their final source. That is the "fix one, break another"
failure, and it happened only where the compiler's facts were withheld.

## 3. Tokens (extra)

```
ail      median 8305  range 5623 to 8752
control  median 9096  range 8155 to 12268
```

Protocol tokens: briefs, file reads, gate feedback, and written source. Median
is about 9 percent lower for `ail`, and the worst control trial cost 40 percent
more than the worst `ail` trial. Most of the volume in both arms is reference
reading before the first edit, which is identical by construction, so the arms
can only differ in the tail — the same place the retries differ. Tokens do not
change the verdict either way here; they agree with it.

Protocol tokens exclude an agent's private reasoning tokens, which this
environment does not expose. They are a lower bound, measured identically for
both arms.

## Full table

```
measure                                             ail      ail-t2      ail-t3      ail-t4      ail-t5      ail-t6      ail-t7     control  control-t2  control-t3  control-t4  control-t5  control-t6  control-t7
1 RETRIES to passing publish                          2           2           2           3           2           4           1           1           4           3           2          12           9          16
  converged                                        True        True        True        True        True        True        True        True        True        True        True        True        True        True
2 REBREAKS (fixed, then broken again)                 0           0           0           0           0           0           0           0           1           0           0           4           2           2
  fix cycles (new breakage after an edit)             0           0           0           1           0           2           0           0           2           0           0           5           3           4
  attempts failing the task specification             0           0           0           1           1           1           0           0           2           1           0           8           6          12
3 TOKENS total (protocol, extra)                   8100        8087        8379        8493        8305        8752        5623        8155        9096        8547        8501       10930       10082       12268
- source edits                                        4           4           4           4           3           5           3           3          10           5           6          22          16          31
- god-method rejections                               1           1           1           1           0           1           0           0           2           1           0           4           3           3
- worst dispatch control-flow complexity              7           7           7           7        None           7        None        None           7           7        None           7           7           7
```

## God methods

Eleven of fourteen trials submitted a `transport.dispatch` that inlined
validation and a five-arm match. The architecture checker measured control-flow
complexity 7 against the policy budget of 4 and denied it, in both arms. Every
trial ended up splitting the decision into a second unit, and no trial published
an over-budget dispatch.

So the gate prevented the god method regardless of visibility. What visibility
changed is the cost of learning why: the `ail` arm read
`base_cfc=4 candidate_cfc=7` and fixed it in one edit, while control trials
needed up to 4 separate denials of the same rule (`control-t5`) to locate it.

## What actually separated the arms

Only one of the six faults was hard to see by reading. Every trial in both arms
found the missing import, the `Int` in a `Text` field, the unknown capability,
and the non-exhaustive match from the source, `docs/STATUS.md`, and
`compiler/examples/architecture-denied`. The `ail` ledgers show a median of one
diagnostic class ever reached: `AIL.ARCH.HOTSPOT_GROWTH`. That is where the
compiler earned its keep, and it is also where a control agent could substitute
arithmetic, since `architecture.json` states the budget and
`specs/architecture.md` gives the metric as `E - N + 2`. Two control trials did
that arithmetic correctly and matched the best `ail` trials. Five did not.

The compiler's advantage here is not that its facts are unavailable elsewhere.
It is that they arrive without the agent having to be right about them first,
and that being wrong about them is expensive: 9, 12, and 16 retries, with a
false conclusion written into the final source in three trials.

## Threats to validity

1. Seven trials per arm, one fixture, one model. The medians and the tail are
   consistent, but this is still a demonstration and not a measurement with
   error bars.
2. The control arm can read the policy file and the metric definition, so most
   compiler facts in this fixture are re-derivable by hand. That is realistic,
   a project's policy is readable, and it caps how large the gap can be.
3. The fixture is five modules and six faults, small enough to hold in context
   entirely. The thesis is about scale; this does not test scale.
4. Both arms used the same strong model. The compiler's facts should matter more
   for a weaker or cheaper model. Not tested.
5. The gate is `ailc publish` plus a task specification matched against
   normalized source text. There is no behavior test in this language fragment,
   so an arm can satisfy the letter of the specification while losing behavior.
   Two holes of this kind were found and closed during this work; more may
   remain.
6. Both arms are told which task-specification rules fail. That is symmetric,
   but it hands both arms one of the six faults, the missing match arm, with no
   compiler involvement. Three control trials used the specification list as a
   free oracle while bisecting.
7. Nothing physically prevented a control agent from reconstructing the
   workspace from `runs/<arm>/state.json` and compiling it itself. The brief
   forbids it and the committed command logs show none did, but this is a trust
   assumption, not an enforcement.

## What would make this a real measurement

- A weaker model as a second condition, and enough trials to put error bars on
  the tail rather than the median.
- A fixture too large to hold in context, where the agent must locate the
  relevant module before it can reason about it.
- A fault whose fix is not derivable from readable policy: cross-module type and
  effect interactions several calls deep, where the compiler's answer is cheap
  and hand derivation is not.
- Behavior verification in the gate, so specification text cannot stand in for
  working code.

## Defects found in the harness, and fixed

Recorded because they changed the numbers.

1. **The gate lied about `check`.** The first run reported `FAIL` for a `check`
   that passed, because only `publish` could report `PASS`. The `ail` arm saw
   `ok` in the compiler output and ignored the label; the control arm could not,
   and spent 18 gate calls probing a workspace that had been correct since its
   second attempt. That run measured my bug, not the withheld diagnostics, and
   is archived as invalid in `runs-v1-gate-defect/`. `check` now reports its own
   outcome in both arms, and every trial in this report ran on the fixed gate.
2. **The ledger dropped diagnostics.** `AIL.ARCH.ANALYSIS_INCOMPLETE` lines
   carry no rule field and matched no parser, so five control-arm failures
   recorded no diagnostic at all. Unrecognized compiler output is now recorded
   as `HARNESS.UNPARSED_OUTPUT` rather than vanishing. That fix is what made the
   `contracts.ail` coverage story above visible.
3. **The specification was too weak.** It required only that a function named
   `summarize` exist, so an arm could hollow it out to an identity function and
   still pass. It now pins the four outcome labels and the control-character
   rule, which both arms must implement.
4. **Comments satisfied the specification.** AIL has `//` and `/* */` comments,
   and the specification was matched against raw source text, so commenting a
   requirement out satisfied it. The archived invalid run exploited this.
   Normalization now strips comments, and the self-test asserts that a
   commented-out requirement fails. All fourteen trials were audited against the
   hardened check: policy byte-identical in all fourteen, zero specification
   violations in every final source, no comments in any final source.
