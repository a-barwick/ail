# Benchmark artifacts

This directory contains executable, language-independent inputs for the
job-service benchmark. The artifacts describe accepted application behavior;
they are not AIL source and do not define AIL syntax.

## Public fixture corpus

`fixtures/public/` contains one JSON case per file. The cases exercise the
accepted UC-001 request-validation and persistence behavior and the UC-003
priority evolution, including the complete UC-001 regression matrix against
service version 2.

`fixtures/manifest.json` records every public fixture and its SHA-256 digest,
traceability, and coverage tags. `schemas/job-service-fixture.schema.json`
defines the machine-readable file shape.

Run the complete fixture gate with:

```bash
python3 benchmarks/tools/fixtures.py check
```

The command validates:

- JSON schema and canonical two-space formatting;
- explicit request, response, and stored-record versions;
- canonical padded Base64 payloads;
- request bounds and validation-issue ordering;
- response, final state, and ordered store calls;
- required UC-001 and UC-003 public-case coverage; and
- the schema and fixture digests in the manifest.

To format fixture files or check the manifest independently:

```bash
python3 benchmarks/tools/fixtures.py format
python3 benchmarks/tools/fixtures.py manifest --check
```

`format` intentionally does not rewrite the manifest because formatting a case
changes its digest. After an intentional reviewed fixture change, regenerate
the manifest with `manifest --write`, review the resulting traceability, and
run the complete check.

## Runner and task contract

M2 freezes the process boundary shared by every baseline:

- [runner-contract.md](contracts/runner-contract.md) defines one-case and corpus
  commands;
- [runner-result.schema.json](schemas/runner-result.schema.json) defines
  normalized functional output;
- [run-manifest.schema.json](schemas/run-manifest.schema.json) records source,
  task, tests, tools, model, environment, permissions, limits, retries, token
  accounting, repair policy, evidence, and artifact locks;
- [run-classification.md](contracts/run-classification.md) defines validation
  attempts, repair cycles, successful runs, failures, timeouts, and the
  pre-start manifest gate;
- [the UC-001 task](tasks/uc001-implement-create-job.md) and
  [the UC-003 task](tasks/uc003-add-priority.md) contain the final task text;
  and
- [hidden-contract.json](contracts/hidden-contract.json) freezes
  language-independent hidden behavior and seeded semantic roles without
  publishing concrete hidden inputs, locations, or answers.

The contract is locked by SHA-256 in
`benchmarks/contracts/contract-lock.json`. Verify the complete harness and
contract with:

```bash
python3 benchmarks/tools/harness.py self-test
```

The self-test proves that a fake conforming implementation is accepted in
one-case and corpus modes. It also seeds response, final-state, ordered-call,
coverage, manifest, schema, process, and timeout failures and verifies their
stable classifications. Changed or incomplete locked run manifests are
rejected before the implementation subprocess starts.

Baseline milestones add:

```text
benchmarks/baselines/<language>/
  runner.json
  verification-manifest.json
  verification-manifest.lock.json
```

That layout makes the roadmap verification command stable:

```bash
python3 benchmarks/tools/harness.py verify \
  --language <language> \
  --visibility public
```

The completed Rust, Go, Python, and TypeScript implementations and their setup,
build, test, coverage, and run instructions are under `baselines/`.

## M7 parity freeze

M7 verifies the four baseline runners together. It checks the locked V1 and V2
source trees, every seed-location category, and the normalized response, final
state, and ordered store calls for every public and private case. The committed
[parity report](m7-parity-report.json) records the four locked tool sets and
one non-reversible digest per private case; it does not publish private case
paths, inputs, or expected values. [m7-freeze.json](m7-freeze.json) locks the
report, manifests, runner descriptors, source checkpoints, task text, and
contract inputs by digest.

M7 also freezes eight independently reproducible
[answer-free task starts](task-starts/README.md). UC-001 starts contain the
selected language's public V1 contracts and ordinary tests with explicit
handler and validation holes. UC-003 starts contain the accepted V1
implementation and ordinary V2 task tests without V2 implementation source.
Every package has an explicit per-file manifest and its own tree digest. The
standalone package gate rejects another baseline language, private fixtures,
hidden seed locations, completed reference source, freeze-only metadata,
changed protected artifacts, nondeterministic rebuilds, and an incomplete
language/task matrix:

```bash
python3 benchmarks/tools/task_starts.py check --run-starting-state
```

The private ZIP is deliberately not stored in the repository. A verifier must
receive the digest-locked archive through `AIL_HIDDEN_PACKAGE` (or the
equivalent `--hidden-package` option):

```bash
export AIL_HIDDEN_PACKAGE=/secure/path/ail-job-service-m7-hidden.zip
python3 benchmarks/tools/harness.py verify-all
```

The ZIP is read only by the harness, extracted beneath an ignored temporary
directory for the individual runner calls, and removed immediately afterwards.
It must contain one canonical fixture for each frozen hidden behavior category,
use stored ZIP entries in lexical order with the fixed 1980 timestamp, and
match the archive digest in every locked baseline manifest.

## M8 agent experiment contract

[ADR 0002](../docs/decisions/0002-m8-agent-experiment-contract.md) freezes the
candidate measured agent, model, interactive tool-use protocol, prompt, initial
context, permissions, limits, token accounting, reference environment, and
terminal classifications used by M8. It also records the reviewed NFR-002
amendment from the infeasible 100,000-token pilot limit to a 500,000 cumulative
delivered-input-token safety limit.

The decision is a configuration contract, not official evidence. M8b encodes
it in the locked
[calibration evidence contract](calibration/README.md), eight JSON schemas, and
`verify-calibration`. The verifier accepts structurally complete empty, pilot,
and partial campaigns, requires final counts for a campaign marked complete,
and rejects changed, missing, inconsistent, mixed, or incorrectly summarized
evidence. M8c implements and dry-tests the interactive runner, generated
least-privilege configuration, pre-start gate, event and final-source capture,
limit enforcement, and M2 activity accounting. M8d adds final-revision
public/private correctness, seeded-role checks, protected-artifact checks,
revision-bound completion evidence, and fresh functional replay. M8e adds
performance measurement, and M8f must pass all readiness configurations before
any official trial counts.

Run the M8b gate with:

```bash
python3 benchmarks/tools/harness.py verify-calibration
```

## Public and hidden boundary

Only public behavior cases belong under `fixtures/public/`. Later milestones
instantiate and package hidden combinations and seeded language-specific
consumers outside that tree. The deterministic hidden ZIP is readable by the
harness but outside agent-readable paths. Hidden inputs may exercise only
behavior already accepted by UC-001 or UC-003, and their answers must not be
disclosed by this public corpus, contract, or manifest.
