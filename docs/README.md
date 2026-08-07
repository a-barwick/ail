# AIL documentation

Start with the running system, then follow the link to the exact contract.

## Current reader path

1. [Project intent](project-intent.md) — what AIL optimizes and why.
2. [Application vision](application-vision.md) — the software AIL is meant to
   build and the first workloads used to test it.
3. [Design direction](design-direction.md) — implemented design and unresolved
   technical choices.
4. [Compiler README](../compiler/README.md) — current APIs and commands.
5. [Current status](STATUS.md) — shipped capability, hard limits, and next move.

Use [use cases](use-cases/README.md) for application behavior,
[requirements](requirements/README.md) for exact constraints, and
[`specs/`](../specs/README.md) for numbered language and compiler-interface
rules. [Decisions](decisions/) record expensive technical choices. The
[roadmap](roadmap.md) is a compact delivery history, not a second specification.

## What defines behavior

Only numbered specification rules and their conformance fixtures define AIL
language or compiler-interface behavior. The Rust compiler must match those
rules where they apply. M28 call and module behavior is locked by its executable
tests while that material is incorporated into the broader specification.

The other documents have narrower jobs:

- use cases state application inputs, outputs, effects, and changes;
- requirements state observable constraints;
- design documents explain intended system properties and unresolved choices;
- examples illustrate behavior but do not invent syntax;
- benchmark fixtures define application results, not AIL semantics; and
- decision records explain why a technical choice was made.

If an example conflicts with a specification fixture, the fixture wins. If
implementation behavior conflicts with a numbered rule, fix the implementation
or change the rule explicitly; do not bless the accident in prose.

## Compiler state

The current Rust compiler implements:

- lossless parsing and canonical formatting;
- type, name, capability-effect, call, module, and import checking;
- immutable source revisions and revision-scoped semantic handles;
- deterministic interpretation of the supported language;
- schema-impact analysis and atomic multi-file candidate validation; and
- the M24 architecture snapshot, delta, policy, and publication rules.

M28 added direct and imported calls, explicit modules, import aliases, qualified
references, transitive effect checking, left-to-right argument evaluation, and
nested execution. The compiler rejects recursion and import cycles.

It does not implement iteration, general collections, concurrency, networking,
packages, foreign code, a production runtime, native lowering, or deployment.
The full architectural-health catalog is also not implemented; only the M24 set
is.

## Precise terms

- **Canonical source** is the one formatted textual representation stored in
  version control.
- **Normative** means a numbered rule or fixture requires the behavior.
- **Conformance** means an implementation matches those required results.
- **Deterministic** means identical declared inputs produce identical ordered
  logical results. It does not imply bit-identical native binaries.
- **Revision-bound** means a result names the exact immutable source revision it
  describes and cannot be reused silently after an edit.
- **Coverage** names what the compiler analyzed and every boundary it could not
  inspect.

Use these words only with those concrete meanings.

## Documentation checks

Run:

```bash
python3 tools/check_docs.py
```

The command checks local links, decision-record structure, and agreement between
`roadmap.md` and `STATUS.md`.
