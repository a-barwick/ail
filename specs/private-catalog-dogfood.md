# M33 private catalog dogfood contract

Status: **Accepted 2026-08-16**

M33 makes the M32 catalog service usable for a small private engineering
runbook without adding language semantics, hot reload, authentication, or
persistent audit storage. The catalog remains operator-owned and immutable for
the process lifetime.

The executable accepts an optional loopback port after the catalog path. It
still binds only `127.0.0.1`; omitting the port preserves `3000`. Invalid, zero,
or out-of-range ports stop startup.

### M33-CATALOG-001 — Semantic catalog identity

The catalog provider computes `catalog_digest` from a domain-separated,
canonical encoding of the parsed key/value map. JSON whitespace, object-field
order, and entry order do not change the digest. Changing any key or value does.
The provider supplies its own digest to the host so the data snapshot and
reported identity cannot be configured independently.

Every 200 response and every completed execution record contains the same
`catalog_digest`. Editing the source file does not change a running host's map
or digest. A restart loads and identifies the new snapshot.

### M33-CATALOG-002 — Bounded fail-closed audit

The process admits at most 256 compiler executions. After strict transport and
request validation, the host atomically reserves one audit position before it
creates a cancellation identity, locks the dependency provider, or invokes the
compiler. A full audit returns 503 `audit_capacity` with zero compiler or
dependency work. Concurrent requests cannot overbook the bound.

A failed compiler execution consumes its reserved position and retains its
failure record. If audit reservation or completion is unavailable, the host
returns 503 `audit_unavailable`; it never reports an unaudited 200. Reading a
poisoned audit store reports an error rather than an empty record set. Restart
clears the process-local records. A host failure before compiler execution
releases its reservation instead of consuming hidden audit capacity.

Malformed requests still return 400 and semantically out-of-bounds requests
still return 422 even when audit capacity is full. They do not reserve an audit
position.

### M33-CATALOG-003 — Self-identifying ordered outcomes

Every HTTP outcome includes the exact key from its corresponding input request.
The host verifies the compiler result length before pairing inputs and results.
`Found`, `NotFound`, `Unavailable`, `TimedOut`, and `Cancelled` all carry the
key; `Found` additionally carries its value. Input order and duplicate keys are
preserved.

Example:

```json
{
  "revision_id": "r1",
  "source_set_digest": "sha256:...",
  "catalog_digest": "sha256:...",
  "outcomes": [
    {"case": "Found", "key": "ail.full-check", "value": "cargo test --workspace"},
    {"case": "NotFound", "key": "ail.missing"}
  ]
}
```

## Canonical proof

`service-host/tests/m33_private_catalog_dogfood.rs` checks semantic digest
stability and change detection, file-snapshot immutability, response/audit
binding, every keyed outcome case including duplicates, exact capacity with
zero provider work, validation precedence, and concurrent admission. A unit
test checks poisoned-audit visibility, fail-closed HTTP behavior, pre-execution
reservation release, and strict optional port parsing. The M32 suite remains
the regression gate.

## Private-use boundary

The endpoint binds only to loopback, but it has no authentication and returns
catalog values in HTTP responses, process memory, and terminal-visible output.
An operator may use an untracked `0600` catalog for local paths, commands,
private local URLs, and low-consequence operating notes. Catalogs must not
contain credentials, access tokens, private keys, customer records, regulated
data, purchase information, or child-related information.

## Non-goals

No secret storage, redaction, encryption, authentication, audit endpoint,
plaintext audit file, database, mutation endpoint, prefix search, structured
catalog values, hot reload, general networking, or public deployment.
