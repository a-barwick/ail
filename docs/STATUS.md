# Current status

Last updated: 2026-07-21

## Active milestone

M23 — UC-007 acceptance package

## Current goal

Prepare a deterministic acceptance package for architectural regression control
before accepting UC-007 or changing the compiler. The execution path is:

```text
M11 five-construct contract (complete)
  -> ADR 0004 Rust decision (accepted)
  -> M14 lossless syntax and formatter (complete)
  -> M15 static semantics and diagnostics (complete)
  -> M16 revision protocol and validated rename (complete)
  -> M17 deterministic core interpreter (complete)
  -> M18 next validation-slice selection (complete)
  -> M19 UC-003 schema-evolution contract (complete)
  -> M20 workspace semantic graph and impact query (complete)
  -> M21 atomic schema evolution and completion evidence (complete)
  -> M22 post-UC-003 validation-slice selection (complete)
  -> M23 UC-007 acceptance package (active)
  -> M24 architectural regression contract (planned, conditional)
  -> M25 architectural snapshot and agent rendering (planned, conditional)
  -> M26 architectural delta, policy, and atomic enforcement (planned, conditional)
  -> M27 non-official architecture-feedback pilot (planned, conditional)
```

M12 and M13 are superseded. Do not build TypeScript compiler semantics or a
candidate scorecard.

The next agent should:

- read [ADR 0006](decisions/0006-prepare-architectural-regression-control.md),
  [UC-007](use-cases/UC-007-architectural-regression-control.md), the
  [architectural-health requirements](requirements/architectural-health.md),
  and the [proposed manifest](architecture-health.md);
- work only on the M23 documentation, fixtures, and dependency-free acceptance
  checker; do not write Rust compiler or runtime code;
- freeze the exact starting workspace, `CancelJob` behavior, valid change,
  centralized regression, superficial helper split, project policy, hotspot
  baseline, minimal metrics, expected structured and compact text, baseline
  comparison, and fixed budgets;
- keep UC-007 and its requirements Proposed until the written gate passes and
  two independent readers can classify all three candidates identically; and
- stop after M23 with a review summary. Do not start M24 implicitly.

Amp may be used to author M23. If it is, start from a clean scoped branch,
record `amp --version`, the selected mode, exact prompt, and thread ID or URL,
then review the diff and rerun the checks before merging. Amp is an optional
operator, not official evidence or a source of language semantics.

## Restart prompt

Use this prompt with Codex, Amp, or another repository-aware coding agent:

```text
Implement only active milestone M23, the UC-007 acceptance package.

Read AGENTS.md and the required project documents in its stated order. Then
read docs/decisions/0006-prepare-architectural-regression-control.md,
docs/use-cases/UC-007-architectural-regression-control.md,
docs/requirements/architectural-health.md, docs/architecture-health.md,
docs/STATUS.md, and M23 in docs/roadmap.md.

Do not write Rust compiler or runtime code. Freeze the exact mature job-service
workspace, CancelJob behavior, valid/centralized/helper-splitting candidates,
project boundaries and hotspot baseline, minimal metrics, expected structured
and compact text output, baseline-tool comparison, and fixed false-finding,
analysis, and manifest-size budgets. Add a dependency-free acceptance checker.
Keep UC-007 and its requirements Proposed until the written gate passes and two
independent readers can classify all three candidates identically. Run the M23
focused checks and the repository documentation check. Stop with a review
summary; do not start M24, commit, merge, or push unless explicitly asked.
```

For Amp, the shortest safe invocation is:

```bash
amp -x "Read docs/STATUS.md and implement only the active milestone exactly as its restart prompt says. Do not start the successor milestone."
```

## M22 result

M22 selected architectural regression control as the next scaling direction in
[ADR 0006](decisions/0006-prepare-architectural-regression-control.md). The
selection is deliberately conditional: M23 must first make the workspace,
behavior, policy, metrics, examples, baseline comparison, output contract, and
budgets concrete enough to accept or reject.

The planned campaign is one short acceptance gate, one contract milestone, two
implementation milestones, and one small non-official usability pilot. Compact,
deterministic agent text is now an explicit product surface derived from the
compiler's authoritative structured facts. UC-007 and its requirements remain
Proposed, and no architectural-health compiler implementation is authorized
during M23.

## M21 result

M21 implemented the accepted whole-workspace candidate transaction. The
compiler validates the current base, exact M20 impact accounting, complete
canonical source set, stable schema identities, effect and capability
non-growth, and caller-supplied public behavior evidence before publishing one
child revision.

The committed R1-to-R2 fixture matches the frozen source-set digest, five
ordered whole-path edits, persistent identity classifications, seven semantic
changes, unchanged store authority and effect ordering, and revision-bound
completion evidence. Structured stale, missed-consumer, incompatible-identity,
effect-growth, behavior-mismatch, incomplete-source, and static-invalid
rejections all return empty edits and publish no revision. All prior compiler
tests and the 37-case public AIL corpus remain passing.

## M20 result

M20 implemented contextual schema identities, immutable ordered source-set
revisions, the twelve-kind semantic relationship graph, and exact revision-bound
impact queries in the authoritative Rust compiler. Persistent identities remain
separate from source-revision handles, and incomplete declared coverage cannot
produce a clean impact report.

The focused tests match the M19 R1 digest and exact impact categories, exercise
every relationship kind in deterministic order, retain parent snapshots, reject
stale requests and incomplete coverage, and preserve repeated results. All
M11–M17 tests remain unchanged and passing.

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
- M20 workspace graph and exact impact query
- M21 atomic schema evolution and completion evidence
- ADR 0006 architectural-regression-control direction and bounded campaign

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
- M20 — Workspace semantic graph and impact query
- M21 — Atomic schema evolution and completion evidence
- M22 — Post-UC-003 validation-slice selection

## Superseded

- M12 — Comparable Rust and TypeScript compiler spikes
- M13 — Separate compiler-stack decision milestone

## Deferred

- M8g–M8o — Official baseline calibration campaign
- M9 — Numeric AIL benchmark targets
- M10 — Illustrative AIL variants

These items require an explicit maintainer decision to resume. They do not
block the active acceptance work.

UC-007 and its requirements remain Proposed while active M23 prepares their
acceptance evidence. M24–M27 remain conditional and activate one at a time.

## Do not start yet

- Architectural-health compiler or runtime implementation before M23 accepts
  the use case and M24 freezes the contract
- Native code generation, production runtime work, or general concurrency
- Official agent or performance evidence

## Blockers

None.

## Handoff checklist

After meaningful work:

- keep M23 focused on acceptance evidence, fixtures, and its dependency-free
  checker;
- do not implement architectural-health compiler behavior before M24 is active
  and its numbered contract is accepted;
- run `cargo fmt --all --check`;
- run `cargo test --workspace`;
- run `cargo clippy --workspace --all-targets -- -D warnings`;
- run `python3 specs/tools/core_contract.py check`;
- run `python3 benchmarks/tools/harness.py verify --language ail --visibility public`;
- run `python3 benchmarks/tools/fixtures.py check`;
- run `python3 tools/check_docs.py`; and
- update this file and the roadmap only when the active milestone's exit
  criterion passes.
