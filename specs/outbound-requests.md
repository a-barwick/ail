# M30 cooperative outbound request contract

Status: **Accepted 2026-08-08**

This contract adds only the auditable outbound operation needed by the
[cooperative outbound dependency workload](../docs/workloads/outbound-dependency-request.md).
It retains existing `client.operation(args)` syntax and capability-effect
authority. Metadata is supplied by the host; it does not add networking syntax
or ambient access.

## Static operation contract

### M30-LANG-001 — Existing syntax and exact authority

One fixed capability instance is supplied for each capability parameter. The
canonical function has `dependency: capability DependencyClient`, makes one
direct `dependency.fetch(key, timeout, cancellation)` call, and declares
`effects { dependency.fetch }`. Existing missing-capability and
`AIL.CAPABILITY.UNDECLARED_EFFECT` checks apply. Outbound metadata neither
grants authority nor changes ordinary operation compatibility.

### M30-LANG-002 — Outbound argument metadata

Every host operation has kind `ordinary` or `outbound`. Outbound metadata names
distinct zero-based timeout and cancellation argument indices within the
operation signature. The timeout parameter is exactly `Int`. The cancellation
parameter is exactly the built-in external-only `Cancellation` type. Invalid
timeout metadata emits `AIL.CAPABILITY.OUTBOUND_TIMEOUT_CONTRACT`; invalid or
overlapping cancellation metadata emits
`AIL.CAPABILITY.OUTBOUND_CANCELLATION_CONTRACT`.

`Cancellation` is opaque. Source may declare a parameter and pass its value but
cannot construct, pattern-match, inspect, compare, or query it. There is no
source operation that creates or signals cancellation.

### M30-LANG-003 — Timeout and closed completion metadata

The metadata maximum timeout is an integer in `1..=18,446,744,073,709,551,615`
(`u64::MAX`). The operation result is a declared closed variant. Metadata names
the persistent identities of declared, distinct, payloadless cases for timed-out
and cancelled completion. The result variant itself must also declare a
persistent identity so inspection cannot substitute a display name.
An invalid maximum, non-variant result, absent or payload-bearing case, or equal
case emits `AIL.CAPABILITY.OUTBOUND_RESULT_CONTRACT`, except a zero/out-of-range
maximum emits `AIL.CAPABILITY.OUTBOUND_TIMEOUT_CONTRACT`.

### M30-LANG-004 — Closed expected remote outcomes

Expected remote outcomes such as `Found`, `NotFound`, and `Unavailable` are
ordinary cases in the declared result variant and enter through a provider
`Returned(value)` outcome. Only the designated `TimedOut` and `Cancelled` cases
are synthesized by the interpreter. No catchable provider fault, retry, URL,
transport, clock, async, or concurrency behavior is implied.

## Runtime and provider behavior

### M30-RUNTIME-001 — External validation and timeout precedence

The interpreter validates all external ordinary arguments in declaration order
before capability availability or effect checks. A cancellation value must be a
well-formed opaque host token. It then reads the designated timeout as an exact
integer and requires `1..=maximum` before recording or invoking the call. Zero,
over-maximum, or non-representable timeout emits
`AIL.RUNTIME.OUTBOUND_TIMEOUT_ARGUMENT` with expected maximum, actual value, and
argument index and leaves observed calls empty. Malformed cancellation uses the
existing `AIL.RUNTIME.ARGUMENT_TYPE` fault and also leaves calls empty.

### M30-RUNTIME-002 — Separate cooperative provider path

An outbound call invokes only the provider's distinct outbound API, passing the
ordinary arguments, timeout, and opaque cancellation token. The API returns
exactly `Returned(value)`, `TimedOut`, or `Cancelled`. A provider without that
API emits `AIL.RUNTIME.OUTBOUND_UNSUPPORTED`; the interpreter never falls back
to the ordinary provider API. The call is recorded immediately before supported
provider invocation.

### M30-RUNTIME-003 — Closed completion and faults

`Returned(value)` must validate as the operation's declared result or emits the
existing `AIL.RUNTIME.CAPABILITY_RESULT` fault. `TimedOut` and `Cancelled`
produce the metadata-designated payloadless result cases. The typed provider
outcome is closed, so an unknown outcome tag is unrepresentable; an unknown case
inside `Returned(value)` is `AIL.RUNTIME.CAPABILITY_RESULT`. A provider
`RuntimeFault` propagates unchanged, as for ordinary capability calls. Neither
fault is converted to an expected closed result. A missing interpreter-owned
operation contract remains `AIL.RUNTIME.CAPABILITY_CONTRACT`.

### M30-RUNTIME-004 — Synchronous cooperative limit

The interpreter is synchronous and the provider must cooperate with timeout
and cancellation. A timeout value is not an interpreter clock or hard deadline.
The interpreter does not promise to preempt a stuck provider, asynchronously
observe cancellation, or roll back remote effects.

## Inspection and retained revisions

### M30-PROTO-001 — Deterministic outbound inspection

Source-unit and source-set inspection expose operation kind, receiver and
operation permission, timeout and cancellation indices and parameter types,
maximum timeout, result variant identity, and designated timeout/cancel case
identities. Observed calls expose the same outbound permission, timeout value,
cancellation token identity, arguments, and returned or synthesized result in
dynamic order. Cancellation contents are never exposed.

### M30-PROTO-002 — Revision-bound capability environment

Each immutable source-set revision stores the complete validated capability
environment and its stable digest. The digest includes deterministically ordered
interface, operation signature, operation kind, outbound indices, maximum, and
case identities. Source vector or registration order cannot change it.
Inspection and execution of a retained revision use its own environment and
report its requested revision and digest; a child cannot alter its parent.

### M30-PROTO-003 — Complete validation and rejection matrix

Capability metadata is validated before a source set is published. Failure
publishes no revision. The executable matrix covers invalid indices, wrong
timeout and cancellation types, invalid result/cases, missing permission/effect,
timeout zero and over maximum, malformed external cancellation, unsupported
provider, malformed returned value/outcome, real provider fault, timeout and
cancel completion, ordinary-call compatibility, deterministic inspection, and
retained-revision environment binding.

## Non-goals

M30 adds no general networking, URLs, transports, retries, asynchronous calls,
concurrency, clocks, cancellation construction/query, provider interruption,
hard-preemption guarantee, or remote rollback guarantee.
