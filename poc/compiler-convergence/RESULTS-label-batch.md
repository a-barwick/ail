# Results: fixture `label-batch`

**The compiler-visibility arm did not win this fixture.** With seven trials per
arm, the blind arm reached a passing publish in 1 gate call before the publish
and the compiler arm needed 2, in 42 of 49 paired comparisons and never worse.
In a second condition where both arms were told gate calls are expensive, every
trial in both arms published on its first gate call, and the compiler arm never
looked at a diagnostic at all.

This is a different experiment from the first fixture and its numbers are not
comparable to it. Do not read [RESULTS.md](RESULTS.md) as this fixture's result.
The fixture, its seven faults, and how to run it are in
[fixture-label-batch/README.md](fixture-label-batch/README.md).

Run on 2026-08-24 on branch `ab/type-capability-fixture-0224`, whose parent is
`main` at `d52141f`. No compiler source changed on this branch, so `ailc` is the
same binary the first fixture's latest run used. Every ledger was built from
`ailc check --json`. Both arms used the same model.

Measures in the order that decides the outcome, unchanged from the first
fixture:

1. **Retries to a passing publish** is the win condition.
2. **Rebreak** is whether the change was actually safe.
3. **Tokens** are extra and decide nothing on their own.

## 1. Retries to a passing publish (the win condition)

```
ail      n=7  retries [1, 2, 2, 2, 2, 2, 2]  median 2  worst 2  did not converge 0
control  n=7  retries [1, 1, 1, 1, 1, 1, 1]  median 1  worst 1  did not converge 0

paired comparisons (49, every ail trial against every control trial):
  control needed more retries in 0, fewer in 42, equal in 7
```

**`control` wins the win condition, with ties.** No `ail` trial needed fewer
retries than any `control` trial.

`retries_to_pass` counts every gate call before the publish that passed, whether
that call failed or succeeded. Splitting it by outcome:

| arm | trials | failed gate calls before the pass | passing checks before the pass |
| --- | --- | --- | --- |
| `ail` | 6 of 7 | 1 | 1 |
| `ail` | 1 of 7 (`ail-t6`) | 0 | 1 |
| `control` | 7 of 7 | 0 | 1 |

Six of seven `ail` trials ran the same trajectory: `check` on the broken
workspace (13 findings), three writes, `check` (clean), `publish` (PASS). All
seven `control` trials ran: three writes, `check` (clean), `publish` (PASS). The
one `ail` trial that skipped the opening probe, `ail-t6`, tied the control arm at
1 retry.

## 2. Rebreak: did a later edit break an earlier check

```
ail      rebreak events 0, across 0 of 7 trials
control  rebreak events 0, across 0 of 7 trials
         fix cycles: 0 in both arms; attempts failing the task specification: 0 in both arms
```

Nothing to separate the arms. No trial in either arm ever broke something it had
already fixed, because no trial in either arm ever needed a second repair pass.

## 3. Tokens (extra)

```
ail      median 8154   range 7015 to 8539
control  median 7020   range 6448 to 7357
```

The compiler arm cost about 16 percent more protocol tokens, and the reason is
visible in one number: the findings dump on the opening `check` is 1,111 tokens
(5,194 characters, 114 lines) for thirteen findings. `ail-t6`, the trial that
never asked for it, cost 7,015, inside the control arm's range.

## The repairs were identical, so the findings changed nothing

All fourteen primary trials and all six secondary trials edited exactly the three
faulty modules — `batch.ail`, `classify.ail`, `report.ail` — created no new
module, and left `types.ail` byte-identical to the fixture. Every published
workspace satisfies the task specification.

`batch.ail` and `classify.ail` are byte-identical across all twenty trials. The
only variation anywhere is the name an arm chose for one unused match binding in
`report.ail` (`value`, `raw`, `reason`, `illegal_value`, `illegal_text`,
`accepted_text`). Every trial in both arms produced the same repair, so seeing the
compiler's thirteen findings did not change what any arm wrote.

Audit of all twenty runs (`harness.py audit`): 120 checks, all passing. No
architecture finding appeared in any gate call in any trial, in either arm, which
is the structural claim this fixture makes: it has no `architecture.json`.

