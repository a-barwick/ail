# M11 transport-independent compiler protocol

Status: **Proposed normative rules for the compiler-stack spikes**

This field model defines the semantic interface M12 candidates must expose. It
does not select JSON-RPC, HTTP, a process boundary, or an on-disk database. The
JSON fixtures are only the repository's deterministic conformance encoding.

## Shapes

`Revision`
: `workspace_id`, `revision_id`, optional `parent_revision_id`, and
  `source_digest`. A revision is immutable.

`Handle`
: `revision_id`, `kind`, and opaque `local_id`. Kinds in M11 are `symbol`,
  `syntax`, and `expression`.

`Diagnostic`
: `code`, `revision_id`, `category`, `primary_handle`, `expected`, `actual`,
  ordered `related_handles`, ordered `causal_chain`, and optional `cascade_of`.
  Human prose is optional and never authoritative.

`InspectionRequest`
: `revision_id` and `handle`.

`InspectionResult`
: `revision_id`, `handle`, `semantic_kind`, `explicit_type`, `inferred_type`,
  ordered `effects`, ordered `capabilities`, and ordered `dependencies`.

`RenameRequest`
: `base_revision_id`, a symbol `handle`, and `new_name`.

`RenameSuccess`
: `status`, `base_revision_id`, new `revision`, ordered canonical UTF-8 byte
  `edits`, `identity_map`, `diagnostics`, and `validation`.

`RenameFailure`
: `status`, `base_revision_id`, `current_revision_id`, one primary
  `diagnostic`, and empty `edits`. It creates no revision.

`IdentityMap`
: `from_revision_id`, `to_revision_id`, ordered `entries`, and ordered
  `new_handles`. Each old handle entry is `surviving`, `replaced`, `removed`, or
  `unmapped`; surviving and replaced entries have a new handle.

The exact required fields and enum values are duplicated in
[`protocol.json`](protocol.json) so the contract checker can verify them.

## Proposed protocol rules

### M11-PROTO-001 — Immutable explicit revisions

Every request and result names one source revision. A successful edit creates a
new immutable child revision with a digest of its canonical source set. It does
not mutate the base revision.

Traceability: PROTO-001, PROTO-003, PROTO-004, APP-005.

### M11-PROTO-002 — Revision-scoped handles

A handle is valid only in the revision named by its `revision_id`. Equality of
`local_id` across revisions has no meaning without an identity map. Persistent
schema or serialization identities are separate from M11 handles and are not
introduced by this contract.

Traceability: PROTO-003.

### M11-PROTO-003 — Elaborated inspection

Inspection returns explicit boundary types and locally inferred types without
adding semantic facts in prose. Empty effect, capability, and dependency sets
are returned as empty ordered lists. Inspection of a handle from another
revision is rejected as stale.

Traceability: PROTO-001, LANG-001, LANG-003.

### M11-PROTO-004 — Structured diagnostics

Diagnostics use the fields defined above and stable codes selected by the
language rules. `expected` and `actual` are typed field maps, not formatted
sentences. Related handles and causal steps are ordered by source position,
then handle kind, then `local_id`.

Traceability: PROTO-005, APP-005.

### M11-PROTO-005 — Rename preconditions

Rename accepts only a symbol handle from `base_revision_id` and a lexically
valid non-keyword `new_name`. It must resolve every reference in the complete
M11 source unit. A name collision, invalid name, non-symbol handle, or
incomplete reference set rejects the operation.

Traceability: LANG-005, PROTO-002, PROTO-003, PROTO-004.

### M11-PROTO-006 — Atomic validated rename

Rename rewrites the declaration and all resolved references, canonicalizes the
complete source unit, and reruns the M11 parse and static checks. Success
publishes one child revision. Any diagnostic rejects the transaction and
returns no partial revision.

Traceability: PROTO-004, PROTO-005, APP-005.

### M11-PROTO-007 — Canonical edit representation

Each edit names a source path, half-open UTF-8 byte range in the base revision,
and replacement text. Edits are sorted by path and ascending start byte, do not
overlap, and reproduce the returned canonical source when applied from last to
first within each file.

Traceability: PROTO-004, APP-005.

### M11-PROTO-008 — Complete identity map

For every handle named by the request or returned inspection evidence, the
identity map reports `surviving`, `replaced`, `removed`, or `unmapped`.
`new_handles` contains identities with no old counterpart. A renamed symbol is
`surviving`; syntax reference nodes whose source bytes change are `replaced`.

Traceability: PROTO-003, APP-005.

### M11-PROTO-009 — Stale revision rejection

If `base_revision_id` is not the workspace's current revision at commit time,
rename returns `AIL.PROTOCOL.STALE_REVISION`, empty edits, and no new revision.
The diagnostic identifies the requested and current revisions. The compiler
must not silently remap and retry.

Traceability: PROTO-003, PROTO-004, PROTO-005.

### M11-PROTO-010 — Deterministic protocol ordering

For identical revision content and request fields, all handles, diagnostics,
edits, identity-map entries, and inspection lists use deterministic ordering.
Opaque local IDs may differ between compiler candidates, so fixture assertions
name logical fixture IDs; a candidate-neutral adapter maps its internal IDs to
those logical IDs.

Traceability: LANG-004, PROTO-001, PROTO-005, NFR-001.

## Validated rename example

**Proposed fixture:** `fixtures/rename.json` renames the record symbol `Job` to
`StoredJob`, rewrites its declaration and three references, returns canonical
text, and classifies the semantic symbol as surviving while changed syntax
nodes are replaced.

## Stale edit example

**Proposed fixture:** `fixtures/stale-revision.json` submits an R1 handle while
R2 is current. It returns `AIL.PROTOCOL.STALE_REVISION`, no edits, and no child
revision.

These two examples exercise rename transaction mechanics only. General semantic
diff, move, extraction, multi-file schema evolution, and concurrent client
policy remain outside M11.
