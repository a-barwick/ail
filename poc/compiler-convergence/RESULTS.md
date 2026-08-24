# Results: three trials per arm, same broken workspace

Run on 2026-08-24. Six agent runs, all with the same model, the same broken
fixture, the same `ailc publish` gate, and the same reference material. The only
difference between arms is whether the agent may see compiler output.

**The thesis is not proven by these numbers.** The `ail` arm wins the median on
every measure, but the margins are one gate call and about five percent of
protocol tokens, and one control trial beat all three `ail` trials outright.
What the numbers do show cleanly is variance: the compiler arm cost the same
amount three times out of three, and the blind arm did not.

## Measures

```
per trial

measure                                           ail      ail-t2      ail-t3     control  control-t2  control-t3
converged                                        True        True        True        True        True        True
gate calls to pass                                  3           3           3           2           5           4
first check that passed                             2           2           2           1        None           2
tokens harness to agent                          7699        7701        7993        7883        8361        8112
tokens agent to harness                           401         386         386         272         735         435
tokens total (protocol)                          8100        8087        8379        8155        9096        8547
source edits                                        4           4           4           3          10           5
reads                                              20          20          23          23          25          25
distinct diagnostics reached                        1           1           1           0           2           1
fix cycles (new breakage after an edit)             0           0           0           0           2           0
rebreaks (fixed then broken again)                  0           0           0           0           1           0
god-method rejections                               1           1           1           0           2           1
worst dispatch control-flow complexity              7           7           7        None           7           7
attempts failing the task specification             0           0           0           0           2           1

median of ail: 3 trials, control: 3 trials

measure                                           ail     control
gate calls to pass                                  3           4
tokens total (protocol)                          8100        8547
source edits                                        4           5
fix cycles (new breakage after an edit)             0           0
rebreaks (fixed then broken again)                  0           0
attempts failing the task specification             0           1
```

## Measure by measure

**Fewer tokens: yes, by about five percent at the median, and that is small.**
Median protocol tokens were 8100 for `ail` and 8547 for `control`. The ranges
overlap: the best control trial (8155) used fewer tokens than the worst `ail`
trial (8379). The control arm's extra cost is concentrated in one trial that
had to probe (9096). Most of the token volume in both arms is reference reading
before the first edit, which is identical by construction, so the arms can only
differ in the tail.

**Fewer cycles on bug fixes: yes, and this is the cleanest result.** All three
`ail` trials took exactly three gate calls: one failing `check`, one passing
`check`, one passing `publish`. Control took 2, 5, and 4. Every fix cycle,
every rebreak, and every specification-violating attempt in the entire
experiment came from the control arm. `control-t2` recorded two fix cycles and
one rebreak: it submitted the correct repair, learned only `FAIL`, tore the
workspace down to trivial bodies to isolate the failure, then rebuilt it and
re-hit the same complexity denial it had already hit at attempt 1.

**A change that passes one check must not silently break another: yes, and the
mechanism is visible.** The `ail` arm never introduced a diagnostic it had not
already seen. The control arm broke working code on purpose, because with
binary feedback that is the rational move: deliberately degrade the workspace,
observe the bit, and infer. Two control trials did this. That is exactly the
"fix one, break another" pattern, and it appeared only where the compiler's
facts were withheld.

**Fewer accidental god methods: the guardrail did it, not the diagnostic.**
Five of six trials submitted a `transport.dispatch` that inlined validation and
a five-arm match. The architecture checker measured control-flow complexity 7
against the policy budget of 4 and denied it, in both arms. Every trial ended
up splitting the decision into a second unit. So the god method was prevented by
the gate regardless of visibility. What visibility changed is how the agent
learned why: the `ail` arm read `base_cfc=4 candidate_cfc=7` and fixed it in one
edit; `control-t2` needed three more gate calls and a teardown to locate the
same fact; `control-t3` needed one probe; `control` derived the metric by hand
from `specs/architecture.md` and never hit the denial at all.

## What actually separated the arms

Only one of the six faults was hard to see by reading. Every trial in both arms
found the missing import, the `Int` in a `Text` field, the unknown capability,
and the non-exhaustive match by reading the source, `docs/STATUS.md`, and
`compiler/examples/architecture-denied`. The `ail` arm's ledger shows exactly
one diagnostic class ever reached: `AIL.ARCH.HOTSPOT_GROWTH`. That is where the
compiler earned its keep, and it is also where the control arm could partly
substitute arithmetic: `architecture.json` states the budget and
`specs/architecture.md` gives the metric as `E - N + 2`, so a careful agent can
compute the denial before submitting. One control trial did, and won the whole
experiment with two gate calls.

That is the honest boundary of this result. The compiler's advantage here is
not that its facts are unavailable elsewhere; it is that they arrive without
the agent having to be right about them first. Two of three control trials were
not right the first time.

## Threats to validity

1. Three trials per arm, one fixture, one model. This is a demonstration, not a
   measurement with error bars. A one-call median difference over three trials
   is inside the noise the trials themselves show.
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
   remain. See "defects found in the harness".
6. Protocol tokens exclude an agent's private reasoning tokens, which this
   environment does not expose. They are a lower bound, measured identically
   for both arms.
7. Nothing physically prevented a control agent from reconstructing the
   workspace from `runs/<arm>/state.json` and compiling it itself. The brief
   forbids it and the committed command logs show none did, but this is a trust
   assumption, not an enforcement.
8. Both arms are told which task-specification rules fail. That is symmetric,
   but it hands both arms one of the six faults (the missing match arm) without
   any compiler involvement.

## What would make this a real measurement

- Many more trials per arm, and a weaker model as a second condition.
- A fixture too large to hold in context, where the agent must locate the
  relevant module before it can reason about it.
- A fault whose fix is not derivable from readable policy: cross-module type
  and effect interactions several calls deep, where the compiler's answer is
  cheap and hand derivation is not.
- Behavior verification in the gate, so specification text cannot stand in for
  working code.

## Defects found in the harness, and fixed

Recorded because they changed the numbers.

1. **The gate lied about `check`.** The first run reported `FAIL` for a `check`
   that passed, because only `publish` could report `PASS`. The `ail` arm saw
   `ok` in the compiler output and ignored the label; the control arm could
   not, and spent 18 gate calls probing a workspace that had actually been
   correct since its second attempt. That run measured my bug, not the withheld
   diagnostics, and is archived as invalid in `runs-v1-gate-defect/`. `check`
   now reports its own outcome in both arms.
2. **The ledger dropped diagnostics.** `AIL.ARCH.ANALYSIS_INCOMPLETE` lines
   carry no rule field and matched no parser, so five control-arm failures
   recorded no diagnostic at all. Unrecognized compiler output is now recorded
   as `HARNESS.UNPARSED_OUTPUT` rather than vanishing.
3. **The specification was too weak.** It required only that a function named
   `summarize` exist, so an arm could hollow it out to an identity function and
   still pass. It now pins the four outcome labels and the control-character
   rule, which both arms must implement.
4. **Comments satisfied the specification.** AIL has `//` and `/* */`
   comments, and the specification was matched against raw source text, so
   commenting a requirement out satisfied it. The archived invalid run exploited
   this. Normalization now strips comments, and the self-test asserts that a
   commented-out requirement fails. All six trials in this report were
   re-audited against the hardened check: policy byte-identical in all six,
   zero specification violations, no comments in any final source.
