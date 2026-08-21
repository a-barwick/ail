# ADR 0004: Use Rust for the AIL compiler

- Status: Accepted
- Date: 2026-07-19

## Context

AIL needs one authoritative compiler implementation. A Rust versus TypeScript
spike comparison was planned and then skipped. The desired properties are
strong representation invariants, a native binary without a required managed
runtime, and one language for semantics, diagnostics, and structural edits.

## Decision

Rust is the implementation language for the AIL compiler. The production
compiler lives in the root Cargo workspace under `compiler/`. TypeScript is
not a second semantic implementation.

## Consequences

- A root Cargo workspace is authorized.
- Specs and fixtures constrain the implementation; compiler behavior may not
  silently reinterpret them.
- Rust-specific library and storage choices still require ordinary review.
- This ADR does not choose a parser library, backend, or AIL memory model.

## Alternatives considered

Complete comparative spikes first: extra implementation work in a stack that
would not be kept.

Start in TypeScript and rewrite later: two sources of truth and a likely
semantic rewrite.

Split compiler and tooling languages immediately: extra boundaries before the
compiler has demonstrated a need for them.

## Validation

The root Rust workspace builds and `cargo +1.87.0 test --workspace` passes.
