# Python job-service baseline

This directory contains the M5 typed Python baseline for the frozen UC-001 and
UC-003 job-service benchmark.

- `v1/` is the tested UC-001 checkpoint used as the start of the priority
  evolution task.
- `v2/` is the verified UC-003 reference implementation and benchmark runner.
- `checkpoints.json` freezes both source trees with reproducible SHA-256
  digests.
- `seed-locations.json` identifies stable Python semantic locations for every
  frozen hidden seed category.
- `runner.json` and the locked verification manifest connect V2 to the
  language-neutral M2 harness.

The baseline uses CPython 3.13.5. Runtime behavior uses only the standard
library. Development tools are pinned in `uv.lock`.

## Setup

Install CPython 3.13 and `uv` 0.7.12 or later, then run from this directory:

```bash
uv sync --frozen
```

After the initial sync, all build and verification commands can run without
network access.

## Checks

Run from this directory:

```bash
uv run --frozen ruff format --check .
uv run --frozen ruff check .
uv run --frozen mypy
uv run --frozen pytest
uv run --frozen pytest --cov --cov-report=term-missing
```

Run the shared functional oracle from the repository root:

```bash
python3 benchmarks/tools/harness.py verify \
  --language python \
  --visibility public
```

The tests exercise:

- V1 and V2 field bounds, validation order, and effect-free failures;
- the complete closed result and store-outcome contracts;
- exact one-call persistence and insert-if-absent postconditions;
- all priority identities and unchanged propagation;
- V1 request and persisted-record adaptation to normal priority;
- V1 response projection and V2 persisted encoding;
- boundary decoding and malformed process inputs;
- all 37 shared public fixtures;
- normalized one-case and corpus runner behavior;
- checkpoint source-tree digests; and
- every frozen hidden seed category.

Coverage is branch-aware and must remain at or above 95%.

## Run one case

From this directory:

```bash
python3 -m v2.runner \
  --case benchmarks/fixtures/public/create_job/uc003-v2-created-priority-high.json
```

## Run the corpus

```bash
python3 -m v2.runner --corpus benchmarks/fixtures/manifest.json
```

Each command writes exactly one normalized JSON value to standard output.
Diagnostics are written to standard error.
