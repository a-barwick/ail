# M17 deterministic interpreter contract

Status: **Accepted 2026-07-21**

This contract adds only the language, runtime, and protocol behavior needed to
execute the accepted job-service reference slice. It extends the fixed M11
contract; it does not change the M11 construct count, rules, fixtures, or
protocol shapes.

The accepted canonical service is
[`runtime-fixtures/job-service.ail`](runtime-fixtures/job-service.ail). JSON,
Base64, request-version selection, deterministic capability implementation, and
response projection remain host concerns. Request validation, V1 request and
stored-job adaptation, job construction, capability invocation, and closed
outcome mapping remain visible in canonical AIL source.

## Grammar extension

```text
expression          = M11-expression
                    | field-access
                    | if-expression
                    | match-expression ;
field-access        = expression "." Identifier ;
if-expression       = "if" expression block "else" block ;
match-expression    = "match" expression "{" match-arm+ "}" ;
match-arm           = Identifier "::" Identifier
                      ("(" Identifier ")")? "=>" block ","? ;
```

`Bool` and `Bytes` are built-in named types. M17 adds no Boolean or byte
literal: Boolean values are produced by the fixed pure intrinsics below, and
byte values enter through typed execution arguments. The fixed intrinsic set is
`text.is_empty`, `text.byte_length_between`,
`text.first_ascii_alphanumeric`, `text.rest_ascii_alphanumeric_or`,
`text.scalar_count_gt`, `text.contains_control`, and `bytes.length_gt`.

## Accepted language rules

### M17-LANG-001 — Values, fields, and fixed pure intrinsics

`Bool` and `Bytes` use exact named-type equality. Field access requires a
record value and a declared field and has that field's declared type. The fixed
intrinsics have the signatures recorded in `runtime-contract.json`, are pure,
and add no capability effect. Text byte counts use UTF-8 bytes, scalar counts
use Unicode scalar values, ASCII predicates use ASCII code points, control
testing uses Unicode control values, and byte length counts opaque bytes.

Traceability: APP-001, LANG-001, LANG-003, LANG-004.

### M17-LANG-002 — Typed conditional expressions

An `if` condition has type `Bool`. Both branches are blocks, are checked in
isolated nested scopes, and must produce the same exact type. Only the selected
branch evaluates. `AIL.TYPE.IF_CONDITION` and
`AIL.TYPE.IF_BRANCH_MISMATCH` are stable diagnostics.

Traceability: APP-001, LANG-001, LANG-004, PROTO-005.

### M17-LANG-003 — Closed exhaustive matching

A match target has a declared closed variant type. Every declared case appears
exactly once, every arm names that variant, and payload binding exactly matches
the case payload shape. Arm blocks have one exact result type. Arms are tested
by declared case identity with no catch-all. `AIL.TYPE.NON_EXHAUSTIVE_MATCH` and
`AIL.TYPE.MATCH_ARM_MISMATCH` are stable diagnostics.

Traceability: APP-003, LANG-002, LANG-004, PROTO-005.

### M17-LANG-004 — Canonical and statically valid executable source

Field access is printed as `target.field`. Conditional and match blocks use the
two-space layout in the canonical fixture; match arms retain source order and
end with `,`. Formatting is idempotent. Execution is available only for a
revision with no parse, name, type, or capability diagnostic. The reference
service's complete bounded validation and compatibility adapters are ordinary
checked AIL source, not runner behavior.

Traceability: APP-001, APP-004, APP-005, LANG-001, NFR-001.

## Accepted runtime rules

### M17-RUNTIME-001 — Deterministic left-to-right evaluation

Bindings evaluate in source order. Record initializers evaluate in declaration
field order, call arguments evaluate left to right, a conditional evaluates one
branch, and a match evaluates one selected arm. Runtime records use declared
field identities and runtime variants use declared case identities. Identical
revision, arguments, and supplied capability behavior produce identical values,
faults, and ordered calls.

Traceability: APP-005, LANG-004, NFR-001.

### M17-RUNTIME-002 — Supplied capabilities and ordered calls

Capability instances are supplied separately from ordinary value arguments and
must match the parameter receiver and interface. A call is recorded immediately
before host invocation after all arguments evaluate. Its result is recorded
after the provider returns and must match the declared operation result type.
The reference handler performs zero calls for invalid requests and exactly one
`jobs.insert_if_absent` call for valid requests.

Traceability: APP-001, APP-002, LANG-003, LANG-004.

### M17-RUNTIME-003 — Stable uncaught runtime faults

Runtime faults are distinct from closed domain results and contain a stable
code, source span, expected facts, and actual facts. M17 faults are not
catchable. Unknown revisions or functions, invalid argument counts or values,
missing capabilities, invalid capability results, integer overflow, and
violations of statically guaranteed record, field, conditional, match, or
intrinsic shapes return faults rather than changing domain results.

Traceability: APP-003, APP-005, PROTO-005.

## Accepted protocol rules

### M17-PROTO-001 — Revision-scoped execution

`execute` names one retained immutable revision and one function display name.
The function and all returned handles belong to that revision. Publishing a
child revision does not change execution of a retained parent. Unknown or
statically invalid revisions fail before capability invocation.

Traceability: APP-005, PROTO-001, PROTO-003.

### M17-PROTO-002 — Structured deterministic execution results

Success returns status, revision, function handle, runtime value, and ordered
capability calls. Failure returns status, revision, requested function, runtime
fault, and calls observed before the fault. The transport-independent shapes
are fixed by `runtime-protocol.json`; repeated identical requests have equal
results.

Traceability: APP-005, LANG-004, PROTO-001, PROTO-005, NFR-001.

## Boundaries

M17 does not add general function calls, collections, loops, recursion,
concurrency, production I/O, native lowering, catchable faults, ambient
authority, or architectural-health analysis.
