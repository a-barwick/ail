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

The completed M3 Rust implementation and its setup, build, test, and run
instructions are in [`baselines/rust/`](baselines/rust/README.md).

## Public and hidden boundary

Only public behavior cases belong under `fixtures/public/`. Later milestones
instantiate and package hidden combinations and seeded language-specific
consumers outside that tree. The deterministic hidden ZIP is readable by the
harness but outside agent-readable paths. Hidden inputs may exercise only
behavior already accepted by UC-001 or UC-003, and their answers must not be
disclosed by this public corpus, contract, or manifest.