## Secondary condition: gate calls declared expensive

One operator instruction, worded identically for both arms, added to the primary
protocol: *gate calls are the expensive resource; use as few as you can; plan
your whole repair first*. Three trials per arm, same fixture, same brief,
separate runs in `runs-label-batch-frugal/`.

```
ail      n=3  retries [0, 0, 0]  median 0
control  n=3  retries [0, 0, 0]  median 0
tokens   ail median 7358, control median 7645
distinct diagnostics reached: 0 in every trial of both arms
```

Every trial in both arms wrote all three modules and then called `publish` once,
which passed. The compiler arm reached zero diagnostics: it published without
ever seeing compiler output. On this fault set the treatment can be skipped
entirely and still succeed on the first attempt.

All six arms gave the same reason for spending one call: `publish` runs the same
checks as `check`, emits the same findings on failure, and writes nothing when it
fails, so `check` is dominated. That reasoning is what switched the compiler arm's
information channel off. Under the primary protocol its brief tells it to treat
compiler output as the only source of truth, and it opened with a probe; told to
economize, it reasoned that a failing `publish` would have returned the same
findings anyway, and never needed them.

That condition also answers the obvious objection to the primary numbers. The
`ail` brief tells that arm to treat compiler output as its only source of truth,
which encourages spending a gate call on a diagnostic probe. So the primary
result is not "compiler output costs a retry" as a general claim. It is: on this
fault set the probe bought nothing, and an arm told to economize skipped it.

## Why the blind arm wins here

Every one of the seven faults is local. Each one is visible at its own site by
reading six files totalling about ninety lines: a list bound that disagrees with
the `map` above it, an `effects` clause naming a parameter the function does not
have, an argument of the wrong type, a variant case built without its payload, a
match arm with no binding for a payload, a record literal missing a field, and a
capability parameter whose interface nothing supplies. A strong model reading the
workspace finds all seven and repairs them in one pass.

Nothing in this fixture requires a number that only the compiler computes. That
is the difference from the first fixture, where the fault that separated the arms
was a measured control-flow complexity against a policy budget, and where five
task-specification rules were violated by the broken source and gave every
bisecting control trial a free oracle. Here the broken fixture satisfies every
specification rule, so the specification points at nothing, and the compiler's
findings restate what the source already shows.

The honest summary of both fixtures together: compiler visibility paid when the
repair depended on a fact the compiler alone measures, and paid nothing when the
faults were locally readable type and capability errors at this scale.

## Threats to validity

1. Seven trials per arm in the primary condition, three in the secondary, one
   model, one fixture. Every measure was deterministic inside an arm, so these
   numbers are stable but narrow. They say nothing about a weaker model.
2. The fixture is six files and about ninety lines, small enough to hold in
   context entirely. Locality is exactly why the control arm wins, so this result
   is a statement about small local fault sets, not about scale.
3. The `ail` brief encourages a diagnostic probe before editing. The secondary
   condition isolates that effect but does not remove it from the primary
   numbers.
4. `LabelRequest.origin` is dead in every arm's repair: all twenty trials built
   `Label::Illegal(request.raw)` where the stored reference repair used
   `request.origin`. The specification does not pin which text the `Illegal`
   payload carries, so the field exists only to make the payload a real choice
   and no arm chose it.
5. Behavior is checked by matching normalized source text, not by executing
   anything. An arm can satisfy the letter of a rule.
