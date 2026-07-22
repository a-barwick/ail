# M19 compiler-guided schema-evolution contract

Status: **Accepted 2026-07-21**

This contract defines only the semantic facts and protocol results needed to
analyze and validate the accepted UC-003 priority evolution. It extends M11 and
M17 without changing their rules or fixtures.

The canonical workspaces under `evolution-fixtures/r1` and
`evolution-fixtures/r2` use one build-wide declaration namespace across an
ordered set of source paths. This is not a package or module system.

## Grammar extension

```text
record-decl  = "record" Identifier [identity] "{" field+ "}" ;
variant-decl = "variant" Identifier [identity] "{" variant-case+ "}" ;
field        = Identifier [identity] ":" type ";" ;
variant-case = Identifier [identity] ["(" type ")"] ";" ;
identity     = "identity" String ;
```

`identity` is contextual in these four positions and remains an ordinary
identifier elsewhere. An identity string matches
`[a-z][a-z0-9]*(?:[.-][a-z][a-z0-9]*)*`. A declaration identity is unique in
the build. Field and case identities are unique within their declaration. The
complete identity of a field or case is its declaration identity, `/`, and its
local identity. A schema-bearing declaration has an identity and every field or
case in it has an identity. Non-schema declarations may omit identities.

## Source-set encoding

Paths are normalized, repository-relative UTF-8 strings using `/`, with no
empty, `.` or `..` component. They are sorted by UTF-8 byte order. Each source
is canonical and ends in one line feed.

The source-set digest is SHA-256 over this unambiguous byte sequence for each
source in path order:

```text
utf8(path) || NUL || ascii(decimal byte length) || NUL || utf8(source)
```

The externally encoded digest is `sha256:` followed by lowercase hexadecimal.

## Relationship graph

An edge is `source`, `kind`, `target`, and `site`. `source` and `target` are a
persistent schema identity or a revision-scoped handle. `site` is a
revision-scoped handle. The accepted kinds, in ordering precedence, are:

1. `declares-member`;
2. `signature-input`;
3. `signature-output`;
4. `constructs`;
5. `reads-field`;
6. `matches-case`;
7. `capability-argument`;
8. `declares-effect`;
9. `adapts-from`;
10. `projects-to`;
11. `verifies`;
12. `source-artifact`.

Edges sort by the source site's path, start UTF-8 byte, kind precedence, target
identity, then target handle kind and local ID. Reverse impact traversal visits
edges in that order and emits each location once at its first minimal path.

## Accepted language rules

### M19-LANG-001 — Explicit stable schema identities

Schema-bearing records, variants, fields, and cases carry canonical identities
with the grammar and uniqueness rules above. Display-name changes do not change
an identity. Reusing one identity for an incompatible declaration is a static
schema error.

Traceability: APP-004, LANG-001, LANG-005, PROTO-003.

### M19-LANG-002 — One analyzable source-set namespace

All ordered sources in the revision form one declaration namespace. The
accepted M11 and M17 constructs create complete typed relationships for the
edge kinds above. This slice has no reflection, generated declarations,
imports, or dynamically selected fields.

Traceability: LANG-001, LANG-005, PROTO-001, PROTO-002.

### M19-LANG-003 — Schema changes preserve explicit consequences

Adding a required field or closed case creates semantic consequences at every
typed construction, complete match, adapter, projection, capability boundary,
and declared verification artifact. A conforming implementation may not erase
those relationships during lowering or formatting.

Traceability: APP-004, LANG-002, LANG-005, PROTO-002.

## Accepted protocol rules

### M19-PROTO-001 — Immutable ordered source-set revisions

An evolution request names one immutable revision with the source-set digest
defined above. Handles remain revision scoped. Persistent schema identities are
separate values and may relate declarations across revisions.

Traceability: APP-005, PROTO-001, PROTO-003.

### M19-PROTO-002 — Deterministic semantic graph

Inspection exposes the complete in-scope relationship graph using the accepted
edge kinds and ordering. Every edge names its contributing source site.

Traceability: LANG-005, PROTO-001, PROTO-002, PROTO-005.

### M19-PROTO-003 — Exact categorized impact

`impact` accepts a base revision, a persistent subject identity, and one typed
proposed schema change. It returns ordered `must_change`, `review`, and
`unchecked` lists. `must_change` contains only locations that cannot validate
unchanged. `review` contains typed dependents whose required edit depends on
policy or representation. `unchecked` contains unavailable consumers and an
explicit reason. Every entry includes one minimal relationship path.

Traceability: APP-004, APP-005, PROTO-002, PROTO-005.

### M19-PROTO-004 — Honest analysis coverage

The report identifies all analyzed source paths and all declared external or
unavailable boundaries. An analysis gap cannot produce a complete claim. The
M19 fixture permits at most two unnecessary `review` entries and no unnecessary
`must_change` entry.

Traceability: APP-005, PROTO-002, PROTO-005.

### M19-PROTO-005 — Atomic candidate source-set validation

`validate_change` accepts the current base revision and a complete ordered
candidate source set. The compiler canonicalizes and validates the entire set
before publishing one child revision. Stale, incomplete, statically invalid,
identity-incompatible, effect-changing, or behaviorally failing candidates
publish no revision and return no edits.

Traceability: APP-004, APP-005, PROTO-003, PROTO-004, PROTO-005.

### M19-PROTO-006 — Derived canonical edits and identity map

A successful transaction returns non-overlapping canonical UTF-8 byte edits
ordered by path and start byte. Applying them from last to first per path
reproduces the candidate source set. Its identity map classifies every indexed
base handle and reports child-only handles; persistent identities are reported
separately as preserved, added, or retired.

Traceability: APP-005, PROTO-003, PROTO-004.

### M19-PROTO-007 — Typed semantic diff

The semantic diff reports added, removed, or changed public contracts, schema
identities, fields, cases, effects, capabilities, adapters, and projections.
The accepted priority evolution explicitly reports that storage authority and
observable effect ordering are unchanged.

Traceability: APP-004, APP-005, PROTO-001, PROTO-004.

### M19-PROTO-008 — Revision-bound validation and completion evidence

Success returns parse, type, capability, impact, and public-behavior validation
for the child revision, plus the base impact report, canonical edits, identity
map, semantic diff, analyzed paths, and unchecked boundaries. Evidence from a
different revision cannot satisfy completion.

Traceability: APP-005, PROTO-004, PROTO-005, NFR-001.

### M19-PROTO-009 — Stable rejection causes

The primary rejection codes are `AIL.PROTOCOL.STALE_REVISION`,
`AIL.IMPACT.MISSED_CONSUMER`, `AIL.SCHEMA.IDENTITY_INCOMPATIBLE`,
`AIL.CAPABILITY.EFFECT_GROWTH`, and `AIL.VALIDATION.BEHAVIOR_MISMATCH`.
Each rejection identifies the base revision, current revision, validation
phase, semantic location, expected and actual facts, and causal chain.

Traceability: APP-005, PROTO-003, PROTO-004, PROTO-005.

### M19-PROTO-010 — Deterministic result ordering

For identical source sets and requests, graph edges, impact entries, edits,
identity mappings, semantic changes, diagnostics, and validation evidence are
byte-for-byte equal. Ordering uses source path and UTF-8 byte position before
the category-specific tie breakers fixed in this contract.

Traceability: LANG-004, PROTO-001, PROTO-005, NFR-001.

## Boundaries

M19 adds no Rust implementation, modules, imports, general serialization or
migration framework, code generation, reflection, collections, concurrency,
production I/O, native lowering, architectural metrics, or project policy.
