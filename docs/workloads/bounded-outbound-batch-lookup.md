# Bounded outbound batch lookup

The service accepts `List<LookupRequest, 8>`, runs no more than three host
dependency requests at once, and returns `List<LookupOutcome, 8>` aligned to
input positions. Host completions may arrive out of order. Outcomes include
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

The source is under `compiler/examples/batch-lookup/`. Exact behavior is in
[bounded-outbound-workflows.md](../../specs/bounded-outbound-workflows.md).

The separate Rust host under `service-host/` exposes only
`POST /v1/lookups:batch`. It pins revision `r1`, source and capability-setting
digests, the entry function, bound eight, limit three, and `dependency.fetch`
before accepting requests. See
[pinned-http-batch-lookup.md](../../specs/pinned-http-batch-lookup.md).

The executable loads an operator-supplied JSON catalog before it binds
`127.0.0.1:<port>`, defaulting to port 3000. Present keys return their catalog
value as `Found`; absent keys return `NotFound`. Sample catalog:
`service-host/examples/`.

Responses identify the pinned source and catalog snapshot. Every outcome
repeats its original key. The host admits at most 256 executions per process
and returns 503 when that bound is full. See
[private-catalog-dogfood.md](../../specs/private-catalog-dogfood.md).
