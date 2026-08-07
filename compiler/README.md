# AIL compiler

This directory contains the authoritative Rust implementation of AIL.

The compiler is delivered in conformance slices:

- M14: lossless syntax, deterministic recovery, and canonical formatting
  (complete);
- M15: static semantics and structured diagnostics (complete);
- M16: immutable revisions, inspection, validated rename, and identity maps
  (complete);
- M17: deterministic interpretation of the accepted core (complete).

M18 selected compiler-guided UC-003 priority evolution as the next validation
slice. M19 accepted its conformance contract, M20 implemented the ordered
source-set semantic graph and impact query, and M21 completed atomic schema
evolution and completion evidence. M22 selected architectural regression
control as the next direction, M23 accepted its concrete evidence package, M24
accepted the bounded contract, M25 implemented the read-only architectural
snapshot and compact rendering, and M26 implemented cross-revision policy,
governance, bounded failure, and atomic publication. M27 retained one
non-official feedback pilot without changing compiler behavior. A maintainer
scope correction redirects M28 to language composition. M28 adds statically
checked AIL calls plus explicit modules and imports over the existing ordered
source-set model.

These completed slices are conformance and mechanism evidence for bounded
contracts. M27 adds one non-official usability observation. They are not
official comparative evidence that AIL reduces total agent change cost. Compiler
volume, broader syntax, a native backend, or self-hosting must not be treated as
the next maturity step without a selected use case and roadmap milestone.

The numbered rules and fixtures under [`../specs`](../specs/README.md) constrain
the earlier slices. The M28 implementation and focused executable tests define
the new bounded composition behavior authorized directly by the maintainer.
Backend work remains outside scope.

Run the current compiler checks from the repository root:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The `ail_compiler::check_source` API checks one source revision using
caller-supplied capability interfaces. It returns canonical source, inferred
local and explicit public type facts, and structured diagnostics.
`ail_compiler::Workspace` stores immutable canonical revisions, exposes
deterministic revision-scoped handles and elaborated inspection, and validates
atomic rename transactions with canonical edits and complete identity maps.
`ail_compiler::EvolutionWorkspace` additionally validates complete multi-source
candidates atomically, exposes the exact uncommitted revision to a behavior
oracle, and returns revision-bound impact, edit, identity, semantic-diff, and
completion evidence.
An explicit multi-file source set uses one `module name;` header per file and
zero or more `import dependency;` headers. Imports expose a dependency's
records, variants, and functions under their declared names; declarations in
non-imported modules are inaccessible. `EvolutionWorkspace::execute` runs a
checked cross-module entry point. AIL calls use `function(arguments)` syntax,
evaluate arguments left-to-right, propagate same-named capability parameters,
and require callers to declare all transitively reachable effects. Recursive
call cycles are rejected.
`ail_compiler::architecture_snapshot` derives the accepted four-scope,
seven-metric architecture snapshot from validated immutable semantic facts and
returns bounded incomplete results when coverage or a fixed budget is exhausted.
`ail_compiler::ArchitectureWorkspace::validate_architecture_change` compares a
validated candidate with the current immutable revision, derives the canonical
snapshot and delta, evaluates trusted policy and governance, and publishes one
child only when behavior passes and no denied or incomplete finding remains.
