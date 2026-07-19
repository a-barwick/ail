# Current status

Last updated: 2026-07-18

## Active milestone

M2 — Benchmark harness and frozen task contract

## Current goal

Freeze the language-neutral runner, result, run-manifest, task, hidden-behavior,
and failure-classification contracts used by every baseline.

The next agent should build:

- the command contract for running one public case or the full corpus;
- the common JSON result and locked run-manifest schemas;
- final UC-001 implementation and UC-003 evolution task text;
- language-independent hidden behavior and seed-category rules; and
- a dependency-free harness self-test covering pass, fail, malformed result,
  timeout, and changed or incomplete locked manifests.

## Starting point

The following work is accepted and should not be redesigned in M2:

- [UC-001 request validation and persistence](use-cases/UC-001-request-validation-and-persistence.md)
- [UC-003 public schema evolution](use-cases/UC-003-public-schema-evolution.md)
- [Initial requirements](requirements/reference-slice.md)
- [Benchmark policy](benchmarks/README.md)
- [JSON fixture format](benchmarks/job-service-fixtures.md)
- [Public fixture artifact guide](../benchmarks/README.md)
- [ADR 0001 deferring stack selection](decisions/0001-implementation-stack.md)

Key decisions:

- public correctness is the response, final state, and ordered-call oracle
  frozen by M1;
- a run starts only from a complete locked manifest;
- every edit followed by an incomplete validation result counts as a repair
  cycle;
- hidden inputs may combine only accepted UC-001 and UC-003 behavior;
- language-specific seeded locations are instantiated later without changing
  M2 behavior; and
- the harness remains dependency-free benchmark tooling, not compiler code.

## Completed

- M0 — Reference workload and requirements
- M1 — Frozen job-service fixture corpus
- Documentation alignment checkpoint: M0–M13 is the sole operational roadmap,
  old phase labels have been retired, and accepted requirements point to
  numbered milestones.

M1 delivered 37 canonical public JSON cases, a machine-readable schema, a
dependency-free semantic checker and formatter, negative-path tool tests, and a
SHA-256 manifest with complete UC-001/UC-003 traceability. Run
`python3 benchmarks/tools/fixtures.py check` to verify the frozen corpus.

## Planned next

- M3–M6 — Rust, Go, Python, and TypeScript baselines, in that default serial
  order unless coordinated otherwise
- M7 — Cross-baseline parity and freeze

## Proposed future validation

[UC-007 architectural regression control](use-cases/UC-007-architectural-regression-control.md),
its [proposed requirements](requirements/architectural-health.md), and the
[architectural health manifest](architecture-health.md) define a later scaling
gate. They do not expand M1 or authorize implementation before review and
acceptance.

## Do not start yet

- AIL syntax design
- AIL compiler implementation
- Compiler-stack prototypes
- Agent benchmark runs
- Production performance targets

Those depend on later milestones.

## Blockers

None.

## Handoff checklist

After meaningful work:

- update this file with what changed and what remains;
- keep harness behavior traceable to the accepted benchmark requirements;
- add or update executable checks;
- record any unresolved behavior instead of choosing it in code;
- run the active milestone's verification commands;
- run `python3 tools/check_docs.py`; and
- update the roadmap only when the milestone exit criterion passes.
