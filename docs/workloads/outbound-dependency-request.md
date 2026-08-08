# Cooperative outbound dependency request

The service makes one direct request through one host-supplied dependency
capability. Canonical source passes a key, positive bounded timeout, and opaque
external cancellation token. It declares the exact `dependency.fetch` effect
and returns a closed outcome containing ordinary remote outcomes plus distinct
payloadless `TimedOut` and `Cancelled` completions.

The canonical example is `compiler/examples/outbound-request/`. Exact static,
runtime, revision, and inspection behavior is in the accepted
[M30 outbound request contract](../../specs/outbound-requests.md).

The provider is synchronous and cooperative. A timeout is a request constraint,
not a runtime clock or hard deadline: the interpreter cannot preempt a stuck
provider and does not promise remote rollback. This workload adds no URLs,
general networking, retries, asynchronous execution, concurrency, cancellation
creation or query, or ambient authority.
