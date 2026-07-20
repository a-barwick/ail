# AIL specifications

Status: **Proposed normative contract**

This directory contains the smallest shared contract needed by the M12 Rust
and TypeScript compiler-stack spikes. It is deliberately narrower than the
eventual AIL core.

The M11 contract consists of:

- [the five-construct language rules](core.md);
- [the transport-independent compiler protocol](protocol.md);
- [machine-readable contract metadata](core-contract.json);
- [machine-readable protocol shapes](protocol.json); and
- canonical fixtures under `fixtures/`.

The JSON files are a conformance-fixture encoding, not a selected compiler
transport. M12 candidates must implement the same rules and expected results.
They may not use implementation behavior to fill a gap in this contract.

Run the dependency-free contract check with:

```bash
python3 specs/tools/core_contract.py check
```

M11 does not authorize a production compiler tree, root package manager,
interpreter, runtime, or candidate-specific extension.
