# Delivery history

The compiler and pinned service host have shipped through M32. No milestone is
active.

This file records what each milestone produced. Exact behavior lives in the
specifications, tests, and compiler—not in milestone prose.

## Milestones

| Milestone | Shipped capability | Status | Depends on |
| --- | --- | --- | --- |
| M0 | Job-service workload and requirements | Complete | — |
| M1 | Job-service fixtures and validator | Complete | M0 |
| M2 | Shared benchmark harness and task contract | Complete | M1 |
| M3 | Rust baseline | Complete | M2 |
| M4 | Go baseline | Complete | M2 |
| M5 | Python baseline | Complete | M2 |
| M6 | TypeScript baseline | Complete | M2 |
| M7 | Cross-language behavior parity and locked task starts | Complete | M3–M6 |
| M8 | Agent and performance measurement infrastructure through M8f | Deferred | M7 |
| M9 | Numeric AIL comparison targets | Deferred | M8 |
| M10 | Illustrative syntax comparison | Deferred | — |
| M11 | Five-construct compiler contract and fixtures | Complete | M0, M7 |
| M12 | Rust/TypeScript compiler spikes | Superseded | M11 |
| M13 | Separate compiler-stack decision | Superseded | M11 |
| M14 | Rust lossless parser and canonical formatter | Complete | M11, ADR 0004 |
| M15 | Name, type, capability, and diagnostic checking | Complete | M14 |
| M16 | Immutable revisions, inspection, and validated rename | Complete | M15 |
| M17 | Deterministic interpreter and 37-case AIL job service | Complete | M16 |
| M18 | Selected schema evolution as the next compiler problem | Complete | M17 |
| M19 | Schema identity, impact, and transaction contract | Complete | M18 |
| M20 | Multi-file semantic graph and exact impact query | Complete | M19 |
| M21 | Atomic schema evolution and completion result | Complete | M20 |
| M22 | Selected architecture regression control | Complete | M21 |
| M23 | Concrete `CancelJob` architecture cases and policy | Complete | M22 |
| M24 | Architecture metrics, deltas, policy, and diagnostics contract | Complete | M23 |
| M25 | Revision-bound architecture snapshot and compact output | Complete | M24 |
| M26 | Cross-revision policy enforcement and atomic publication | Complete | M25 |
| M27 | One recorded repair using architecture diagnostics | Complete | M26 |
| M28 | AIL calls, modules, imports, transitive effects, and nested execution | Complete | M27 |
| M29 | Immutable bounded lists, sequential map, pre-effect validation, and ordered batch cancellation | Complete | M28 |
| M30 | Cooperative outbound request metadata, timeout, cancellation, closed results, and revision binding | Complete | M29 |
| M31 | Bounded outbound workflows, ordered results, cooperative batch cancellation, and complete inspection | Complete | M30 |
| M32 | One revision-pinned HTTP batch-lookup host with strict JSON and auditable execution | Complete | M31 |

## Shipped compiler path

### Frontend and execution: M11, M14–M17

The project defined the first five constructs, selected Rust, and shipped a
lossless parser, canonical formatter, static checker, immutable revision store,
validated rename, and deterministic interpreter. The AIL job service passes all
37 public fixtures.

### Safe schema evolution: M18–M21

The compiler gained ordered multi-file revisions, stable schema identities, a
typed semantic graph, exact `must_change`/`review`/`unchecked` impact results,
semantic diffs, and atomic whole-workspace validation. Stale, incomplete,
effect-changing, or behaviorally invalid candidates publish no revision.

### Architecture enforcement: M22–M27

The compiler derives architecture facts at function, module, dependency-component,
and configured-group scopes. It compares revisions, applies project policy, and
rejects changes that grow the dispatch hotspot or move jobs-store authority and
state into transport. Helper splitting does not evade aggregate checks. Required
analysis that exceeds coverage or budget returns `incomplete`, not success.

M27 recorded one agent repair of the seeded centralized `CancelJob` change. That
run proved the output was usable in that case; it did not compare AIL with other
languages.

### Language composition: M28

M28 added canonical module and import headers, import aliases, qualified
references, local and imported calls, exact argument checking, transitive
capability-effect checking, left-to-right argument evaluation, and deterministic
nested interpretation. The compiler rejects recursive calls, inaccessible or
ambiguous names, and import cycles. A working three-file program is under
`compiler/examples/composed-service/`.

### Bounded ordered work: M29

M29 added structural `List<T, N>` types with exact bounds, contextual sequential
`map`, complete external list validation before capability checks or calls,
ordered fail-stop execution, map/binder/bound inspection, and linked source-set
function inspection. The three-module cancellation program under
`compiler/examples/batch-cancellation/` returns one ordered closed outcome per
input position and rejects oversized or malformed external input with zero
calls.

### Cooperative outbound dependency call: M30

M30 added one host-bound outbound operation without adding networking syntax.
Existing capability parameters and effects remain the exact permission. The
compiler validates timeout and cancellation positions, a bounded millisecond
timeout, and persistent identities for closed timeout/cancel results. Retained
revisions own and digest their complete capability environments; inspection and
execution use those saved contracts. The provider is synchronous and
cooperative, not a hard-preemptive production network runtime.

### Bounded outbound batch lookup: M31

M31 added contextual `parallel map ... limit C` for exactly one outbound
operation over a bounded list. The interpreter validates the complete batch
before starts, keeps at most `C` opaque host handles active, stores completions
at input positions, cooperatively cancels active work, and preserves unexpected
host faults. Architecture settings survive ordinary child revisions and
outside-operation/state facts propagate through callers across modules.

### Pinned batch-lookup HTTP host: M32

M32 added a separate Rust host for only `POST /v1/lookups:batch`. Startup binds
the exact retained revision, source and capability-setting digests, function,
types, effect, list bound, concurrency limit, and timeout maximum to compiler
inspection. Strict JSON and bounds rejection starts no dependency work. Complete
results report the pinned revision and digest; unexpected execution failure
returns 502 without partial outcomes. Retaining a newer revision does not move a
running host's selector.

## Deferred work

M8g–M10 remain deferred. A superseded plan assigned M29–M36 to UC-008 workload,
language, and comparison work. That sequence was never started. The M29 number
now identifies the bounded-list delivery described above; it does not reactivate
the old UC-008 plan.

## Next delivery

No next milestone has been selected. The next owner should start from a concrete
service behavior blocked by the current language, define deterministic source,
static, runtime, diagnostic, and protocol tests, and implement the smallest
missing semantics. Current high-value gaps are general iteration and collection
operations beyond M29, resource-safe concurrency, production execution, and
package boundaries, but none is selected merely by appearing on this list.
