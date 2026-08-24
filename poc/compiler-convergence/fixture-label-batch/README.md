# Fixture `label-batch`: type, capability, and effect faults

The second fixture for the two-arm harness in [../harness.py](../harness.py).

**Outcome: the blind arm won this fixture**, 1 retry against 2 over seven trials
per arm, and both arms tied at 0 retries when told gate calls are expensive. The
numbers and why are in [../RESULTS-label-batch.md](../RESULTS-label-batch.md).

This fixture exists to answer one objection to the first one
([../fixture](../fixture)): that its measured gap came from an architecture
complexity budget, a number a blind agent can recompute by hand from
`architecture.json` and `specs/architecture.md`. This workspace has **no
`architecture.json` at all**, so the architecture checker never runs and no
`AIL.ARCH.*` finding is reachable. Every fault is a type, capability, or effect
fault, and the compiler is the only thing in the environment that states which.

The protocol, the gate, and the measures are the harness's, unchanged: one
broken workspace, two arms, `ailc publish` as the success gate, compiler output
returned verbatim to arm `ail` and withheld from arm `control`. Neither arm
rewrites the task in another language.

## The workspace

Six modules classifying a bounded batch of label requests:

| module | role |
| --- | --- |
| `types.ail` | public types: `LabelRequest`, `Label`, `LabelReport`. Immutable. |
| `rules.ail` | pure predicates over one request |
| `classify.ail` | one request to one `Label` |
| `batch.ail` | `map` over `List<LabelRequest, 8>` |
| `report.ail` | note text per label, and `map` to `List<LabelReport, 8>` |
| `tests.ail` | entry point `label_batch` over the whole pipeline |

`types.ail` is immutable, so the faults must be repaired at their use sites: an
arm cannot delete a variant payload or a record field to make a call site type
check. That is this fixture's analogue of the first fixture's immutable
`architecture.json`.

## The seven faults

1. `batch.classify_batch` returns `List<types.Label, 4>` while its `map` over an
   8-bounded list produces `List<types.Label, 8>`
   (`AIL.TYPE.RESULT_MISMATCH`). The wrong bound also mistypes
   `tests.label_batch`'s call into `report.report_batch`
   (`AIL.TYPE.FUNCTION_ARGUMENT`).
2. `batch.classify_batch` declares `effects { journal.write }` and has no
   `journal` parameter (`AIL.CAPABILITY.INVALID_EFFECT`). The stale effect
   propagates to its caller (`AIL.CAPABILITY.UNDECLARED_TRANSITIVE_EFFECT`).
3. `classify.classify_one` passes `request.raw` to `rules.is_illegal`, which
   takes a `LabelRequest` (`AIL.TYPE.FUNCTION_ARGUMENT`).
4. `classify.classify_one` builds `types.Label::Illegal` with no `Text` payload
   (`AIL.TYPE.VARIANT_PAYLOAD_MISMATCH`).
5. `report.note_for` takes `journal: capability AuditJournal` and declares
   `effects { journal.write }`. No capability environment supplies that
   interface (`AIL.CAPABILITY.UNKNOWN_INTERFACE`), so both call chains through
   it fail as well (`AIL.CAPABILITY.MISSING_TRANSITIVE_CAPABILITY` and three
   `AIL.CAPABILITY.UNDECLARED_TRANSITIVE_EFFECT`).
6. `report.note_for`'s `Illegal` arm binds no payload
   (`AIL.TYPE.MATCH_BINDING`).
7. `report.report_one` builds a `LabelReport` without its `label` field
   (`AIL.TYPE.RECORD_FIELD_SET`).

`ailc check` reports all thirteen findings on the broken fixture in one call,
across four files and two categories. There is no staging: arm `ail` sees the
whole failure set immediately, and arm `control` sees `FAIL`. That dump is 1,111
protocol tokens, which is what the compiler arm pays for it.

Every one of the seven faults is visible at its own site by reading the six
files, which is why the blind arm repaired all of them in one pass and won the
win condition. A fault set that only the compiler can characterize would have to
be non-local.

Fault 5 carries the trap. Its obvious local repair is to pass the capability up:
give `report_one` and `report_batch` a `journal` parameter and declare the
effect. That satisfies the two call-site findings and keeps
`AIL.CAPABILITY.UNKNOWN_INTERFACE` on three functions instead of one, and the
task specification pins `tests.label_batch`'s signature so the capability cannot
reach the entry point either. The only shape that publishes is a capability-free
chain. `negative-controls/thread-capability-up/` is that plumbing repair, and the
self-test asserts it fails the compiler while satisfying the specification.

## The task specification does not point at a fault

Every requirement in [contract.json](contract.json) already holds in the broken
fixture. The self-test asserts it:

```
ok   broken fixture satisfies the task specification, so a violation never points at a fault
```

That closes a hole in the first fixture, where five specification rules were
violated by the broken source and every bisecting control trial used the
violation list as a free oracle. Here the specification only forbids repairs
that throw behavior away, and the compiler is the only source of information
about what is wrong.

## Running it

Build the compiler once:

```bash
cargo +1.87.0 build --release -p ail-compiler --bin ailc
```

Prove the fixture is solvable, the gate cannot be gamed, and the failure classes
are what this fixture claims:

```bash
python3 poc/compiler-convergence/harness.py self-test --fixture label-batch
```

Run an arm:

```bash
python3 poc/compiler-convergence/harness.py start   --fixture label-batch --arm ail
python3 poc/compiler-convergence/harness.py brief   --fixture label-batch --arm ail
python3 poc/compiler-convergence/harness.py files   --fixture label-batch --arm ail
python3 poc/compiler-convergence/harness.py read    --fixture label-batch --arm ail report.ail
python3 poc/compiler-convergence/harness.py write   --fixture label-batch --arm ail report.ail < new_report.ail
python3 poc/compiler-convergence/harness.py check   --fixture label-batch --arm ail
python3 poc/compiler-convergence/harness.py publish --fixture label-batch --arm ail
```

Repeat with `--arm control`, and with trial names such as `ail-t2` and
`control-t2`. Then:

```bash
python3 poc/compiler-convergence/harness.py report --fixture label-batch --operator
```

Runs land in `runs-label-batch/<arm>/state.json` and the report in
`report-label-batch/`. Audit the finished runs against the claims a reader should
not have to take on trust:

```bash
python3 poc/compiler-convergence/harness.py audit --fixture label-batch
```

`--fixture label-batch-frugal` is the same broken workspace and the same brief
with its own runs directory. It holds the secondary condition, in which the
operator added one instruction worded identically for both arms: gate calls are
expensive, use as few as you can.

## Guards against a fake win

- **No fact-applying script.** The harness never edits `.ail` source. `self-test`
  is the only path that applies a repair, it applies the stored reference repair,
  and it is excluded from arm results.
- **No architecture policy to empty or edit.** This workspace has no
  `architecture.json`. The self-test asserts that, and asserts no architecture
  finding is reachable.
- **`types.ail` is immutable.** `harness.py write` rejects it, and the self-test
  asserts the rejection and that the reference repair leaves it byte-identical.
- **Negative controls.** Three stored candidate repairs that must not pass:
  plumbing the capability upward (compiler denies), bypassing the pipeline in
  `tests.ail` (specification denies), and collapsing `note_for` to a constant
  (specification denies).
- **Publish writes nothing on fail.** The workspace exists on disk only while
  `ailc` runs, in a private temporary directory, and the self-test asserts no
  revision store survives a failed publish or a check.
- **The compiler runs in both arms.** The control arm's diagnostics are recorded
  and withheld, not skipped.
