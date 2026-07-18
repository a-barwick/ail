# Current status

Last updated: 2026-07-18

## Active milestone

M1 — Frozen job-service fixture corpus

## Current goal

Turn the accepted job-service behavior into executable, language-independent
JSON test data.

The next agent should build:

- the public UC-001 and UC-003 JSON cases;
- a machine-readable fixture schema;
- a dependency-free validator and formatter;
- a manifest containing every fixture path and SHA-256 digest; and
- one command that checks the schema, formatting, manifest, and required case
  coverage.

## Starting point

The following work is accepted and should not be redesigned in M1:

- [UC-001 request validation and persistence](use-cases/UC-001-request-validation-and-persistence.md)
- [UC-003 public schema evolution](use-cases/UC-003-public-schema-evolution.md)
- [Initial requirements](requirements/reference-slice.md)
- [Benchmark policy](benchmarks/README.md)
- [JSON fixture format](benchmarks/job-service-fixtures.md)
- [ADR 0001 deferring stack selection](decisions/0001-implementation-stack.md)

Key decisions:

- invalid requests never call storage;
- valid requests call storage exactly once;
- V1 jobs and requests convert to normal priority under V2;
- V1 requests receive V1 responses;
- V2 requests receive V2 responses;
- fixture files use JSON with explicit version fields; and
- benchmark tooling is separate from the AIL compiler implementation.

## Completed

- M0 — Reference workload and requirements
- Documentation alignment checkpoint: M0–M13 is the sole operational roadmap,
  old phase labels have been retired, and accepted requirements point to
  numbered milestones.

## Planned next

- M2 — Benchmark harness and frozen task contract
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
- keep fixture behavior traceable to UC-001 or UC-003;
- add or update executable checks;
- record any unresolved behavior instead of choosing it in code;
- run the active milestone's verification commands;
- run `python3 tools/check_docs.py`; and
- update the roadmap only when the milestone exit criterion passes.
