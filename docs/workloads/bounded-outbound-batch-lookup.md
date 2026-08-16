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

M32 exposes only this entry point at `POST /v1/lookups:batch`. The separate Rust
host under `service-host/` pins revision `r1`, its source and capability-setting
digests, the exact function boundary, bound eight, limit three, and
`dependency.fetch` permission before accepting requests. The
[M32 host contract](../../specs/pinned-http-batch-lookup.md) fixes the HTTP,
JSON, version, failure, and audit behavior.

The executable host loads an explicit operator-supplied JSON catalog before it
binds `127.0.0.1:<port>`, defaulting to port 3000. Present keys return their
catalog value as `Found`; absent keys return `NotFound`. The sample catalog is
under `service-host/examples/`.

M33 makes this usable as a small local engineering runbook. Responses identify
both the pinned source and immutable catalog snapshot, and every outcome repeats
its original key while preserving order and duplicates. The host reserves one
of 256 process-local audit positions before compiler or provider work; after the
bound is reached, valid requests return 503 until restart. An optional explicit
port permits another loopback-only instance when port 3000 is occupied. See the
[M33 contract](../../specs/private-catalog-dogfood.md).
