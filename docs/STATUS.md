# Current status

Last updated: 2026-07-18

## Active milestone

M3 — Rust baseline

## Current goal

Build the first idiomatic baseline against the frozen fixtures and harness:
stable Rust implementing UC-001 and UC-003.

The next agent should build:

- a Rust version-one checkpoint suitable for the UC-003 change task;
- a verified version-two reference implementation;
- the M2 one-case and corpus runner commands;
- idiomatic focused tests for public contracts, types, closed outcomes, and
  ordered effects; and
- plain setup, format, lint, build, test, and run instructions.

## Starting point

The following work is accepted and should not be redesigned in M3:

- [UC-001 request validation and persistence](use-cases/UC-001-request-validation-and-persistence.md)
- [UC-003 public schema evolution](use-cases/UC-003-public-schema-evolution.md)
- [Initial requirements](requirements/reference-slice.md)
- [Benchmark policy](benchmarks/README.md)
- [JSON fixture format](benchmarks/job-service-fixtures.md)
- [Executable benchmark artifact guide](../benchmarks/README.md)
- [Runner contract](../benchmarks/contracts/runner-contract.md)
- [Run and repair classifications](../benchmarks/contracts/run-classification.md)
- [Hidden behavior and seed contract](../benchmarks/contracts/hidden-contract.json)
- [Final UC-001 task](../benchmarks/tasks/uc001-implement-create-job.md)
- [Final UC-003 task](../benchmarks/tasks/uc003-add-priority.md)
- [ADR 0001 deferring stack selection](decisions/0001-implementation-stack.md)

Key decisions:

- the shared fixtures, task text, runner protocol, normalized result, and
  failure classifications are frozen;
- the Rust baseline receives its normal stable compiler, Cargo, rustfmt,
  Clippy, rust-analyzer, and ordinary tests;
- the baseline may use idiomatic Rust representations but must not change the
  shared oracle;
- V1 and V2 source checkpoints must remain distinct and digestible; and
- concrete hidden seed locations instantiate the M2 semantic roles without
  changing or disclosing their behavior.

## Completed

- M0 — Reference workload and requirements
- M1 — Frozen job-service fixture corpus
- M2 — Benchmark harness and frozen task contract
- Documentation alignment checkpoint: M0–M13 is the sole operational roadmap,
  old phase labels have been retired, and accepted requirements point to
  numbered milestones.

M1 delivered 37 canonical public JSON cases, a machine-readable schema, a
dependency-free semantic checker and formatter, negative-path tool tests, and a
SHA-256 manifest with complete UC-001/UC-003 traceability. Run
`python3 benchmarks/tools/fixtures.py check` to verify the frozen corpus.

M2 delivered the language-neutral one-case and corpus runner protocol, common
result and run-manifest schemas, final task text, run/repair classifications,
hidden behavior and seed rules, a 13-artifact contract lock, and a self-test
that proves 14 pass and failure outcomes. Run
`python3 benchmarks/tools/harness.py self-test` to verify the contract.

## Planned next

- M4–M6 — Go, Python, and TypeScript baselines, in that default serial order
  unless coordinated otherwise
- M7 — Cross-baseline parity and freeze

## Proposed future validation

[UC-007 architectural regression control](use-cases/UC-007-architectural-regression-control.md),
its [proposed requirements](requirements/architectural-health.md), and the
[architectural health manifest](architecture-health.md) define a later scaling
gate. They do not expand M3 or authorize implementation before review and
acceptance.

## Do not start yet

- AIL syntax design
- AIL compiler implementation
- Compiler-stack prototypes
- Agent benchmark runs
- Production performance targets
- Go, Python, or TypeScript baselines without explicit coordination

Those depend on later milestones.

## Blockers

None.

## Handoff checklist

After meaningful work:

- update this file with what changed and what remains;
- keep Rust behavior traceable to the frozen shared fixtures and tasks;
- add or update executable checks;
- record any unresolved behavior instead of choosing it in code;
- run the active milestone's verification commands;
- run `python3 tools/check_docs.py`; and
- update the roadmap only when the milestone exit criterion passes.
