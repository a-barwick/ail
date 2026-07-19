# Current status

Last updated: 2026-07-19

## Active milestone

M7 — Cross-baseline parity and freeze

## Current goal

Run the completed Rust, Go, Python, and TypeScript implementations through one
parity gate, instantiate the frozen hidden seed categories consistently, and
freeze the V1 task starts and V2 reference results by digest before M8 records
agent or performance measurements.

The next agent should:

- provision the frozen Go 1.26, CPython 3.13.5, and Node.js 23.10 toolchains;
- run `python3 benchmarks/tools/harness.py verify-all` with the separately
  held `AIL_HIDDEN_PACKAGE`; and
- mark M7 complete and advance to M8 only after all four real baselines pass.

## Starting point

The following work is accepted and should not be redesigned in M7:

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
- [Rust baseline](../benchmarks/baselines/rust/README.md)
- [Go baseline](../benchmarks/baselines/go/README.md)
- [Python baseline](../benchmarks/baselines/python/README.md)
- [TypeScript baseline](../benchmarks/baselines/typescript/README.md)
- [ADR 0001 deferring stack selection](decisions/0001-implementation-stack.md)

Key decisions:

- shared fixtures, task text, runner protocol, normalized results, and failure
  classifications remain frozen;
- each baseline has distinct digest-locked V1 and V2 source checkpoints;
- normal language tooling remains available without custom semantic advantages;
- hidden cases may combine only already accepted UC-001 and UC-003 behavior;
  and
- M7 resolves baseline differences without weakening the accepted oracle.

## Completed

- M0 — Reference workload and requirements
- M1 — Frozen job-service fixture corpus
- M2 — Benchmark harness and frozen task contract
- M3 — Rust baseline
- M4 — Go baseline
- M5 — Python baseline
- M6 — TypeScript baseline

M1 delivered 37 canonical public JSON cases, a machine-readable schema, a
dependency-free semantic checker and formatter, negative-path tool tests, and a
SHA-256 manifest with complete UC-001/UC-003 traceability.

M2 delivered the language-neutral one-case and corpus runner protocol, common
result and run-manifest schemas, final task text, run/repair classifications,
hidden behavior and seed rules, a 13-artifact contract lock, and a self-test
that proves 14 pass and failure outcomes.

M3 and M4 delivered the distinct Rust and Go V1/V2 baselines, ordinary language
tools and tests, frozen checkpoints, hidden seed locations, and verified shared
runner results.

M5 delivered a typed CPython 3.13 baseline with `uv`-locked development tools,
strict mypy, Ruff, pytest, branch-aware coverage, 61 focused and integration
tests, frozen V1/V2 checkpoints, and all 37 public fixtures accepted by the
shared harness.

M6 delivered a strict TypeScript 5.8 baseline on Node.js 23, a locked dependency
tree, TypeScript, ESLint, Prettier, the ordinary Node test runner, c8 coverage,
57 focused and integration tests, frozen V1/V2 checkpoints, and all 37 public
fixtures accepted by the shared harness.

M7 now provides the candidate-neutral `verify-all` gate, validates both locked
source checkpoints and every frozen seed location for each baseline, verifies
the response, final state, and ordered store calls for public and digest-locked
private cases, and compares all normalized results across languages. It also
publishes a 42-case parity report (37 public and 5 private) and a 28-artifact
freeze lock. The private ZIP is outside the repository and is identified only
by its SHA-256 digest in the locked manifests.

## Planned next

- M8 — Baseline agent calibration
- M9 — Frozen AIL success targets

## Proposed future validation

[UC-007 architectural regression control](use-cases/UC-007-architectural-regression-control.md),
its [proposed requirements](requirements/architectural-health.md), and the
[architectural health manifest](architecture-health.md) define a later scaling
gate. They do not expand M7 or authorize implementation before review and
acceptance.

## Do not start yet

- AIL syntax design
- AIL compiler implementation
- Compiler-stack prototypes
- Agent benchmark runs
- Production performance targets

Those depend on later milestones.

## Blockers

The local verification host has Rust 1.88.0, but does not have the frozen Go
1.26, CPython 3.13.5, or Node.js 23 toolchains. The M7 harness unit suite and
the Rust public-and-private parity run pass; the remaining three real baseline
runs require those provisioned tools before M7 can meet its exit criterion.

## Handoff checklist

After meaningful work:

- update this file with what changed and what remains;
- preserve every frozen fixture, task, protocol, and oracle;
- add or update executable checks;
- record behavioral differences instead of silently weakening the oracle;
- run the active milestone's verification commands;
- run `python3 tools/check_docs.py`; and
- update the roadmap only when the milestone exit criterion passes.
