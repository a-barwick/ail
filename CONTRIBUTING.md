# Contributing

AIL is currently a design-stage language project. Contributions should reduce
semantic ambiguity or produce evidence for a recorded decision.

## Before proposing syntax

Specify:

1. the semantic problem being solved;
2. the single canonical textual form;
3. static and dynamic behavior;
4. effects, faults, and boundary visibility;
5. structured diagnostics for invalid use;
6. formatter behavior; and
7. how the feature changes total agent work.

Syntax preference alone is not sufficient justification.

## Design changes

Use an architecture decision record for changes to externally observable
semantics, determinism, the compiler protocol, repository architecture, or the
implementation stack.

Examples must identify whether they are:

- illustrative and non-normative;
- proposed normative fixtures; or
- accepted conformance fixtures.

## Prototypes

Keep experimental implementations under `prototypes/` and follow its README.
Do not add root dependency files for a candidate stack before ADR 0001 is
accepted.

## Commits

Keep documentation, prototype evidence, and normative semantic changes
separable where practical. Commit messages should describe the decision or
behavior changed rather than only the files touched.
