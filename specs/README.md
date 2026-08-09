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

M29 adds the accepted [bounded ordered list contract](bounded-lists.md), its
[protocol facts](bounded-list-protocol.json),
[machine-readable rules](bounded-list-contract.json), and the canonical
three-module program under `compiler/examples/batch-cancellation/`. This is a
standalone extension; it does not change the locked M11, M17, or M19 contracts.

M30 adds the accepted cooperative [outbound request contract](outbound-requests.md),
its [protocol shapes](outbound-request-protocol.json),
[machine-readable rules](outbound-request-contract.json), and canonical program
under `compiler/examples/outbound-request/`. It retains ordinary call syntax
and capability-effect authority and adds no general networking.

M31 adds the accepted [bounded outbound workflow contract](bounded-outbound-workflows.md),
its [protocol facts](bounded-outbound-workflow-protocol.json),
[machine-readable rules](bounded-outbound-workflow-contract.json), and canonical
batch lookup under `compiler/examples/batch-lookup/`. It adds no general async,
threads, retries, or networking.

M32 adds the accepted [pinned HTTP batch-lookup host contract](pinned-http-batch-lookup.md)
and the separate Rust program under `service-host/`. It adds one fixed endpoint
around the M31 entry function, not HTTP syntax or ambient authority inside AIL.

Run the dependency-free contract check with:

```bash
python3 specs/tools/core_contract.py check
python3 specs/tools/bounded_list_contract.py check
python3 specs/tools/outbound_request_contract.py check
python3 specs/tools/bounded_outbound_workflow_contract.py check
```

[ADR 0004](../docs/decisions/0004-rust-compiler-stack.md) now authorizes the
production Rust compiler tree. M11 still does not authorize fixture-specific
extensions. M17 authorizes only the additional numbered behavior in
`runtime.md`; M19 authorizes only the schema-evolution behavior in
`evolution.md`.
M29 authorizes only immutable `List<T, N>` values and sequential binder-style
`map`; broader collection and iteration semantics remain outside the contract.
M30 authorizes only synchronous cooperative outbound operation metadata,
external cancellation values, closed timeout/cancel completion, and
revision-bound capability environments.
M31 authorizes only one fixed-limit direct outbound map and its cooperative host
lifecycle; general concurrency remains outside the contract.
M32 authorizes only the pinned batch-lookup HTTP adapter; general routing,
client-selected revisions, authentication, TLS, retries, and hot reload remain
outside the contract.
