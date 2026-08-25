# Compiler convergence proof-of-concept

One broken AIL workspace, two agents, one success gate. The arms differ in a
single variable: whether the agent may see what the AIL compiler already knows.

Two independent fixtures run on this harness. Their numbers are separate
experiments and must not be quoted as one result.

| fixture | faults | who won the win condition | results |
| --- | --- | --- | --- |
| `cancel-dispatch` (default) | module, type, capability, and an architecture complexity budget | the compiler arm, median 2 retries against 11 | [RESULTS.md](RESULTS.md) |
| `label-batch` | type, capability, and effect only; no `architecture.json` in the workspace | the blind arm, 1 retry against 2 | [RESULTS-label-batch.md](RESULTS-label-batch.md), [fixture-label-batch/README.md](fixture-label-batch/README.md) |

Read that second row before quoting the first. Compiler visibility paid when the
repair depended on a number only the compiler measures, and paid nothing when
every fault was a locally readable type or capability error in six small files.

Pass `--fixture <name>` to every harness command. It defaults to
`cancel-dispatch`, so the commands in this file run the first fixture. The rest
of this file describes that fixture.

## Fixture `cancel-dispatch`

Results are in [RESULTS.md](RESULTS.md), from seven trials per arm against
`main` at `36636f8` plus seven more against `5fd65db`, which is the same
compiler binary. Pooled over fourteen trials per arm, in priority order.

On **retries to a passing publish**, the win condition, the compiler arm ran
1,2,2,2,2,2,2,2,3,3,3,3,3,3 and the blind arm ran
1,2,2,3,8,10,11,11,12,13,13,13,14,14: median 2 against 11, worst 3 against 14.
The compiler arm never exceeded 3 retries in fourteen trials. The blind arm is
bimodal — four trials got the whole repair right before the first gate call and
finished in 1 to 3, the other ten fell into bisection and needed 8 to 14. Report
that distribution, not the median.

On **rebreak**, whether a later edit broke an earlier check, the compiler arm
had zero events in fourteen trials. **Tokens** are extra: median 8600 against
11124 on the latest run.

Both fixtures use the same two arms:

- Arm `ail`: `check` and `publish` return the compiler's own output, including
  `expected.type`, `expected.cases`, `expected.capability`, and architecture
  findings with the numbers the checker measured.
- Arm `control`: the same broken workspace, the same `ailc publish` gate, and
  compiler output withheld. Every gate call returns `PASS` or `FAIL` and
  nothing else.

The control arm is not a rewrite in another language. It is the same AIL repair
job with the compiler's facts hidden.

## What is being measured

The harness records every gate call for both arms. It runs the real compiler in
both arms; in the control arm it refuses to show the diagnostics and keeps them
out of that arm's ledger until `unseal` rebuilds them from the arm's own sources
after it finishes. That is what makes the two arms comparable: the same ledger
is built from the same checker either way.

Ranked by what decides the outcome. Retries to a passing publish is the win
condition. Rebreak is whether the change was safe. Tokens are extra and decide
nothing on their own.

| measure | how it is computed |
| --- | --- |
| 1. retries to a passing publish | failed gate calls before the `publish` that passed the gate |
| 2. rebreaks | diagnostics that were fixed and later came back |
| 2b. fix cycles | gate calls where a diagnostic appeared that the previous call did not have, that is, breakage introduced by the last edit |
| 3. tokens | protocol tokens between agent and harness, both directions |
| god-method rejections | gate calls denied by `M23-POL-DISPATCH-NO-GROWTH`, `M23-POL-NEW-UNIT`, `M23-POL-TRANSPORT-CAPABILITY`, or `M23-POL-TRANSPORT-STATE` |
| worst dispatch control-flow complexity | highest `candidate_cfc` the architecture checker measured for `transport:dispatch`, against a policy budget of 4 |
| distinct diagnostics reached | how deep into the failure chain the arm got |

A diagnostic's identity is its code plus its detail or fact payload, never its
span, because spans move on every edit. Token counts come from a documented
offline pre-tokenizer (an approximation of `cl100k_base` pre-tokenization);
character counts are reported beside them so anyone can recompute with a real
tokenizer.

Token counts cover the agent-harness protocol: briefs, file reads, gate
feedback, and written source. They do not include an agent's private reasoning,
which this environment does not expose. Protocol tokens are therefore a lower
bound on total cost, measured identically for both arms.

## The task

`fixture/broken/` is a five-module workspace derived from
`compiler/examples/architecture-denied` (same module groups, same policy shape)
with types borrowed from the cancel-job workload. It carries six faults:

