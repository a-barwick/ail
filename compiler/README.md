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
slice. M19 is defining its conformance contract; no schema-evolution compiler
behavior is authorized until those numbered rules and fixtures are accepted.

The numbered rules and fixtures under [`../specs`](../specs/README.md) constrain
behavior. Implementation details do not create language semantics.

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
