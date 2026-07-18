# Initial specification review

Date: 2026-07-18

Status: **Historical design review**

Scope: review of the finalized design direction supplied at repository creation.
Subsequent project and application framing is captured in
[project-intent.md](project-intent.md) and
[application-vision.md](application-vision.md).

Documentation role: point-in-time review. Findings remain useful, but this file
does not supersede the artifacts it reviews.

## Assessment

The document is a strong language thesis and a useful set of design constraints.
It is not yet an implementable language specification. Its most valuable
decision is the separation between canonical source as the durable artifact and
the compiler's semantic model as the primary agent interface.

The ideas reinforce one another:

- canonical syntax reduces generation entropy and irrelevant diffs;
- explicit boundaries reduce the context needed to use a unit safely;
- local inference keeps ordinary source compact;
- elaboration prevents inference from becoming opaque;
- explicit capabilities expose authority and nondeterminism;
- structured diagnostics and typed holes reduce repair cascades; and
- semantic operations make refactoring safer than coordinated text replacement.

The central delivery risk is scope. The direction currently spans a language,
compiler, formatter, type-and-effect system, safe runtime, concurrency model,
replay system, semantic database, refactoring engine, diagnostic protocol,
context packer, and multiple backends. These should not all be first-release
requirements.

## What is already decided

- AIL is executable and has native semantics.
- Canonical text is authoritative.
- The compiler exposes a typed, revision-scoped semantic interface.
- Public contracts are explicit; local details may be inferred.
- External nondeterminism and authority enter through capabilities.
- Errors are typed values.
- Ordinary code has no undefined behavior.
- Concurrency and resource growth are controlled and inspectable.
- Structured diagnostics are authoritative.
- Target-source emission is optional and subordinate to AIL semantics.

These decisions are sufficient to begin a core semantics document.

## Blocking semantic questions

The following questions must be answered before independent implementations
could agree on program behavior:

1. **Evaluation:** What is the evaluation order of expressions, arguments,
   matching guards, and destructors? Which operations may be reordered?
2. **Values and numbers:** Which primitive types exist? What are integer widths,
   overflow faults, floating-point rules, text encoding, and equality rules?
3. **Memory:** Is the first core ownership-based, garbage-collected, region
   based, or abstract over storage? What aliasing is observable at boundaries?
4. **Modules:** How do packages, modules, imports, visibility, dependency
   versions, initialization, and cycles behave?
5. **Effects:** Are effects inferred only inside a function or can effect
   polymorphism cross boundaries? How are capability instances introduced,
   passed, delegated, and revoked?
6. **Errors and faults:** Which failures are typed domain errors and which are
   language faults? Can faults be caught? Are cancellation and resource limits
   typed?
7. **Concurrency:** What exactly happens on child failure, cancellation,
   simultaneous `select` readiness, scope exit, and capability use across tasks?
8. **Collections:** Are maps and sets ordered? If not, how is iteration kept
   deterministic? Are allocation and hash seeds observable?
9. **Replay:** What is recorded, at which abstraction boundary, with which
   schema/version compatibility and security/redaction rules?
10. **Foreign code:** How can a foreign primitive's requirements and guarantees
    be trusted, checked, sandboxed, and represented in replay?

## Compiler protocol questions

The semantic interface is a product surface and needs an explicit protocol:

- transport and encoding;
- schema and protocol version negotiation;
- revision creation and lifetime;
- concurrency and stale-revision handling;
- handle stability and invalidation;
- transaction, validation, and rollback rules;
- diagnostic causality;
- deterministic context-budget calculation;
- semantic diff compatibility;
- workspace and dependency boundaries; and
- authorization for structural edits.

JSON-RPC is a reasonable bootstrap transport, but the semantic schema should be
transport-independent.

## Determinism traps

“Same inputs produce the same result” needs a narrower observable contract.
Otherwise platform details can leak through:

- floating-point behavior and NaN payloads;
- map/set iteration and hashing;
- filesystem ordering and path normalization;
- locale, Unicode version, and time-zone data;
- environment-dependent linker metadata;
- timestamps and absolute paths in artifacts;
- allocator behavior and resource exhaustion;
- dependency retrieval and build-tool versions; and
- simultaneous concurrency events.

Define logical execution determinism separately from bit-reproducible build
output, then state the supported platforms and artifact normalization rules.

## Scope recommendation

The first executable milestone should contain:

- lexer, parser, lossless source representation, and canonical formatter;
- modules with explicit imports;
- structs, closed enums, functions, calls, blocks, `let`, `return`, `if`, and
  exhaustive `match`;
- a deliberately small primitive type set;
- explicit public signatures with local type inference;
- `Option`, `Result`, typed holes, and checked arithmetic;
- capability declarations and a minimal non-polymorphic effect checker;
- structured diagnostics with revisions and semantic handles; and
- a deterministic tree-walking interpreter.

Defer native code generation, source-to-source targets, general concurrency,
full ownership analysis, replay infrastructure, resource policies, transactional
refactors, and token-budgeted context slicing until the core semantics can run a
small conformance suite.

An interpreter is recommended for the semantic oracle even if a native backend
is the long-term goal. It makes evaluation rules testable before code generation
introduces target-specific behavior.

## Acceptance test for the next phase

The design phase is complete enough to implement when:

1. every core construct has grammar, typing, effect, evaluation, formatting, and
   diagnostic rules;
2. ten small example programs have exactly one canonical rendering;
3. two independent readers can predict the same output and primary diagnostic;
4. the protocol can represent a parsed file, a typed hole, an undeclared effect,
   and a revision-safe rename; and
5. unresolved choices are explicitly marked rather than hidden in examples.
