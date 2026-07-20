# M11 five-construct core contract

Status: **Proposed normative rules for the compiler-stack spikes**

This document defines exactly five AIL language constructs for M12. It is not
the broader job-service language core. The rules are proposed so two candidate
compiler implementations can be compared against one contract; acceptance of
the eventual language core remains later work.

The five constructs are:

1. record declarations and record construction;
2. closed variant declarations and variant construction;
3. function declarations with explicit public signatures;
4. local `let` bindings with local type inference; and
5. capability operation calls.

Text and integer literals, identifiers, punctuation, named types, and the
function body delimiters are lexical or supporting forms. They do not add
independent declaration, control-flow, or expression constructs. General
calls, field access, matching, conditionals, loops, modules, imports, generic
types, typed holes, and execution are outside M11.

## Why these five

The subset serves accepted UC-001 and UC-003 requirements while keeping the
stack experiment bounded:

- records and variants expose explicit public data and closed outcomes;
- functions test explicit boundary types and declared effects;
- `let` tests local inference and elaborated inspection;
- capability calls test absence of ambient authority and a stable capability
  diagnostic; and
- declarations plus references give validated rename enough semantic structure
  to test revision-scoped handles and source rewriting.

The subset reduces stack-decision risk, not source tokens. It tests the work
that is expensive to rewrite later: lossless syntax, canonical printing,
semantic identity, local checking, diagnostics, and structural edits. A human
can audit every fixture as ordinary canonical text and compare it with the
machine-readable expected result.

## Lexical and grammar notation

Identifiers match `[A-Za-z_][A-Za-z0-9_]*`. The keywords in this document are
reserved. `Text`, `Int`, and `Unit` are built-in named types. Text literals use
JSON string escaping and Unicode scalar values. Integer literals are unsigned
base-10 digits with no leading zero unless the value is zero. Line comments
start with `//` and end before the line terminator. Whitespace and comments are
trivia retained by the lossless syntax tree.

The grammar uses `?` for optional, `*` for zero or more, and comma-separated
suffix `,` in the ordinary EBNF sense.

```text
source              = declaration* EOF ;
declaration         = record-declaration
                    | variant-declaration
                    | function-declaration ;

record-declaration  = "record" Identifier "{" field-declaration* "}" ;
field-declaration   = Identifier ":" type ";" ;
record-expression   = Identifier "{" field-initializer-list? "}" ;
field-initializer-list
                    = field-initializer ("," field-initializer)* ","? ;
field-initializer   = Identifier ":" expression ;

variant-declaration = "variant" Identifier "{" variant-case* "}" ;
variant-case        = Identifier ("(" type ")")? ";" ;
variant-expression  = Identifier "::" Identifier
                      ("(" expression ")")? ;

function-declaration
                    = "fn" Identifier "(" parameter-list? ")"
                      "->" type effect-clause? block ;
parameter-list      = parameter ("," parameter)* ","? ;
parameter           = Identifier ":" parameter-type ;
parameter-type      = type | "capability" Identifier ;
effect-clause       = "effects" "{" effect-list? "}" ;
effect-list         = effect ("," effect)* ","? ;
effect               = Identifier "." Identifier ;
block               = "{" let-binding* expression "}" ;

let-binding         = "let" Identifier "=" expression ";" ;

capability-call     = Identifier "." Identifier
                      "(" argument-list? ")" ;
argument-list       = expression ("," expression)* ","? ;

expression          = TextLiteral
                    | IntLiteral
                    | Identifier
                    | record-expression
                    | variant-expression
                    | capability-call ;
type                = Identifier ;
```

The grammar is intentionally not extensible by candidate implementations.
Fixtures supply capability interface signatures as compiler input because a
capability-interface declaration would be a sixth language construct.

## Proposed language rules

### M11-LANG-001 — Lossless source representation

The parser must represent every source byte exactly once as a token, trivia, or
explicit invalid token. Every syntax node has a half-open UTF-8 byte span in
the source revision. Reconstructing the source from the lossless tree must
produce byte-identical input, including non-canonical whitespace and comments.

Traceability: LANG-001, PROTO-001, PROTO-003, PROTO-005.

### M11-LANG-002 — Deterministic recovery

On a missing required token, the parser inserts one zero-width missing token at
the first position where that token is required, emits
`AIL.PARSE.EXPECTED_TOKEN`, and resumes at the next token in the enclosing
construct's recovery set. For a record field, the recovery set is `:`, `;`,
and `}`. The primary diagnostic is the earliest error by byte position; ties
use grammar order. Static checking does not run for a source with a parse
diagnostic.

Traceability: PROTO-005 and the deterministic evidence requirement APP-005.

### M11-LANG-003 — Record declaration and construction

A record declaration introduces one named type and an ordered, closed field
set. Field names are unique. A record expression must name every declared field
exactly once and no undeclared field. Each initializer must have the declared
field type. Field order in a record expression is not semantic; the formatter
rewrites it to declaration order.