1. `tests.ail` calls `transport.dispatch` without importing `transport`.
2. `tests.ail` builds `CancelRequest { value: 1 }` where the field is `Text`.
3. `transport.dispatch` matches four of the five `StoreOutcome` cases.
4. `domain.summarize` takes `store: capability JobsStore`, which no capability
   environment provides.
5. `transport.dispatch` calls `domain.normalize`, and policy allows the
   transport group to depend on the contract group only.
6. `transport.dispatch` inlines validation and classification, over the
   `dispatch_no_growth` budget.

Faults 5 and 6 interact: removing the `domain` call while leaving the logic
inlined still busts the complexity budget, and splitting the logic into the
`domain` module re-creates the boundary violation. The only shape that passes
is a thin `transport.dispatch` over contract-group logic.

The compiler reveals these in stages. Names and imports come first, then types,
then capabilities, then architecture. An arm that cannot see diagnostics cannot
see that ordering either.

## Running the arms

Build the compiler once:

```bash
cargo +1.87.0 build --release -p ail-compiler --bin ailc
```

Prove the fixture is solvable, the gate cannot be gamed, a failed publish writes
nothing, and a blind trial runs before any compiler-arm ledger exists:

```bash
python3 poc/compiler-convergence/harness.py self-test \
  --reference poc/compiler-convergence/fixture/reference-solution
```

`fixture/reference-solution/` is one repair that passes the gate. It is kept
outside the working tree while the arms run, so neither agent can read it, and
committed afterwards so the self-test and the solvability claim are auditable.
Git history shows which commit added it.

Start an arm and hand its brief to an agent. The blind arms run first, so start
with `--arm control`:

```bash
python3 poc/compiler-convergence/harness.py start --arm control
python3 poc/compiler-convergence/harness.py brief --arm control
```

The agent then works only through the harness:

```bash
python3 poc/compiler-convergence/harness.py files   --arm control
python3 poc/compiler-convergence/harness.py read    --arm control transport.ail
python3 poc/compiler-convergence/harness.py write   --arm control transport.ail < new_transport.ail
python3 poc/compiler-convergence/harness.py check   --arm control
python3 poc/compiler-convergence/harness.py publish --arm control
```

Repeat for every other blind trial (`control-t2`, `control-t3`, ...). Once all
of them have finished, rebuild their withheld findings and run the compiler
arms:

```bash
python3 poc/compiler-convergence/harness.py unseal --arm control --operator
```

Then, after the `ail` arms have run:

```bash
python3 poc/compiler-convergence/harness.py report --operator
```

`report` needs `--operator` because it shows both arms, including diagnostics
that were withheld from the control arm.

`report/measures.txt` is the table, `report/measures.json` has the per-attempt
detail, and `runs/<arm>/state.json` is the full ledger: every command, every
edit, every gate call, the compiler output for each, and the final source.

Every command takes `--fixture`. Each fixture keeps its own task specification,
its own reference material list, its own reference repair, and its own
`runs-<fixture>/` and `report-<fixture>/` directories; `cancel-dispatch` keeps
the original `runs/` and `report/` paths. The protocol, the gate, and the
measures are shared, which is what makes two fixtures comparable as experiments
even when their faults are unrelated.

## No compiler output while a blind arm works

Two rules together mean a blind trial has nothing to read: the arms are
staggered, and the blind arm's own ledger is sealed while it runs.
`harness.py self-test` proves both.

### Arm order

Every blind arm of a broken workspace finishes before the first compiler arm of
that workspace starts. The harness enforces both halves and refuses the command
that would break either one:

- A `control` arm cannot start, read, write, check, or publish while any `ail`
  ledger for its broken workspace exists on disk. There is no compiler output
  for a blind trial to find, because the file that would hold it has not been
  written yet.
- An `ail` arm cannot start until every `control` arm of that workspace has
  finished, which means it passed the gate, spent its 40 gate calls, or was
  closed by the operator with `harness.py close --arm <name> --operator
  --reason <why>`.

An `ail` ledger counts against a workspace whichever run directory holds it,
because relevance is the broken workspace's digest, not the directory name.
`label-batch` and `label-batch-frugal` share one broken workspace, so a
compiler arm in either one blocks a blind arm in the other, and an archived run
directory blocks the same way a live one does. That means the committed
fixtures cannot take a new blind trial: their compiler ledgers are in the tree.
Run a new batch under `--runs-root <dir>`, which holds that batch's `runs-*`
and `report-*` directories and is where the order is enforced.

Each arm records what the order looked like when it started: the compiler
ledgers present (`[]` for a blind arm) and, for a compiler arm, when each blind
arm finished. `harness.py audit` rechecks both from the committed ledgers, and
reports arms recorded before this was enforced as exactly that instead of
scoring them.

### Sealed blind ledgers

A control arm's own state file used to record the compiler output the harness
had just withheld from it, so an arm that disobeyed the brief and read its own
ledger saw every finding. The operator canary asked it to disclose that; nothing
stopped it.

