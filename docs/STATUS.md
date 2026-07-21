# Current status

Last updated: 2026-07-20

## Active milestone

M16 — Rust revision protocol and validated rename

## Current goal

Implement the first revision-safe compiler protocol surface over the completed
M15 semantic layer. The execution path is:

```text
M11 five-construct contract (complete)
  -> ADR 0004 Rust decision (accepted)
  -> M14 lossless syntax and formatter (complete)
  -> M15 static semantics and diagnostics (complete)
  -> M16 revision protocol and validated rename (active)
  -> M17 deterministic core interpreter
```

M12 and M13 are superseded. Do not build TypeScript compiler semantics or a
candidate scorecard.

The next agent should:

- build immutable revision storage around the M15 semantic-checking API rather
  than reparsing canonical text;
- assign deterministic revision-scoped handles for declarations, syntax, and
  expressions, and expose elaborated inspection results;
- implement atomic validated rename for the complete M11 source unit,
  including canonical edits and complete identity mapping;
- reject stale base revisions, invalid handles, invalid names, and collisions
  without publishing a partial revision;
- preserve M14 losslessness and formatting plus M15 type facts, diagnostics,
  deterministic ordering, and CLI commands; and
- keep execution, broader constructs, general semantic diffs, and other
  refactors outside M16.

## M14 result

M14 created the production root Cargo workspace and `ail-compiler` crate. It
delivered:

- a lossless lexer whose token spans partition every UTF-8 source byte;
- typed syntax for records, closed variants, functions, local bindings, and
  capability calls;
- deterministic missing-colon recovery with
  `AIL.PARSE.EXPECTED_TOKEN`;
- canonical declaration, expression, literal, effect, and record-field
  formatting;
- idempotent formatting across every parseable M11 fixture;
- an `ailc` command with `check`, `format`, and `reconstruct`; and
- seven focused conformance tests backed by all seven M11 fixture inputs and
  the three CLI operations.

## M15 result

M15 extended the existing M14 typed syntax tree with deterministic semantic
checking. The compiler now resolves top-level and function-local names; checks
exact named types for records, closed variants, function results, local
bindings, and capability calls; accepts capability interfaces as compiler
input; verifies declared effects; and returns inferred local types with
structured diagnostics.

The M11 type-error and capability-error fixtures produce
`AIL.TYPE.FIELD_MISMATCH` and `AIL.CAPABILITY.UNDECLARED_EFFECT` with their
required expected/actual facts, related identities, and causal chains. Parsing
still blocks static checking. The new `check_source` API is deliberately
revision-input-only; immutable revision storage, inspection requests, rename,
and identity maps are M16 work.

## Stack decision

[ADR 0004](decisions/0004-rust-compiler-stack.md) selects Rust as the
authoritative compiler language through the first production backend. It
supersedes ADR 0001 and the M12/M13 comparison path. The decision accepts Rust's
ownership, compile-time, and contributor-learning risks and manages them in the
production compiler rather than through a disposable TypeScript implementation.

## M11 contract

M11 delivered exactly five constructs:

1. records;
2. closed variants;
3. functions with explicit public signatures;
4. local `let` inference; and
5. capability operation calls.

The contract contains 24 numbered proposed language and protocol rules, seven
canonical fixture categories, nine transport-independent protocol shapes, and
a dependency-free checker. The M11 subset remains proposed language material,
but it is the fixed conformance boundary for M14 through M16.

## Accepted foundation

- UC-001 request validation and conditional persistence
- UC-003 public and persisted schema evolution
- Accepted `APP-*`, `LANG-*`, `PROTO-*`, and benchmark requirements
- 37 public behavior fixtures and five separately held private behaviors
- Rust, Go, Python, and TypeScript V1/V2 reference implementations
- Cross-language normalized parity
- Eight digest-locked answer-free task starts
- M8a–M8f calibration infrastructure and non-official pilot evidence
- M11 language/protocol contract and fixtures
- ADR 0004 Rust compiler decision
- M14 Rust lossless syntax and canonical formatter
- M15 Rust static semantics and diagnostics

## Completed

- M0 — Reference workload and requirements
- M1 — Frozen job-service fixture corpus
- M2 — Benchmark harness and frozen task contract
- M3 — Rust baseline
- M4 — Go baseline
- M5 — Python baseline
- M6 — TypeScript baseline
- M7 — Cross-baseline parity and freeze
- M8a–M8f — Calibration preparation and readiness infrastructure
- M11 — Five-construct language and protocol contract
- M14 — Rust lossless syntax and canonical formatter
- M15 — Rust static semantics and diagnostics

## Superseded

- M12 — Comparable Rust and TypeScript compiler spikes
- M13 — Separate compiler-stack decision milestone

## Deferred

- M8g–M8o — Official baseline calibration campaign
- M9 — Numeric AIL benchmark targets
- M10 — Illustrative AIL variants
- UC-007 — Architectural regression control

These items require an explicit maintainer decision to resume. They do not
block compiler implementation.

## Do not start yet

- The broader 20–30 construct language core before its rules are accepted
- Native code generation, production runtime work, or general concurrency
- Official agent or performance evidence

## Blockers

None.

## Handoff checklist

After meaningful work:

- keep compiler behavior within the active milestone and numbered M11 rules;
- add executable tests for every delivered behavior;
- run `cargo fmt --all --check`;
- run `cargo test --workspace`;
- run `cargo clippy --workspace --all-targets -- -D warnings`;
- run `python3 specs/tools/core_contract.py check`;
- run `python3 tools/check_docs.py`; and
- update this file and the roadmap only when the active milestone's exit
  criterion passes.
