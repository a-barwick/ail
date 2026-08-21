# ADR 0014: Architecture on check, publish is a separate write

- Status: Accepted
- Date: 2026-08-21

## Context

`ailc check` already builds an `EvolutionWorkspace`. Architecture evaluation
and atomic candidate publication already exist in-library:
`ArchitectureWorkspace::validate_architecture_change` and
`EvolutionWorkspace::validate_source_architecture_change` publish a child only
when behavior and architecture pass.

The CLI never called those APIs. A type-correct workspace that broke project
architecture printed `ok`. There was no `ailc` command that wrote a revision,
so “failed candidate publishes nothing” was a library-only story.

Policy is project configuration, not AIL source. Capability environment stays
empty. Check must remain read-only.

## Decision

When the named path has `architecture.json` beside it, `ailc check` builds the
workspace with `EvolutionWorkspace::new_with_architecture` and evaluates the
current revision through `ArchitectureWorkspace`. Denied or incomplete
findings fail check with an `AIL.ARCH.*` diagnostic. Missing
`architecture.json` skips architecture, so
`compiler/examples/composed-service` still prints `ok`.

`ailc publish <dir>` runs the same checks. Only a passing candidate writes
`<dir>/.ail/revisions/published`. A failing candidate creates no store and
does not replace an existing store.

The architecture evaluator still requires the existing M26 6/6 behavior gate.
`ailc` does not execute, so that gate is reported as passed and architecture
findings remain the result under test.

## Consequences

`ailc check compiler/examples/architecture-denied` fails with
`AIL.ARCH.BOUNDARY` and writes nothing. Publish of that candidate writes
nothing. Publish of a passing candidate writes a revision whose
`source_set_digest` is bound to the checked sources.

No capability environment file is invented. No IR is invented. Check with no
path still fails; it does not default to the repository root.

## Alternatives considered

- Make `check` publish: rejected. Check stays read-only.
- Require a policy file on every workspace: rejected. Composed-service has
  none and must keep passing.
- Invent a capability env file so architecture examples can run behavior:
  rejected. Empty capability environment remains the CLI rule.
- Prove publication only in library tests: rejected. AB-33 requires the
  `ailc` binary.

## Validation

`cargo +1.87.0 test -p ail-compiler --test ailc_check --test ailc_publish`
proves composed-service still prints `ok`, architecture-denied fails with
`AIL.ARCH.BOUNDARY` and writes no revision, publish of a failing candidate
writes no revision, and publish of a passing candidate writes
`.ail/revisions/published`.
