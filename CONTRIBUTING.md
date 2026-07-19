# Contributing

AIL is currently a design-stage language project. Contributions should reduce
semantic ambiguity or produce evidence for a recorded decision.

Read [docs/README.md](docs/README.md) before contributing. Every document or
example must identify its layer and authority. Application illustrations,
proposed language syntax, normative rules, conformance fixtures, and prototype
behavior are not interchangeable.

## Before proposing a language feature

Specify:

1. the accepted use case and numbered requirement;
2. the semantic problem being solved;
3. why it belongs in the language, protocol, runtime, or standard library;
4. the single canonical textual form, if source syntax is involved;
5. static and dynamic behavior;
6. effects, faults, and boundary visibility;
7. structured diagnostics for invalid use;
8. formatter behavior; and
9. how the feature changes total agent work.

Syntax preference alone is not sufficient justification.

Proposed use cases, requirements, and design sketches may be reviewed together,
but the design cannot become normative until its use case and requirements are
accepted.

## Milestone workflow

Before implementation work, read [docs/STATUS.md](docs/STATUS.md) and the active
milestone in [docs/roadmap.md](docs/roadmap.md). Work within that milestone,
keep checkpoints independently buildable, run its focused and repository-wide
checks, and update both documents when its exit criterion passes. The
repository-wide documentation and local-link check is:

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
Do not add root dependency files for a candidate stack before M13 accepts the
stack-selection ADR that supersedes ADR 0001.

## Commits

Keep documentation, prototype evidence, and normative semantic changes
separable where practical. Commit messages should describe the decision or
behavior changed rather than only the files touched.
