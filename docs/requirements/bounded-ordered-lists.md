# Bounded ordered list requirements

These requirements come from
[UC-009](../use-cases/UC-009-bounded-ordered-batch-cancellation.md). The numbered
M29 rules in [`specs/bounded-lists.md`](../../specs/bounded-lists.md) define the
compiler behavior.

## Traceability summary

| Requirement | Primary constraint |
| --- | --- |
| APP-007 | ordered one-outcome-per-position cancellation with zero-call rejection |
| LANG-007 | immutable bounded list values and deterministic sequential map |
| PROTO-008 | revision-bound list, module, dependency, effect, and execution facts |
| NFR-008 | deterministic bounded work and honest completion |

## APP-007 — Bounded ordered batch cancellation

Status: **Accepted**

Source use case: UC-009.

Requirement: The service accepts 0 to 32 job identifiers, processes each input
position in order, makes one cancellation call through its existing single-item
path, and returns one closed outcome at the matching position. Duplicates are
preserved. Oversized input or any malformed element makes zero calls. An
unexpected provider fault stops before later positions and does not report a
partial list as complete.

Rationale and acceptance evidence: Without a first-class bounded value and
ordered operation, an agent must manually unroll a fixed number of jobs or move
batch behavior into opaque host code. The M29 runtime tests assert exact output
and call traces for empty, mixed, duplicate, boundary, rejection, and fault
cases.

Dependencies and scope: Depends on the existing closed variants, capability
calls, modules, ordinary calls, and deterministic interpreter. It does not add
atomic multi-job storage, duplicate rejection, retries, or concurrency.

## LANG-007 — Immutable bounded lists and sequential map

Status: **Accepted**

Source use case: UC-009.

Requirement: `List<T, N>` is an immutable ordered structural value type whose
runtime length is 0 through `N`. `T` is one named value type and `N` is a
positive decimal integer no greater than 4,294,967,295. Type equality requires
equal resolved element identity and equal bound. `map binding in source { body }`
evaluates `source` once and evaluates `body` once per stored element in index
order, producing `List<U, N>` with the same actual length.

The binder is immutable and obeys the existing no-shadowing rules. Calls and
effects in the body use ordinary call and capability semantics. `map` and `in`
are contextual so existing declarations and parameters with those names remain
valid. Nested lists, list literals, indexing, mutation, append, filter, fold,
general loops, implicit bound widening, and parallel evaluation are excluded.

Rationale and acceptance evidence: The structural type makes cardinality and
element identity compiler facts. The binder removes the need for closures or
first-class functions while preserving deterministic effect order. Parser,
formatter, checker, composition, recursion/effect traversal, and runtime tests
exercise the rule directly.

## PROTO-008 — Revision-bound list and map facts

Status: **Accepted**

Source use case: UC-009.

Requirement: Inspection exposes canonical `List<T, N>` types, list-bound syntax,
map expression and binder types, linked module and function identities, resolved
element schema identity when present, declared effects, capability parameters,
and deterministic body dependencies. Source-set execution names the requested
immutable revision and returns ordered values and observed calls.

Rationale and acceptance evidence: Agents should not parse type strings or
reload multiple files to recover the bound, canonical element, helper path, or
authority. M29 tests inspect both a source unit and the linked cancellation
entry point, reverse source input order, and execute retained revisions with
different exact bounds.

Dependencies and scope: Depends on revision handles, source-set linking,
semantic graph facts, and M28 qualified references. The API is in-process Rust;
a stable external wire protocol remains unresolved.

## NFR-008 — Deterministic bounded batch work

Status: **Accepted**

Source use case: UC-009.

Requirement: Identical canonical sources, revision, ordered input, and supplied
capability outcomes produce equal ordered results and call traces. Complete
ordinary input validation occurs before capability availability checks or
calls. Cardinality failure takes precedence over element failure. A successful
map reports every result; a runtime fault reports no partial map result.

Rationale and acceptance evidence: Bounded deterministic behavior prevents
filesystem, hash, or scheduler ordering from becoming hidden input and prevents
late malformed positions from causing partial effects. Tests assert zero
provider checks and calls for oversized and late-invalid inputs, exact behavior
at length 32, and a precise call prefix for provider faults.

Dependencies and scope: The bound limits interpreter work after the host has
constructed `RuntimeValue::List`. A future transport decoder must enforce the
declared bound while decoding if host allocation itself must be bounded.
