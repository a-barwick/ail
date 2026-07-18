# AIL design direction

Status: **Design input**

Purpose: capture the agreed identity and constraints that precede a normative
language specification.

Documentation layer: language and compiler design. This document proposes how
AIL may satisfy the [project intent](project-intent.md) and
[application vision](application-vision.md). It does not define normative program
behavior. See the [documentation model](README.md).

## Identity

AIL is a deterministic, executable programming language designed for agents to
author, inspect, debug, and refactor, and for an ordinary compiler to translate
into bytecode, machine code, or compatible target source.

AIL is not a specification language, an agent protocol, a prompt format, or a
compiler-agent collaboration. Human authorship ergonomics are secondary, but
human auditability is a hard requirement.

Two choices anchor the design:

1. Canonical text is authoritative, with a first-class structural interface
   exposed by the compiler.
2. Public boundaries are explicit, while local implementation details may be
   inferred and inspected through an elaborated compiler view.

## Optimization target

AIL should optimize the total cost of reaching a correct change, not source
brevity in isolation:

```text
generation + context + diagnostics + repair + regression risk
```

The eventual benchmark suite should measure first-pass parse and compile rates,
tokens to a correct implementation, context needed for local changes,
repair iterations, regression frequency, diagnostic localization, diff noise,
runtime predictability, and mechanically diagnosable target conversion.

## Compilation model

```text
Canonical AIL source
  -> parse and canonicalize
  -> resolve names and types
  -> check effects, capabilities, safety, and concurrency
  -> typed AIL IR
  -> optimize and lower
  -> bytecode, machine code, or compatible target source
```

Given identical source, compiler version, target, dependency lock, and build
configuration, compilation must produce the same result. Builds should be
bit-reproducible wherever the output format permits it.

The compiler applies fixed language semantics. It does not infer product
requirements or reinterpret developer intent.

## Canonical source and semantic interface

Repository source is ordinary text. The compiler additionally exposes modules,
symbols, types, calls, effects, capabilities, data dependencies, control flow,
public contracts, implementations, and diagnostics.

Structural operations include rename, move, expression replacement, parameter
addition, and function extraction. Each operation validates the result and
rewrites canonical text transactionally.

Node and symbol handles are scoped to a source revision. After an edit, the
compiler returns an identity map from old handles to surviving, replaced, or
new handles. Persistent identities exist only where they carry semantic value,
such as ABI, schema, serialization, migration, service, or message identity.

## Low-entropy language surface

AIL selects one canonical representation for each construct. The canonical
formatter is the language's textual encoder, not an optional style tool.
Declaration and import order, escaping, numeric notation, generics, error
propagation, collection construction, function bodies, matching, visibility,
and formatting must not have stylistic aliases.

The grammar should use familiar tokens where model priors make them reliable
while remaining compact, context-free where practical, incrementally parsable,
resistant to cascading errors, free of significant whitespace and textual
preprocessing, and free of optional syntax that changes parsing.

Names remain semantically meaningful rather than minified.

## Explicit boundaries and inferred internals

Anything another unit may depend on is explicit. Anything confined to one
implementation may be inferred.

Public parameter and result types, generic constraints, error variants, effects,
capabilities, escaping ownership or aliasing, visible mutation, concurrency and
ordering guarantees, resource contracts, and nondeterministic inputs are
explicit.

Local variable types, obvious generic arguments, temporary representation,
internal effect propagation, non-escaping lifetimes, internal scheduling,
unobservable evaluation details, internal storage choices, and pure
intermediate values may be inferred.

The compiler provides a fully elaborated view and can temporarily materialize
selected inferred types or effects into source for debugging.

## Determinism and replay

Execution is deterministic relative to explicit inputs:

```text
initial state + ordered input stream = state transitions + output stream
```

Time, randomness, environment variables, filesystem state, network responses,
and scheduler decisions enter through explicit capabilities. Recorded external
values and orderings can reproduce the same logical execution.

## Controlled execution

Concurrency is structured and lexically owned. Parallel result ordering is
defined by declaration order. A task must be joined, cancelled under defined
semantics, or have ownership explicitly transferred. Detached tasks are not the
default.

