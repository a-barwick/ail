# Current status

Last updated: 2026-07-19

## Active milestone

M15 — Rust static semantics and diagnostics

## Current goal

Add the first semantic layer to the authoritative Rust compiler. The execution
path is:

```text
M11 five-construct contract (complete)
  -> ADR 0004 Rust decision (accepted)
  -> M14 lossless syntax and formatter (complete)
  -> M15 static semantics and diagnostics (active)
  -> M16 revision protocol and validated rename
  -> M17 deterministic core interpreter
```

M12 and M13 are superseded. Do not build TypeScript compiler semantics or a
candidate scorecard.

The next agent should:

- extend the existing typed syntax tree rather than reparsing canonical text;
- build deterministic top-level and function-local symbol tables;
- implement exact named-type checks for records, variants, functions, locals,
  and capability calls;
- accept capability interfaces as compiler input and verify declared effects;
- return inferred local types and the exact structured diagnostics required by
  the M11 fixtures;
- preserve M14 losslessness, spans, recovery, formatter behavior, and CLI
  commands; and
- keep revisions, rename, execution, broader constructs, and lowering outside
  M15.

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

The compiler intentionally performs no name or type checking yet. The
capability-error and type-error fixtures parse and format in M14; M15 must
produce their declared semantic diagnostics.

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

- M16 revision transactions or rename before M15 passes
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
- update this file and the roadmap only when the M15 exit criterion passes.