Traceability: LANG-001, LANG-005, PROTO-001.

### M11-LANG-004 — Closed variant declaration and construction

A variant declaration introduces one named type and an ordered, closed case
set. Case names are unique. A case has either no payload or exactly one named
payload type. A variant expression must name a declared case and supply exactly
the declared payload shape. Matching and exhaustive consumption are deferred;
M11 tests only the closed producer contract needed for APP-003.

Traceability: APP-003, LANG-001, LANG-002, PROTO-001.

### M11-LANG-005 — Explicit function boundary

Every function declaration is a public boundary in the M11 source unit. Every
parameter and result type is explicit. A capability parameter is written
`name: capability CapabilityType`. Capability types and operation signatures
come from the fixture environment. The final expression in the body must have
the declared result type.

Traceability: APP-003, LANG-001, LANG-003, PROTO-001.

### M11-LANG-006 — Declared capability effects

The optional effect clause contains the complete ordered set of capability
operations the function body may call. Each entry is the capability parameter
name and operation name, for example `jobs.insert`. Duplicate effect entries
are invalid. A function with no capability call omits the clause. An empty
clause is canonicalized away.

Traceability: APP-002, LANG-003, PROTO-001.

### M11-LANG-007 — Local `let` inference

A `let` binding has no source type annotation in M11. Its type is the fully
checked type of its initializer. The name is visible only to following `let`
initializers and the final expression in the same body. The compiler inspection
result must expose the inferred type.

Traceability: LANG-001 and PROTO-001.

### M11-LANG-008 — Capability call checking

For `receiver.operation(arguments)`, the receiver must resolve to a capability
parameter, the operation must exist in the supplied capability interface, and
the argument types must equal the operation parameter types. The call's type is
the operation result type. The operation's effect must appear in the enclosing
function effect clause. Missing declaration produces
`AIL.CAPABILITY.UNDECLARED_EFFECT` after parse, name, and ordinary type checks
succeed.

Traceability: APP-002, LANG-003, LANG-004, PROTO-005.

### M11-LANG-009 — Name resolution

Top-level type and function names share one source-unit namespace. Function
parameters and local bindings share one function namespace and may not
duplicate an earlier name in that function. References resolve to exactly one
declaration. M11 has no shadowing, imports, overloads, or implicit members.

Traceability: LANG-001, LANG-005, PROTO-001, PROTO-003.

### M11-LANG-010 — Type equality and primary type diagnostic

M11 type equality is exact named-type equality. There are no implicit
conversions. A mismatched record initializer emits
`AIL.TYPE.FIELD_MISMATCH` at the initializer expression with the record and
field as related identities. The primary static diagnostic is selected in this
order: unresolved name, duplicate declaration, ordinary type mismatch,
capability misuse. Within one category, the lowest byte position wins.

Traceability: LANG-001, LANG-003, PROTO-005.

### M11-LANG-011 — Canonical declaration formatting

Canonical source uses UTF-8, LF line endings, two-space indentation, one ASCII
space between keywords and names, no trailing whitespace, and one final line
terminator. Record fields and variant cases each occupy one indented line and
end in `;`. Top-level declarations are separated by one empty line and retain
source declaration order.

Traceability: LANG-001, APP-005, NFR-001.

### M11-LANG-012 — Canonical expression formatting

Parameters, arguments, field initializers, and effects use `, ` separators with
no trailing comma when they fit the fixture line. Colons use no preceding and
one following space. `let` uses exactly one space around `=` and ends in `;`.
Record initializers are ordered by the record declaration. The function header,
opening brace, and first body line use the layout shown by the fixtures. The
formatter is idempotent: formatting canonical source produces identical bytes.

Traceability: LANG-001, PROTO-004, APP-005.

### M11-LANG-013 — Canonical literals

Text literals use JSON escapes, lowercase escape letters, and no unnecessary
escaping. Integer literals use the shortest valid base-10 spelling. Literal
types are `Text` and `Int` respectively.

Traceability: LANG-001, LANG-004.

### M11-LANG-014 — Static checking boundary

M11 checks parsing, declarations, name resolution, record and variant
construction, function result types, local inference, and capability effects.
It does not execute functions. A successful type result means only that this
bounded static contract passed.

Traceability: APP-005, PROTO-001, PROTO-005.

## Canonical example

**Proposed fixture:** the complete machine-readable expected result is
`fixtures/positive.json`.

```text
record Job {
  job_id: Text;
}

variant StoreOutcome {
  Inserted;
  Duplicate;
}

variant CreateJobResult {
  Created(Job);
  AlreadyExists(Text);
}

fn create_job(job_id: Text, jobs: capability JobsStore) -> CreateJobResult effects { jobs.insert } {
  let job = Job { job_id: job_id };
  let outcome = jobs.insert(job);
  CreateJobResult::Created(job)
}
```

This example does not define handler execution, persistence behavior, matching,
or unused-binding policy. It exists only to exercise the M11 compiler surfaces.