Race-based behavior is explicit and its outcome is a recordable input. Safe AIL
either forbids shared mutable memory or constrains it so that data races are
impossible.

Potentially unbounded behavior is visible. Loops, recursion, retries, queues,
buffers, worker counts, task counts, memory, and blocking behavior have explicit
bounds or an explicit `unbounded` declaration that policy can reject.

Integer overflow cannot be silent.

## Effects, capabilities, and errors

Public behavior declares its effects. Pure functions cannot call effectful
functions. Capabilities identify the accessible instance or namespace, not just
a broad category, and ambient authority is absent.

Errors are closed, typed values in function signatures. Callers propagate,
transform, or exhaustively handle them. Unchecked exceptions do not form a
hidden control-flow system.

## Safety

Ordinary AIL has no undefined behavior. Overflow, bounds errors, invalid shifts,
invalid references, use-after-free, and races have fixed safe meanings or are
rejected. Wrapping and saturating arithmetic are explicit operations.

Low-level foreign primitives may exist behind narrowly defined requirements and
guarantees, but they do not weaken ordinary source semantics.

## Incomplete programs and diagnostics

Typed holes allow incomplete expressions to survive parsing, type analysis,
indexing, and structural editing. Production builds fail while holes remain.
Hole diagnostics report the expected type, in-scope values, constructors, and
relevant constraints.

Structured diagnostics are authoritative. They include stable diagnostic codes,
the source revision, semantic location, category, expected and actual
semantics, related declarations, a minimal causal chain, bounded repair
categories, and cascading-error relationships. Human prose is a secondary
rendering.

## Agent-oriented program operations

The compiler returns a deterministic, budgeted semantic context slice for a
symbol: its signature, direct types, effects, interfaces, impacted callers,
state invariants, relevant tests, serialization constraints, and conversion
constraints.

Semantic diffs supplement text diffs with public API, effect, error, state,
call-graph, control-flow, and target-compatibility changes.

Multi-file refactors are transactions: the complete transformation commits only
if it validates.

## Architectural health and project policy

Complete impact analysis prevents missed consequences, but it does not prevent a
behaviorally correct change from concentrating control flow, authority, state,
and dependencies in one unit. The compiler should therefore expose primitive
architectural facts for executable units and aggregate semantic scopes.

Given a compatible base and candidate revision, the compiler reports an
architectural delta: new or enlarged hotspots, dependency and capability
changes, state concentration, cycles, context growth, coverage loss, and policy
or baseline changes. Unchanged accepted debt remains visible without becoming a
new regression.

The language creates analyzable relationships; the compiler measures them; the
project decides which thresholds and boundaries to enforce. Universal AIL
semantics do not declare that every large function is invalid or infer the
project's preferred architecture.

Project policy may record facts, warn, or deny validation. Denied results and
incomplete analysis required by a denied rule prevent a structural transaction
from committing. Exceptions are explicit, scoped, versioned policy artifacts,
not source-comment suppressions.

The proposed metric catalog, manifest, baseline, exception, diagnostic, and
validation behavior are defined in the
[architectural health manifest](architecture-health.md). The feature remains
non-normative until UC-007, its requirements, numbered protocol rules, and
conformance fixtures are accepted.

## Targets

Native AIL semantics are authoritative. Compatible target source may be emitted
when the source semantics can be preserved. Conversion failures identify the
unsupported source capability, its location, target coverage, and available
lowering support.

Target ecosystems and lowering packages must not distort the core language
before its semantics are stable.

## Initial exclusions

The first version excludes:

- textual macros;
- wildcard imports;
- implicit numeric conversions;
- operator overloading;
- ambient global capabilities;
- unchecked exceptions;
- multiple equivalent syntaxes;
- undefined behavior;
- detached concurrency by default;
- unrestricted build-script authority;
- comments used as contracts; and
- reflection over arbitrary implementation details.

## Next normative artifact

After the application-use-case and requirements gate, the next normative
artifact is a small core of roughly 20–30 constructs, including:

- a canonical grammar and formatter;
- static and dynamic semantics;
- a type, error, and effect model;
- deterministic evaluation rules;
- a module and visibility model; and
- the compiler's versioned agent-facing semantic protocol.
