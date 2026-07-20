# AIL specifications

Status: **Proposed normative contract**

This directory contains the smallest fixed contract for the first authoritative
Rust compiler milestones. It is deliberately narrower than the eventual AIL
core.

The M11 contract consists of:

- [the five-construct language rules](core.md);
- [the transport-independent compiler protocol](protocol.md);
- [machine-readable contract metadata](core-contract.json);
- [machine-readable protocol shapes](protocol.json); and
- canonical fixtures under `fixtures/`.

The JSON files are a conformance-fixture encoding, not a selected compiler
transport. The Rust implementation must match the same rules and expected
results. It may not use implementation behavior to fill a gap in this contract.

Run the dependency-free contract check with:

```bash
python3 specs/tools/core_contract.py check
```

[ADR 0004](../docs/decisions/0004-rust-compiler-stack.md) now authorizes the
production Rust compiler tree. M11 still does not authorize fixture-specific
extensions, an interpreter, runtime behavior, or semantics beyond its numbered
rules.
