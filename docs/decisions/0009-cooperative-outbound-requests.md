# ADR 0009: Add cooperative outbound dependency requests

- Status: Accepted contract
- Date: 2026-08-08
- Owners: project maintainers
- Documentation layer and scope: M30 capability metadata, interpreter, provider, revision, and inspection semantics

## Context

AIL capability calls can represent a dependency operation, but ordinary calls
do not distinguish timeout and cancellation controls from domain arguments.
Treating timeout, cancellation, and expected completion as undocumented host
conventions would hide authority and behavior from static checks, traces, and
revision inspection. Adding URL or transport syntax would exceed the selected
workload.

## Decision

M30 retains `client.operation(args)` and the existing capability/effect
permission. Host-supplied operation metadata marks an operation ordinary or
outbound. Outbound metadata designates distinct timeout and cancellation
argument indices, a positive maximum timeout, and distinct payloadless cases in
the declared closed result variant.

`Cancellation` is an opaque external-only value. Source may receive and pass it
but cannot construct or query it. Providers use a separate outbound API and
return `Returned(value)`, `TimedOut`, or `Cancelled`. The interpreter validates
external arguments before authority, validates timeout before recording a call,
and synthesizes the designated closed completion for timeout and cancellation.
Capability environments and their stable digest are stored with each immutable
source-set revision.

## Consequences

- Permission remains explicit as the capability parameter and exact operation
  effect; outbound metadata adds no ambient authority.
- Calls and inspection expose timeout, cancellation, permission, and closed
  result facts deterministically.
- Unsupported outbound providers fail rather than falling back to the ordinary
  provider API.
- Timeout and cancellation completion are expected values. Invalid metadata,
  arguments, provider values, and provider faults remain faults.
- The synchronous interpreter relies on a cooperative provider. It cannot
  enforce a hard deadline against stuck provider code and guarantees no remote
  rollback.

## Alternatives considered

### New outbound call syntax

Rejected. Existing call syntax already exposes receiver, operation, arguments,
and effect authority; new syntax adds no required audit fact.

### Ordinary provider fallback

Rejected. It silently discards the stronger outbound contract.

### Runtime-created cancellation and clocks

Rejected. Both introduce ambient authority and semantics unnecessary for a
host-controlled request.

### Preemptive or asynchronous execution

Rejected. Thread interruption, scheduling, resource cleanup, and remote
rollback cannot be guaranteed by the current synchronous interpreter.

## Validation

The dependency-free
[outbound request contract checker](../../specs/tools/outbound_request_contract.py)
validates the numbered rules, canonical source, protocol shapes, diagnostics,
and complete rejection matrix. M30 implementation tests must realize every
matrix case before the milestone is reported as shipped.
