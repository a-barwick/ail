# AIL specifications

Status: **Proposed normative contract**

This directory contains the smallest fixed contract for the first authoritative
Rust compiler milestones. It is deliberately narrower than the eventual AIL
core.

The fixed M11 contract consists of:

- [the five-construct language rules](core.md);
- [the transport-independent compiler protocol](protocol.md);
- [machine-readable contract metadata](core-contract.json);
- [machine-readable protocol shapes](protocol.json); and
- canonical fixtures under `fixtures/`.

The JSON files are a conformance-fixture encoding, not a selected compiler
transport. The Rust implementation must match the same rules and expected
results. It may not use implementation behavior to fill a gap in this contract.

M17 adds the accepted bounded [deterministic interpreter contract](runtime.md),
its [runtime protocol shapes](runtime-protocol.json),
[machine-readable rules](runtime-contract.json), and runtime fixtures under
`runtime-fixtures/`. These additions extend M11 without changing its fixed
five-construct contract.

M19 adds the accepted bounded
[compiler-guided schema-evolution contract](evolution.md), its
[protocol shapes](evolution-protocol.json),
[machine-readable rules](evolution-contract.json), and canonical R1/R2,
impact, transaction, and rejection fixtures under `evolution-fixtures/`. These
rules fix the contract for M20 and M21; they do not implement it.

M20 implements the identity, source-set, semantic-graph, inspection, coverage,
and impact-query portion of that contract. M21 owns candidate publication,
canonical edits, semantic diff, and completion evidence.

Run the dependency-free contract check with:

```bash
python3 specs/tools/core_contract.py check
```

[ADR 0004](../docs/decisions/0004-rust-compiler-stack.md) now authorizes the
production Rust compiler tree. M11 still does not authorize fixture-specific
extensions. M17 authorizes only the additional numbered behavior in
`runtime.md`; M19 authorizes only the schema-evolution behavior in
`evolution.md`.