6. Trust assumption, and it is weaker here than it should be. While the arms
   ran, the broken sources, the reference repair, and the negative controls were
   moved out of the repository and had never been committed, so no arm could read
   the fixture or the answer from the tree or from git history. But the trials ran
   concurrently, so an `ail` arm's state file, which contains the full compiler
   output, existed on the same machine while the `control` arms were working. The
   prompts forbade reading any state file and every state file carries an operator
   canary asking a reader to disclose, but nothing enforced it. The timestamps do
   not clear it either: each `control` arm wrote its repair 96 to 597 seconds
   after the first `ail` findings landed on disk.

   A peeking control arm and a reading control arm would produce the same number
   here, since 1 retry is the minimum for a check-then-publish strategy, so this
   cannot be resolved after the fact. The protocol fix is to stagger the arms so
   that no `ail` ledger exists on disk while a `control` trial runs. Treat the
   control arm's 1 retry as an upper bound that the next run should confirm under
   staggering.

   All twenty arms reported, unprompted and in detail, that they used only harness
   commands: no state file read, no `ailc` or `cargo` run, no private copy of the
   workspace compiled, no other arm's files read, and reference reads limited to
   the paths in the brief. Several disclosed running `git status`, which printed
   the run directories' names and no contents. Those are self-reports from the
   arms themselves, which is weaker evidence than enforcement, and they are
   consistent with what each committed command log shows.

## What would make the next fixture decide something

- A fault whose repair depends on a fact no reader can recompute, which is the
  one thing that separated the arms in the first fixture.
- A workspace too large to read in full, so locating the fault is part of the
  cost.
- Behavior verification in the gate, so specification text cannot stand in for
  working code.
- A weaker or cheaper model as a second condition.
- Staggered arms: every `control` trial finished before any `ail` trial starts,
  so no ledger holding compiler output exists on disk while a blind arm works.

## Full tables

Primary condition, `report-label-batch/measures.txt`:

```
measure                                               ail      ail-t2      ail-t3      ail-t4      ail-t5      ail-t6      ail-t7     control  control-t2  control-t3  control-t4  control-t5  control-t6  control-t7
1 RETRIES to passing publish                            2           2           2           2           2           1           2           1           1           1           1           1           1           1
  of those, gate calls that failed                      1           1           1           1           1           0           1           0           0           0           0           0           0           0
  gate calls to pass                                    3           3           3           3           3           2           3           2           2           2           2           2           2           2
2 REBREAKS (fixed, then broken again)                   0           0           0           0           0           0           0           0           0           0           0           0           0           0
  fix cycles (new breakage after an edit)               0           0           0           0           0           0           0           0           0           0           0           0           0           0
  attempts failing the task specification               0           0           0           0           0           0           0           0           0           0           0           0           0           0
3 TOKENS total (protocol, extra)                     8521        7844        7867        8169        8539        7015        8154        7357        7020        6804        6448        7020        7237        6990
- source edits                                          3           3           3           3           3           3           3           3           3           3           3           3           3           3
- distinct diagnostics reached                         10          10          10          10          10           0          10           0           0           0           0           0           0           0
- distinct type diagnostics reached                     6           6           6           6           6           0           6           0           0           0           0           0           0           0
- distinct capability diagnostics reached               4           4           4           4           4           0           4           0           0           0           0           0           0           0
- distinct architecture diagnostics reached             0           0           0           0           0           0           0           0           0           0           0           0           0           0
```

Secondary condition, `report-label-batch-frugal/measures.txt`:

```
measure                                               ail      ail-t2      ail-t3     control  control-t2  control-t3
1 RETRIES to passing publish                            0           0           0           0           0           0
  gate calls to pass                                    1           1           1           1           1           1
2 REBREAKS (fixed, then broken again)                   0           0           0           0           0           0
3 TOKENS total (protocol, extra)                     6998        7358        7544        7700        7343        7645
- source edits                                          3           3           3           3           3           3
- distinct diagnostics reached                          0           0           0           0           0           0
```

`report-label-batch/measures.json` has the per-attempt detail and
`runs-label-batch/<arm>/state.json` the full ledger: every command, every edit,
every gate call, the compiler output for each, and the final source.

## Reproducing

```bash
cargo +1.87.0 build --release -p ail-compiler --bin ailc
python3 poc/compiler-convergence/harness.py self-test --fixture label-batch
python3 poc/compiler-convergence/harness.py audit     --fixture label-batch
python3 poc/compiler-convergence/harness.py audit     --fixture label-batch-frugal
python3 poc/compiler-convergence/harness.py report    --fixture label-batch --operator
python3 poc/compiler-convergence/harness.py report    --fixture label-batch-frugal --operator
```
