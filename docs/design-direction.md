# AIL design and current implementation

AIL is deterministic executable software, not a specification language or agent
protocol. Canonical source is the durable program. The compiler exposes the
typed, revision-bound model that agents and humans use to inspect and change it.

## Compiler pipeline

```text
canonical AIL source
  -> lossless parse and canonical format
  -> module and name resolution
  -> type, call, effect, and capability checks
  -> revision-bound semantic graph
  -> deterministic interpreter
```

The pipeline has no native backend. LLVM, bytecode, source emission, JIT
execution, and self-hosting are not implemented.

## Source and modules

AIL uses one canonical representation for each supported construct. Formatting
is part of the language contract, not a style preference.

Each file in a multi-file source set declares one `module name;` header and zero
or more `import dependency;` or `import dependency as alias;` headers. Imports
expose records, variants, and functions by declared name. Non-imported
declarations are inaccessible. Qualified references such as `domain.Request`
and `alias.validate` resolve collisions explicitly. Bare colliding references
are rejected. Import cycles are rejected.

Independent modules may reuse declaration names. Entry points can use
`module.function` when an unqualified name is not unique.

## Calls and effects

Calls use `function(arguments)` syntax. The compiler resolves local and imported
functions, checks exact value argument counts and types, and propagates
same-named capability parameters. Every caller must declare all capability
effects reachable through its callees.

The interpreter evaluates arguments left to right and executes nested calls
deterministically. The compiler rejects direct and mutual recursion. AIL does
not provide bounded recursion or general iteration.

Capabilities identify the accessible instance or namespace. Code cannot obtain
store, clock, network, filesystem, environment, randomness, or telemetry access
unless its contract receives that authority. Pure functions cannot call
effectful functions.

## Bounded ordered lists

`List<T, N>` is an immutable ordered value with runtime length `0..=N`. The
bound is part of exact type identity, and the compiler resolves imported,
aliased, and qualified element types structurally. Initial list elements must be
named value types; there is no implicit bound widening or nested list type.

`map item in items { body }` evaluates the source once and the body sequentially
by stored index. It preserves the source bound and actual length, and ordinary
calls in the body retain their dependencies and transitive capability effects.
External list cardinality and every element are validated before capability
availability checks or calls. The syntax does not add literals, indexing,
mutation, filtering, folding, arbitrary loops, or parallel evaluation.

## Revisions and changes

Compiler results identify an immutable source revision. Syntax and symbol handles
from one revision cannot be applied silently to another.

A validated rename produces canonical byte edits, checks the complete candidate,
publishes one child revision, and returns an identity map. Invalid names,
collisions, stale handles, and failed checking publish nothing.

The schema-evolution API extends this rule to ordered multi-file source sets. It
reports `must_change`, `review`, and `unchecked` impact categories, validates
caller-supplied behavior evidence, returns semantic and textual diffs, and
publishes only a complete valid candidate.

## Determinism

Logical execution is deterministic relative to explicit inputs:

```text
initial state + ordered input + supplied capability outcomes
  = result + final state + ordered capability calls
```

This requirement covers observable language behavior. It does not claim
bit-identical native output, because no native backend exists. Future floating
point, broader collections, concurrency, time-zone data, filesystem access, and
resource exhaustion rules must define their observable inputs before they can
preserve this property.

## Diagnostics

Diagnostics carry a stable code, source revision, semantic location, category,
expected and actual facts where applicable, related declarations, and causal
relationships. Text is a rendering of those fields. Agents should branch on
structured data, and humans should be able to read both.

## Architecture checks

The implemented API computes architecture facts for executable units, modules,
dependency components, and configured groups. It reports contributors for seven
implemented metrics, compares compatible revisions, evaluates project policy,
and blocks publication on denied or incomplete results.

This catches helper splitting that leaves authority, state, and dependencies in
the same transport group. It does not infer the best architecture or define a
universal complexity limit. See [architecture-health.md](architecture-health.md)
for the implemented boundary and the larger unimplemented design.

## Required safety properties

These are real design requirements even where the necessary language features
are not implemented yet:

- ordinary AIL has no undefined behavior;
- integer overflow cannot be silent;
- public errors are closed typed values;
- external authority is explicit;
- nondeterministic inputs are explicit and recordable;
- concurrency is structured and bounded;
- potentially unbounded loops, recursion, queues, retries, tasks, and memory are
  visible to policy; and
- incomplete analysis cannot be reported as complete.

Do not infer current support from this list. Today the compiler avoids several
of these problems by not implementing the corresponding feature.

## Unresolved design

The project still needs exact rules for memory and aliasing, integer widths and
additional numeric types, general collections and iteration, concurrency and
cancellation, replay, packages and dependency versions, foreign code, runtime
resource limits, protocol versioning, and native builds. Those decisions must be
made with executable fixtures, not examples alone.