A blind arm's gate call now records what the arm was told — the command, the
exit code, the specification violations, the revision-store facts — and its own
source, content-addressed. It records no compiler output, no diagnostic, and no
finding count. There is nothing in the file to read.

`harness.py unseal --arm <name> --operator` rebuilds the withheld findings by
running each stored gate call's own sources through `ailc` again. It refuses
while any blind arm of that workspace is still working, so the findings never
land on disk during a trial, and it refuses to write a reconstruction that does
not reproduce the exit code and revision-store outcome the run recorded, which
is what happens if the compiler or the fixture changed since. `ailc` output is
byte-identical on identical sources, so the rebuilt ledger is the run's ledger.

`report` and `audit` refuse to score a sealed ledger. Counting zero diagnostics
for a run that produced ten would be a partial result reported as complete, so
they name the arm and the `unseal` command instead.

### What the self-test proves

`harness.py self-test` drives the real command line in a temporary runs root, so
the committed runs are untouched and every refusal is the one an operator would
hit. A blind arm starts, fails a gate call on the broken workspace, repairs it
and publishes, and around that:

- No `ail` ledger exists anywhere on disk while it works, and the compiler arm
  is refused mid-trial, admitted once the blind arm finishes, after which the
  blind arm and a new blind arm are both refused.
- Reading the blind arm's own state file finds no `AIL.` code at all, and none
  of the findings that failing gate call produced.
- The compiler arm's ledger does hold those findings, so sealing is the only
  difference between the two files.
- `report` refuses the sealed ledger, `unseal` rebuilds exactly the findings the
  blind arm was never shown, and `report` then scores it.

## Gate semantics

`check` reports whether `ailc check` passed. `publish` reports whether the gate
was satisfied. Both arms are told that bit for the command they ran, because
both arms must be able to tell whether the command they invoked succeeded. Only
the compiler's diagnostics are withheld.

The first run of this harness got that wrong: `check` reported `FAIL` even when
`ailc check` passed, because only `publish` could report `PASS`. The `ail` arm
could see `ok` in the compiler output and ignore the label; the control arm
could not, so a clean workspace looked like a failure to it. That is an
asymmetry beyond the intended variable, so the first run's control number is
not a valid measurement. It is kept in `runs-v1-gate-defect/` with its report,
because the defect and its effect are part of the record.

## Guards against a fake win

- **No fact-applying script.** The harness never edits `.ail` source. It reads,
  writes what an agent gives it, runs `ailc`, and records. `self-test` is the
  only path that applies a repair, it applies a stored reference repair, and it
  is labelled a self-test and excluded from arm results.
- **Policy is immutable.** `harness.py write` rejects `architecture.json`. The
  self-test also confirms that weakening the transport dependency rule does not
  by itself publish the broken fixture, because the type, capability, and
  complexity faults remain.
- **Publish writes nothing on fail.** The workspace exists on disk only while
  `ailc` runs, in a private temporary directory. The harness asserts no
  revision store survives a failed publish and that `check` never writes one.
- **The compiler runs in both arms.** The control arm's gate call runs the same
  `ailc` command against the same checker; its findings are withheld during the
  run and rebuilt from its own sources afterwards, so both ledgers state what
  the same checker said.
- **The blind arms run first, and their ledgers are sealed.** No compiler output
  for a workspace exists on disk while a blind trial works, in a compiler arm's
  ledger or in its own. The harness refuses the commands that would break either
  rule and `self-test` proves both.
- **No new language.** The only compiler change this work needed is that
  `ailc` now prints the architecture facts the checker had already computed
  (ADR 0015). No new syntax, no new capability file, no new command.

## Trust assumption

The harness holds the authoritative workspace in `runs/<arm>/state.json` and
materializes it only for the duration of a gate call, so there is no on-disk
workspace for a control agent to compile behind the harness's back. A control
agent that deliberately reconstructed the workspace from the state file and ran
`ailc` itself would defeat the experiment; the brief forbids it, and each arm's
command log is committed with the run so a reader can audit the trajectory.

Two ways to cheat that no brief can detect after the fact are now closed by
construction rather than by instruction: reading a compiler arm's findings, and
reading the findings the harness withheld from this arm. The `label-batch` run
had both files available — a compiler arm's full findings on the same machine,
and each control arm's own ledger recording what it had just been denied — so
its control number is an upper bound. Neither file holds compiler output during
a blind trial now.

One hole stays open, and no ordering or sealing closes it: an arm that
reconstructs the workspace and runs `ailc` on it defeats the experiment. The
brief forbids it, the state file's operator canary asks a reader to disclose,
and each arm's command log is committed so a reader can audit the trajectory.
