# Compiler convergence proof-of-concept

Results from three trials per arm are in [RESULTS.md](RESULTS.md). Short
version: the compiler arm wins the median on every measure by a small margin,
one control trial beat every compiler trial, and the clean result is variance,
not magnitude. The thesis is not proven by those numbers.

One broken AIL workspace, two agents, one success gate. The arms differ in a
single variable: whether the agent may see what the AIL compiler already knows.

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
both arms; in the control arm it records the diagnostics and refuses to show
them. That is what makes the two arms comparable: the same ledger is built from
the same checker either way.

| measure | how it is computed |
| --- | --- |
| gate calls to pass | index of the first `publish` that passed the gate |
| tokens | protocol tokens between agent and harness, both directions |
| fix cycles | gate calls where a diagnostic appeared that the previous call did not have, that is, breakage introduced by the last edit |
| rebreaks | diagnostics that were fixed and later came back |
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

Prove the fixture is solvable, the gate cannot be gamed, and a failed publish
writes nothing:

```bash
python3 poc/compiler-convergence/harness.py self-test \
  --reference poc/compiler-convergence/fixture/reference-solution
```

`fixture/reference-solution/` is one repair that passes the gate. It is kept
outside the working tree while the arms run, so neither agent can read it, and
committed afterwards so the self-test and the solvability claim are auditable.
Git history shows which commit added it.

Start an arm and hand its brief to an agent:

```bash
python3 poc/compiler-convergence/harness.py start --arm ail
python3 poc/compiler-convergence/harness.py brief --arm ail
```

The agent then works only through the harness:

```bash
python3 poc/compiler-convergence/harness.py files   --arm ail
python3 poc/compiler-convergence/harness.py read    --arm ail transport.ail
python3 poc/compiler-convergence/harness.py write   --arm ail transport.ail < new_transport.ail
python3 poc/compiler-convergence/harness.py check   --arm ail
python3 poc/compiler-convergence/harness.py publish --arm ail
```

Repeat for `--arm control`. Then:

```bash
python3 poc/compiler-convergence/harness.py report --operator
```

`report` needs `--operator` because it shows both arms, including diagnostics
that were withheld from the control arm.

`report/measures.txt` is the table, `report/measures.json` has the per-attempt
detail, and `runs/<arm>/state.json` is the full ledger: every command, every
edit, every gate call, the compiler output for each, and the final source.

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
- **The compiler runs in both arms.** The control arm's diagnostics are
  recorded and withheld, not skipped, so both ledgers are built from the same
  checker.
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
