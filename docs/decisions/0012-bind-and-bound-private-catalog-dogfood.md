# ADR 0012: Bind and bound private catalog dogfood

- Status: Accepted contract
- Date: 2026-08-16

## Context

M32 can answer real key lookups from an operator catalog, but source revision
identity alone does not identify the data that produced an answer. Its audit
vector can grow without a bound and silently ignores a poisoned record lock.
Clients must also reconstruct result identity from array position. Those gaps
make repeated use of the endpoint for a private engineering runbook harder to
audit than the underlying compiler execution.

## Decision

The parsed catalog owns a SHA-256 identity over a domain-separated canonical
key/value encoding. The service copies that provider-owned identity at startup
and returns it in successful responses and execution records. The running
catalog and its digest remain immutable until restart.

The host admits no more than 256 executions per process. It validates the
request first, then atomically reserves an audit slot before token creation,
provider work, or compiler execution. Full or unavailable auditing returns 503.
Every attempted compiler execution completes its reserved record before the
host can return 200.

Every JSON outcome carries its original request key. Exact result length is
validated before outcomes are paired, so correlation never depends on a
truncating zip. Ordering and duplicate keys remain unchanged.

The executable accepts an optional explicit port but continues to bind only to
`127.0.0.1`. This permits simultaneous private-catalog dogfood when the default
port is already serving another catalog without broadening network exposure.

## Consequences

An operator can tell which source and catalog snapshots produced an answer and
can correlate each outcome without recreating array positions. Audit memory is
bounded. Capacity exhaustion deliberately stops useful work until the process
is restarted; M33 does not add audit eviction because silently discarding old
private results would weaken the current audit guarantee.

The audit remains process-local and retains complete result payloads. The
catalog remains suitable only for low-consequence private operating context,
not secrets or regulated data. The endpoint is still loopback-only and
unauthenticated.

## Alternatives considered

- Hash the input JSON bytes: rejects semantically equivalent formatting and
  entry order as different snapshots.
- Pass a configured digest beside the provider: permits the reported identity
  to diverge from the data being queried.
- Drop oldest records at capacity: returns successful work after discarding
  audit evidence.
- Check capacity after execution: permits unaudited dependency work and races
  beyond the bound.
- Add a key only to `Found`: leaves failure and duplicate correlation dependent
  on position.
- Add persistent audit storage or authentication: broader than the local
  dogfood workflow and not required to learn from it.

## Validation

`cargo +1.87.0 test -p ail-service-host --test m33_private_catalog_dogfood`
proves the catalog, outcome, capacity, and concurrency behavior. The host unit
test proves poisoned audit failure. The M32 focused suite and full workspace
checks remain regression gates.
