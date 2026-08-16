# Current status

Last updated: 2026-08-15

## Active milestone

None — M32 is complete and no next build has been selected.

## Shipped

The Rust compiler now supports:

- canonical parsing and formatting;
- name, type, capability-effect, module, import, and call checking;
- immutable source revisions and revision-scoped semantic inspection;
- validated rename and identity mapping;
- deterministic execution of the supported language;
- ordered multi-file source sets, schema impact, semantic diffs, and atomic
  candidate validation;
- revision-bound architecture snapshots, deltas, project-policy enforcement,
  and rollback on denied or incomplete analysis; and
- local, imported, aliased, and qualified calls with left-to-right arguments
  and transitive effects;
- immutable structural `List<T, N>` values with exact compiler-visible bounds;
- contextual sequential `map` with one aligned result per input position; and
- complete external list validation before capability availability checks or
  calls, plus linked module, element-identity, bound, effect, and dependency
  inspection; and
- one cooperative outbound dependency request with explicit capability
  permission, bounded timeout, opaque cancellation, closed timeout/cancel
  results, revision-bound provider contracts, and deterministic inspection; and
- one bounded outbound workflow over `List<T, N>` with a fixed concurrency
  limit, input-ordered results, cooperative whole-batch cancellation, separate
  start/completion traces, effect-free argument preparation, and host-supplied
  start/check/cancel/collect.

The separate Rust service host now exposes only `POST /v1/lookups:batch`. It
starts only when revision `r1`, source and capability-setting digests, entry
function, types, bound eight, limit three, timeout maximum, and
`dependency.fetch` permission match compiler inspection. Strict JSON rejection
does zero dependency work; complete execution returns ordered outcomes with the
pinned revision and digest; unexpected provider/runtime failure returns 502;
and server-side records retain accurate call lifecycle facts and complete closed
results. The executable loads an explicit immutable JSON catalog at startup and
returns its real value for present keys rather than a canned outcome.

The compiler rejects direct and mutual recursion, import cycles, inaccessible
declarations, ambiguous imports, stale handles, incomplete impact claims, and
invalid partial revisions.

The examples under `compiler/examples/` run through the real checker and
interpreter. Bounded cancellation accepts zero to 32 job positions, preserves
order and duplicates, returns one closed outcome per position, and rejects
oversized or malformed input before effects. The outbound example makes one
dependency request through a host-supplied capability and returns closed remote,
timeout, and cancellation outcomes. The AIL job-service runner passes all 37
public cases.

## Hard limit

AIL now runs one pinned batch lookup behind one fixed HTTP adapter; it cannot
implement a general production service. Its only repeated-work forms are
sequential map and one direct bounded outbound map over immutable bounded lists.
It has no general loops, collection library, mutation, general concurrency,
general networking or routing, package registry, foreign-code boundary,
production runtime, native backend, TLS/authentication, or deployment system.
The outbound provider is synchronous and cooperative: the interpreter cannot
preempt stuck host code, enforce a hard deadline itself, or roll back remote
effects. The executable provider is an immutable local catalog, not a general
network dependency. Recursion is rejected rather than bounded.

The architecture API implements the exact M24 metrics and policy behavior.

## Next executable move

Choose one real service behavior still blocked by the current language. Write
the canonical source, static result, runtime result, diagnostic failures, and
compiler-interface output first. Then implement the smallest missing semantics
and run the full existing suite. Do not infer general collection, networking,
routing, or concurrency semantics from the narrow M29–M32 contracts.

Do not restart the old UC-008 M29–M36 plan or the M8 measurement work by default.
Do not start native lowering, general concurrency, or broad standard-library
work without an executable workload.

## Required checks after compiler changes

```bash
cargo +1.87.0 fmt --all --check
cargo +1.87.0 test --workspace
cargo +1.87.0 clippy --workspace --all-targets -- -D warnings
cargo +1.87.0 test -p ail-service-host --test m32_pinned_http_service
python3 specs/tools/architecture_acceptance.py check
python3 specs/tools/architecture_contract.py check
python3 specs/tools/core_contract.py check
python3 specs/tools/bounded_list_contract.py check
python3 specs/tools/outbound_request_contract.py check
python3 specs/tools/bounded_outbound_workflow_contract.py check
PATH="$HOME/.cargo/bin:$PATH" python3 benchmarks/tools/harness.py verify --language ail --visibility public
python3 benchmarks/tools/fixtures.py check
python3 tools/check_docs.py
```
