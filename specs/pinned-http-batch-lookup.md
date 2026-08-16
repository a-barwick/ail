# M32 pinned HTTP batch-lookup host contract

Status: **Accepted 2026-08-09**

M32 exposes the existing M31 batch lookup through one Rust-owned HTTP adapter.
It adds no HTTP syntax, routing authority, or capability construction to AIL.

### M32-HOST-001 — Exact startup binding

Before a router can accept requests, the host matches configured revision `r1`,
source-set digest, capability-environment digest,
`batch_lookup.service.lookup_batch`, request and result types, list bound eight,
concurrency limit three, maximum timeout, and `dependency.fetch` permission
against compiler revision metadata and function inspection. Any mismatch stops
startup and names the mismatched field.

### M32-HOST-002 — One fixed HTTP boundary

The only operation is `POST /v1/lookups:batch` with JSON content. Unknown routes
return 404 and other methods on the route return 405. The host owns HTTP, JSON,
a 16 KiB request-body limit, cancellation-token construction, and status
mapping. It invokes only the pinned entry function.

### M32-HOST-003 — Strict zero-work rejection

Malformed JSON, wrong or missing content type, unknown fields, wrong field
types, oversized bodies, and client-supplied revision, capability, or
cancellation identities return 400. More than eight requests and timeout values
outside `1..=1000` return 422. These paths create no compiler execution and
start no dependency request.

### M32-HOST-004 — Fixed complete results

A completed batch returns 200 with pinned revision ID, source-set digest, and
ordered `Found | NotFound | Unavailable | TimedOut | Cancelled` outcomes. Any
unexpected provider, runtime, or result-contract failure returns 502 and never
returns partial successful outcomes.

M33 extends each complete response with its immutable catalog digest and each
outcome with the corresponding request key. Exact rules live in
`specs/private-catalog-dogfood.md`.

### M32-HOST-005 — Immutable running version

The host stores its validated revision and function selector. Every request
executes those values even when the workspace later retains a newer revision.
Changing embedded canonical source or capability settings without updating the
configured digests prevents startup rather than silently moving the service.

### M32-HOST-006 — Auditable execution records

For every compiler execution, server-side records retain the pinned revision
and source digest, successfully started calls, input positions, start order,
host completion order, timeout, any observed or specified synthesized closed
outcome/result, and original unexpected fault code. Outcome and result remain
absent when an unexpected check or collect fault prevents them. A failed start
is absent. Synthesized cancellation of active work records Cancelled outcome and
result but no host completion order.

M33 bounds this process-local store at 256 compiler executions and requires
fail-closed reservation and completion. It also binds records to the catalog
digest.

### M32-HOST-007 — Host-injected outside authority

The host injects the `DependencyClient` implementation when it constructs the
service. HTTP input supplies only lookup keys and timeout. It cannot create a
capability, grant `dependency.fetch`, select a revision, or choose a
cancellation-token identity.

The executable host requires one operator-supplied JSON catalog path at process
startup. It loads that catalog before constructing the service, rejects malformed
or duplicate entries, and keeps the resulting key/value map immutable. A present
key returns `Found` with its catalog value; an absent key returns `NotFound`.

## Canonical proof

`service-host/tests/m32_pinned_http_service.rs` checks every startup pin,
out-of-order eight-item completion with at most three active requests, strict
zero-work rejection including negative and non-representable timeout numbers,
complete audit values, operator-catalog lookup behavior, fail-stop 502 behavior
and trace accuracy, immutable `r1` execution after retaining `r2`, and exact
route/method behavior. Existing M11 through M31 suites remain the regression gate.

## Non-goals

No generic routing language, arbitrary JSON reflection, client-selected source
versions, AIL HTTP-client syntax, TLS, authentication, streaming, WebSockets,
retries, hard deadlines, hot reload, unrestricted concurrency, package system,
deployment system, LLVM, or native compilation.
