# Bounded outbound batch lookup

The canonical service accepts `List<LookupRequest, 8>`, runs no more than three
host dependency requests at once, and returns `List<LookupOutcome, 8>` aligned
to input positions. Host completions may arrive out of order. Outcomes include
found, not found, unavailable, timed out, and cancelled; there are no retries.

```text
List<LookupRequest, 8>
        │
        ▼
at most 3 active dependency requests
        │
        ▼
List<LookupOutcome, 8> in input order
```

The source is under `compiler/examples/batch-lookup/`. Exact behavior is fixed
by the [M31 contract](../../specs/bounded-outbound-workflows.md) and checked by
`compiler/ail-compiler/tests/m31_bounded_outbound_workflows.rs`.
