# ADR 0011: Add one pinned HTTP batch-lookup host

- Status: Accepted contract
- Date: 2026-08-09
- Owners: project maintainers
- Documentation layer and scope: M32 service-host boundary and version pinning

## Context

M31 executes the bounded batch lookup through an injected provider but exposes
no production-facing request boundary. Adding HTTP inside AIL would expand the
language, authority model, and runtime before one service adapter proves that
those semantics are necessary. A host that follows the workspace's latest
revision would also make a running endpoint change behavior without an audited
restart.

## Decision

Add a separate Rust host with only `POST /v1/lookups:batch`. At construction it
matches explicit revision, source, capability-settings, function, type, effect,
bound, concurrency, and timeout pins against compiler inspection. It stores the
validated selector and always executes that immutable revision. HTTP decoding
finishes before the host creates an opaque cancellation token or invokes the
compiler, and the dependency provider is injected by host construction.

The host maps malformed transport input to 400, AIL bounds to 422, complete
execution to 200, and unexpected provider or runtime failure to 502. It retains
server-side compiler call traces and never turns an incomplete execution into a
partial 200 response.

## Consequences

AIL now runs behind one auditable HTTP endpoint without gaining ambient network
authority or general routing semantics. Updating canonical source or capability
settings requires an explicit pin update and service restart. The host remains
a narrow executable slice: it has no TLS, authentication, retries, hard
deadlines, hot reload, or deployment system.

Adding the host to the Rust workspace changes the root dependency lock. The AIL
benchmark verification manifest and its external lock therefore bind the new
workspace manifests; fixture behavior and the 37 public cases are unchanged.

## Alternatives considered

- Add HTTP syntax to AIL: broader than the blocked workload and unnecessary for
  explicit dependency injection.
- Follow the workspace current revision: permits silent behavior changes in a
  running service.
- Accept revision or capability selectors in JSON: transfers host authority to
  untrusted input.
- Return completed positions after a provider fault: violates whole-batch result
  semantics and hides partial failure behind 200.

## Validation

`cargo +1.87.0 test -p ail-service-host --test m32_pinned_http_service` proves
startup binding, strict zero-work rejection, bounded out-of-order success,
fail-stop traces, immutable version selection, and fixed routing. The full
workspace and M11–M31 contract checks remain the regression gate.
