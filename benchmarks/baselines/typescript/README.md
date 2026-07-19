# TypeScript job-service baseline

This directory contains the M6 strict TypeScript baseline for the frozen UC-001
and UC-003 job-service benchmark.

- `v1/` is the tested UC-001 checkpoint used as the start of the priority
  evolution task.
- `v2/` is the verified UC-003 reference implementation and benchmark runner.
- `checkpoints.json` freezes both source trees with reproducible SHA-256
  digests.
- `seed-locations.json` identifies stable TypeScript semantic locations for
  every frozen hidden seed category.
- `runner.json` and the locked verification manifest connect V2 to the
  language-neutral M2 harness.

The baseline uses Node.js 23.10.0 and TypeScript 5.8.3. Dependencies and
development tools are pinned by `package-lock.json`.

## Setup

Install Node.js 23.10.0, then run from this directory:

```bash
npm ci
```

After the initial install, all build and verification commands can run without
network access.

## Checks

Run from this directory:

```bash
npm run format:check
npm run lint
npm run build
npm test
npm run coverage
```

Run the shared functional oracle from the repository root:

```bash
python3 benchmarks/tools/harness.py verify \
  --language typescript \
  --visibility public
```

The tests exercise:

- V1 and V2 field bounds, Unicode-scalar validation, ordering, and effect-free
  failures;
- discriminated closed results and store outcomes under strict type checking;
- exact one-call persistence and defensive byte-array state snapshots;
- all priority identities and unchanged propagation;
- V1 request and persisted-record adaptation to normal priority;
- V1 response projection and V2 persisted encoding;
- malformed boundary and process inputs;
- all 37 shared public fixtures;
- normalized one-case and corpus runner behavior;
- checkpoint source-tree digests; and
- every frozen hidden seed category.

Coverage checks enforce at least 95% line, function, and statement coverage and
90% branch coverage over the V1 and V2 service implementation.

## Run one case

From this directory:

```bash
node_modules/.bin/tsx v2/runner.ts \
  --case benchmarks/fixtures/public/create_job/uc003-v2-created-priority-high.json
```

## Run the corpus

```bash
node_modules/.bin/tsx v2/runner.ts \
  --corpus benchmarks/fixtures/manifest.json
```

Each command writes exactly one normalized JSON value to standard output.
Diagnostics are written to standard error.
