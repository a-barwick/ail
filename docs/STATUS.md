# Current status

Last updated: 2026-07-21

## Active milestone

M20 — Workspace semantic graph and impact query

## Current goal

Implement the accepted M19 source-set, identity, semantic-graph, and impact
contract in the authoritative Rust compiler. The execution path is:

```text
M11 five-construct contract (complete)
  -> ADR 0004 Rust decision (accepted)
  -> M14 lossless syntax and formatter (complete)
  -> M15 static semantics and diagnostics (complete)
  -> M16 revision protocol and validated rename (complete)
  -> M17 deterministic core interpreter (complete)
  -> M18 next validation-slice selection (complete)
  -> M19 UC-003 schema-evolution contract (complete)
  -> M20 workspace semantic graph and impact query (active)
  -> M21 atomic schema evolution and completion evidence (planned)
```

M12 and M13 are superseded. Do not build TypeScript compiler semantics or a
candidate scorecard.

The next agent should:

- implement only the M20 source-set revision, stable identity, semantic graph,
  inspection, coverage, and impact-query scope defined in the roadmap and M19
  contract;
- preserve every M11, M16, and M17 single-source API behavior while adding the
  ordered multi-source revision path;
- match the exact M19 R1 graph and impact results, including retained-parent,
  stale-request, deterministic-order, and incomplete-coverage behavior;
- add `compiler/ail-compiler/tests/m20_impact.rs` as the focused executable
  conformance gate; and
- keep candidate commits, schema transactions, semantic diffs, completion
  evidence, UC-007, lowering, concurrency, and unrelated language expansion
  outside M20.

## M19 result

M19 accepted three language and ten protocol rules for the selected UC-003
change loop. The contract fixes explicit stable schema identities, ordered
source-set digests, twelve semantic relationship kinds, exact impact
categorization, honest coverage, whole-workspace candidate validation, semantic
diff, completion evidence, and five stable rejection causes.

Canonical R1 and R2 workspaces contain all required semantic roles. The fixture
set freezes twelve exact `must_change` locations, two `review` locations, one
unchecked external boundary, one successful transaction, and five rejection
scenarios. The dependency-free checker validates 13 rules, 16 protocol shapes,
seven scenarios, and five rejection mutations.

## M18 result

M18 reviewed the completed M17 evidence and selected compiler-guided UC-003
priority evolution as the next validation slice. ADR 0005 records why this
closes accepted schema-consequence and change-protocol gaps before the project
adds another runtime feature or use case.

The bounded sequence is contract first in M19, semantic graph and impact
implementation in M20, then atomic schema evolution and completion evidence in
M21. UC-007 and all unrelated scaling work remain outside that sequence.

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

## M16 result

M16 added a transport-independent `Workspace` API around the completed M15
checker. Each stored revision is immutable, canonical, and SHA-256 identified.
The API assigns deterministic revision-scoped handles to declarations, syntax,
and expressions; returns elaborated inspection results; and validates complete
M11 rename transactions.

A successful rename produces ordered canonical UTF-8 byte edits, reparses and
statically checks the complete candidate unit, publishes one child revision,
and returns a complete deterministic identity map. A stale base, stale or
non-symbol handle, invalid identifier, collision, or validation failure returns
one structured diagnostic with no edits and does not publish a partial revision.
The M16 tests cover canonical source and digest retention, function and local
inspection, edit replay, surviving and replaced identity mappings, rejection
paths, stale edit rejection, and repeated deterministic results.

## M17 result

M17 accepted nine bounded language, runtime, and protocol rules and implemented
them in the authoritative Rust compiler. The parser, canonical formatter,
static checker, and semantic index now cover field access, conditionals,
exhaustive closed-variant matches, `Bool`, `Bytes`, and the seven accepted
intrinsic functions. A deterministic tree-walking interpreter executes only
statically valid functions from an immutable revision.

`Workspace::execute` takes a revision-scoped function handle, checked runtime
arguments, and a caller-supplied capability provider. It returns structured
values, stable runtime faults, and ordered capability calls; invalid requests
cannot reach the store capability. The canonical reference service keeps
request validation, V1 priority adaptation, stored-job adaptation, and outcome
mapping visible in AIL source. The locked AIL benchmark runner passes all 37
public job-service cases while keeping JSON, Base64, and response projection at
the host boundary.

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
but it remains the fixed conformance boundary preserved by M14 through M17.

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
- M16 Rust revision protocol and validated rename
- M17 deterministic interpreter and public reference-service runner

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
- M16 — Rust revision protocol and validated rename
- M17 — Deterministic core interpreter
- M18 — Next validation-slice selection
- M19 — UC-003 schema-evolution contract

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

- M21 schema transaction implementation before M20 completes the accepted
  graph and impact query
- Native code generation, production runtime work, or general concurrency
- Official agent or performance evidence

## Blockers

None.

## Handoff checklist

After meaningful work:

- keep M20 focused on the accepted semantic graph and impact-query contract;
- do not implement M21 transaction or completion behavior early;
- run `cargo fmt --all --check`;
- run `cargo test --workspace`;
- run `cargo clippy --workspace --all-targets -- -D warnings`;
- run `python3 specs/tools/core_contract.py check`;
- run `python3 benchmarks/tools/harness.py verify --language ail --visibility public`;
- run `python3 benchmarks/tools/fixtures.py check`;
- run `python3 tools/check_docs.py`; and
- update this file and the roadmap only when the active milestone's exit
  criterion passes.
