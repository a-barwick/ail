# AIL compiler

This directory contains the authoritative Rust implementation of AIL.

The compiler is delivered in conformance slices:

- M14: lossless syntax, deterministic recovery, and canonical formatting
  (complete);
- M15: static semantics and structured diagnostics (complete);
- M16: immutable revisions, inspection, validated rename, and identity maps
  (active);
- M17: deterministic interpretation of the accepted core.

The numbered rules and fixtures under [`../specs`](../specs/README.md) constrain
behavior. Implementation details do not create language semantics.

Run the current compiler checks from the repository root:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The `ail_compiler::check_source` API checks an immutable source revision using
caller-supplied capability interfaces. It returns canonical source, inferred
local and explicit public type facts, and structured diagnostics. Revision
storage, inspection requests, and rename transactions remain M16 work.
