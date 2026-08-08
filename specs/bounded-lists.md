# M29 bounded ordered list contract

Status: **Accepted 2026-08-08**

This contract adds only the immutable bounded list type and deterministic
sequential map needed by the
[bounded batch-cancellation workload](../docs/workloads/bounded-batch-cancellation.md). It
depends on the shipped M28 module, import, alias, qualified-reference, ordinary
call, transitive-effect, recursion-rejection, and source-set execution behavior.
It does not add general generics, iteration, mutable collections, concurrency,
or a production runtime.

## Syntax and canonical type model

`List` is a contextual structural type constructor. `map` and `in` are
contextual in the binder form, so all three spellings remain available as
ordinary identifiers outside that form.

```text
type                = qualified-name
                    | "List" "<" type "," UnsignedDecimal ">" ;
map-expression      = "map" Identifier "in" expression block ;
```

The initial element type must be one existing named value type. A direct nested
`List` element is rejected. Bounds use canonical positive decimal notation and
the language maximum is 4,294,967,295. The parser retains the element and bound
as structured syntax; compiler consumers do not recover them by parsing a flat
type string.

### M29-LANG-001 — Structural bounded list type

`List<T, N>` denotes immutable values containing zero through `N` elements of
`T` in stored order. Values preserve duplicates. `T` may be built-in, record,
variant, imported, aliased, or qualified. Linking resolves only `T` to its
canonical declaration identity; it does not qualify `List` or change `N`.
`N` is at least 1 and at most 4,294,967,295. Zero or an out-of-range value emits
`AIL.TYPE.LIST_BOUND`. A nested list element emits `AIL.TYPE.LIST_ELEMENT`.

Traceability: LANG-007, PROTO-008, NFR-008.

### M29-LANG-002 — Exact list type equality

`List<T, N>` equals `List<U, M>` only when the linked element identities are
equal and `N == M`. There is no implicit bound widening, narrowing, or other
conversion. Existing mismatch diagnostics render the complete canonical types.

Traceability: LANG-007, PROTO-008.

### M29-LANG-003 — Typed sequential map

For `map binding in source { body }`, `source` must have type `List<T, N>` or the
compiler emits `AIL.TYPE.MAP_SOURCE`. The immutable binding has inferred type
`T`, is visible only in `body`, and obeys existing duplicate and no-shadowing
rules. The body must produce one named value type `U`; a list result is rejected
with `AIL.TYPE.LIST_ELEMENT`. The expression type is `List<U, N>`.

The source is evaluated once. At runtime, the interpreter creates a fresh body
scope and evaluates the complete body once for each stored element at indices
`0..length`, finishing an index before starting the next. Each body evaluates
its `let` initializers and tail expression in source order. Map has no omission,
successful early exit, break, continue, append, mutation, filter, fold, index,
length, literal, or parallel form.

Traceability: APP-007, LANG-007, NFR-008.

### M29-LANG-004 — Calls, effects, and recursion remain visible

Calls in the source and body use ordinary M28 checking and execution. Imported
calls resolve through aliases and qualified names. Reachable capability effects
must appear in the enclosing function effect clause. Recursion detection,
dependency collection, qualification, rename, graph construction, impact
traversal, and inspection descend through both map source and body.

Map introduces no authority and no static multiplication of effect or call-site
facts by `N`. The set-valued effect model does not prove a maximum invocation
count per position. UC-009's one-call property comes from its single body call
path and exact dynamic trace tests.

Traceability: APP-007, LANG-006, LANG-007, PROTO-008.

## Runtime behavior

The host represents a list as `RuntimeValue::List` without bound metadata. The
expected static type supplies the element and maximum. Validation is recursive
through records and variants.

### M29-RUNTIME-001 — Complete pre-effect external validation

Execution checks ordinary argument count, then validates every ordinary value
parameter in declaration order before any capability availability check or
capability call. For a list, cardinality is checked before elements; elements
are checked in ascending index order. Therefore an oversized or late malformed
input produces an empty observed-call list and cannot change provider state.

Oversize emits `AIL.RUNTIME.LIST_CARDINALITY` with expected `element_type` and
`maximum`, and actual `count` and `value_path`. A malformed element emits
`AIL.RUNTIME.LIST_ELEMENT` with expected `element_type`, actual zero-based
`index` and `actual_type`, and the nested value path. Cardinality takes
diagnostic precedence over element shape.

Traceability: APP-007, NFR-008.

### M29-RUNTIME-002 — Ordered aligned completion

An empty list maps to an empty list and makes no body or capability call. A
successful non-empty map returns the same actual length as its input. Output
position `i` is the result of the body bound to input position `i`. Observed
capability calls retain dynamic multiplicity and execution order.

Traceability: APP-007, LANG-007, NFR-008.

### M29-RUNTIME-003 — Fail-stop map faults

If body evaluation faults at index `i`, execution returns that original fault
with actual field `map_index = i`. Calls before and at the failing provider call
remain observable, no later body runs, and no partial list is returned as a
completed result. No rollback is implied. Closed expected cancellation outcomes
therefore remain ordinary variant values rather than provider faults.

Traceability: APP-007, NFR-008.

## Inspection, revisions, and deterministic ordering

### M29-PROTO-001 — Inspectable list and map facts

Source-unit inspection exposes canonical explicit or inferred list types, a
`list-bound` syntax handle, a `map` expression with inferred result type, and a
`map-binding` symbol with inferred element type. Dependencies include named
input and output elements and body references.

Source-set function inspection exposes requested revision, function handle,
linked module and function identity, value parameter and result types,
structured element type, optional stable schema identity, maximum length,
declared effects, capability parameters, and sorted linked dependencies.

Traceability: PROTO-008.

### M29-PROTO-002 — Revision-scoped source-set execution

Execution selects one retained immutable source-set revision. Source-vector
order does not change canonical source-set identity, linking, graph facts,
inspection, or execution. Different retained revisions enforce their own exact
list bounds and all success and failure results name the requested revision.

Traceability: PROTO-008, NFR-008.

### M29-PROTO-003 — Existing atomic validation is preserved

List and map syntax participate in complete source-set parsing, formatting,
linking, static checking, graph construction, revision storage, and the existing
candidate validation paths. A statically invalid candidate cannot publish a
child. M29 does not broaden the specialized schema-change request vocabulary or
claim that runtime capability effects can be rolled back.

Traceability: PROTO-003, PROTO-005, PROTO-008, NFR-008.

## Accepted service behavior

The application bound is 32. The canonical program is the three files under
`compiler/examples/batch-cancellation/`. The host's `JobsStore.cancel` signature
is `(cancellation.domain.JobId) -> cancellation.domain.CancelOutcome`.

The executable matrix covers canonical and contextual syntax; zero,
out-of-range, nested, and wrong-source types; empty and exact-bound lists;
ordered mixed outcomes; duplicates; late malformed and oversized inputs; a
provider fault in the middle; transitive effects; linked aliases and stable
element identities; source-order independence; inspection; and retained
revision execution. All pre-M29 tests and the 37 public job-service cases remain
unchanged.

## Non-goals and limits

This contract does not provide general collections or loops, list literals,
indexing, length queries, mutation, accumulation, filtering, nested lists,
parallel evaluation, uniqueness, cross-element validation, effect-count proofs,
catch-and-continue faults, transactional provider rollback, transport decoding,
native execution, or deployment. Static architecture call-site facts count the
map body site once; they are not multiplied by the runtime bound.
