# Current status

Last updated: 2026-08-07

## Active milestone

None — M28 is complete and no next build has been selected.

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
  and transitive effects.

The compiler rejects direct and mutual recursion, import cycles, inaccessible
declarations, ambiguous imports, stale handles, incomplete impact claims, and
invalid partial revisions.

The composed-service example under `compiler/examples/composed-service/` runs
three AIL modules through the real checker and interpreter. The AIL job-service
runner passes all 37 public cases.

## Hard limit

AIL cannot yet implement a normal production service. It has no iteration,
general collections, concurrency, networking, package registry, foreign-code
boundary, production runtime, native backend, or deployment system. Recursion is
rejected rather than bounded.

The architecture API implements the exact M24 metrics and policy behavior. The
larger catalog in `architecture-health.md` is design work, not compiler behavior.

## Next executable move

Choose one real service behavior blocked by the current language. Write the
canonical source, static result, runtime result, diagnostic failures, and
compiler-interface output first. Then implement the smallest missing semantics
and run the full existing suite.

Do not restart the old UC-008 M29–M36 plan or the M8 measurement work by default.
Do not start native lowering, general concurrency, or broad standard-library
work without an executable workload.

## Required checks after compiler changes

```bash
cargo +1.87.0 fmt --all --check
cargo +1.87.0 test --workspace
cargo +1.87.0 clippy --workspace --all-targets -- -D warnings
python3 specs/tools/architecture_acceptance.py check
python3 specs/tools/architecture_contract.py check
python3 specs/tools/core_contract.py check
PATH="$HOME/.cargo/bin:$PATH" python3 benchmarks/tools/harness.py verify --language ail --visibility public
python3 benchmarks/tools/fixtures.py check
python3 tools/check_docs.py
```
