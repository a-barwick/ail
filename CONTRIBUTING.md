# Contributing

AIL has a working Rust compiler for a narrow language. M28 is complete: the
compiler checks and executes local and imported calls across explicit modules,
enforces transitive effects, and rejects recursion and import cycles. No next
milestone is active.

Contributions should ship a tested compiler capability, remove ambiguity from an
existing contract, or improve a measurement tied to a real change. Start with
[the current limits and next move](docs/STATUS.md).

Read [docs/README.md](docs/README.md) before contributing. Examples do not define
language behavior; numbered rules and conformance fixtures do.

## Before proposing a language feature

Specify:

1. the accepted use case and numbered requirement;
2. the semantic problem being solved;
3. why it belongs in the language, protocol, runtime, or standard library;
4. the single canonical textual form, if source syntax is involved;
5. static and dynamic behavior;
6. effects, faults, and boundary visibility;
7. structured diagnostics for invalid use;
8. formatter behavior;
9. how the feature changes total agent work;
10. which strong existing-language tools form the baseline; and
11. the executable result that would prove the design wrong.

Syntax preference alone is not sufficient justification.
Neither are native code generation, LLVM integration, self-hosting, feature
parity, or source brevity without a representative agent-change rationale.

Design sketches do not change compiler behavior. Add numbered rules and
conformance fixtures before implementation when the change extends a locked
contract.

## Delivery workflow

Before implementation work, read [docs/STATUS.md](docs/STATUS.md). If no build is
active, obtain a concrete scope instead of starting an old deferred plan. Keep
checkpoints independently buildable and run the focused and repository-wide
checks. The documentation and local-link check is:

```bash
python3 tools/check_docs.py
```

## Use cases and requirements

Use cases should define system boundaries, observable behavior, representative
agent changes, operational constraints, and success evidence without choosing
AIL syntax.

Requirements must be observable, measurable where practical, traceable to use
cases, and neutral about implementation mechanism unless the mechanism is itself
the requirement.

## Design changes

Use an architecture decision record for changes to externally observable
semantics, determinism, the compiler protocol, repository architecture, or the
implementation stack.

Examples must identify whether they are:

- behavior illustrations;
- illustrative and non-normative AIL;
- proposed normative fixtures; or
- accepted conformance fixtures.

Architectural-health changes must preserve the distinction in
[the proposed manifest specification](docs/architecture-health.md): the compiler
defines primitive semantic facts, while versioned project policy decides which
facts record, warn, or deny. Do not use a single score or source-size limit as a
substitute for aggregate authority, state, dependency, and context analysis.

## Prototypes

Keep experimental implementations under `prototypes/` and follow its README.
ADR 0004 selects Rust for the compiler. Keep prototypes isolated and state the
technical question and executable result they test.

## Commits

Keep documentation, prototype evidence, and normative semantic changes
separable where practical. Commit messages should describe the decision or
behavior changed rather than only the files touched.
