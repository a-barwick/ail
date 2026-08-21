# ADR 0013: `ailc check` means an EvolutionWorkspace

- Status: Accepted
- Date: 2026-08-21

## Context

`ailc check` only called `parse`. A parseable program printed `ok` even when
names, types, effects, modules, or recursion would fail the library checkers.

Two existing checkers could close that gap. They are not the same command:

1. `check_source` checks one source unit and never reads `module` or `import`.
2. `EvolutionWorkspace::new` validates an ordered source set atomically, then
   checks the linked unit.

Austin chose option 2. Option 1 is not the meaning of `check`.

There is no AIL project or source-set manifest. Examples name workspace files
by basename (`domain.ail`, `service.ail`, `validation.ail`) and construct
`CapabilityEnvironment` in Rust tests, not beside the sources.

## Decision

`ailc check` builds one [`EvolutionWorkspace`](../../compiler/ail-compiler/src/evolution.rs).
It does not call `check_source`.

Source set:

- `ailc check <dir>`: the `.ail` files in that directory whose names pass
  `valid_source_path`, using those basenames as `EvolutionSource` paths.
- `ailc check <file>`: a one-file workspace named by the file name. Imports
  fail as `AIL.MODULE.MISSING_IMPORT`.

Capability environment: empty. No host or project file supplies one next to
examples. Missing capability interfaces fail as
`AIL.CAPABILITY.UNKNOWN_INTERFACE`.

Coverage: `declared_complete: true` with no unchecked boundaries or artifacts,
the same claim `compiler/examples/composed-service/` uses when the tests
construct the workspace.

Revision id is `check`. Workspace id is the file or directory name.

## Consequences

`ailc check compiler/examples/composed-service` succeeds for the same reason
`EvolutionWorkspace::new` accepts that three-file set. A type, name, effect,
or recursion error fails with the workspace diagnostic. A single imported
file fails as a module error, not as success. Capability-using examples such
as `batch-lookup` fail honestly until a caller supplies the environment
through the library API.

`format` and `reconstruct` stay parse tools. The driver does not execute.

Parse failures surface as `EvolutionBuildFailure` causes
(`<file> has parse diagnostics`), not as `AIL.PARSE.EXPECTED_TOKEN`. That is
the existing workspace API.

## Alternatives considered

- `check_source` as `check`: rejected. It still lies about modules.
- File uses `check_source`, directory uses evolution: rejected. Two meanings
  under one verb.
- Invent a project manifest or capability config file: rejected. The repo has
  none, and adding one would invent ambient project I/O.
- Load `service-host` capability constructors from the CLI: rejected. Those
  are Rust test/host values, not a source-set config.

## Validation

`cargo +1.87.0 test -p ail-compiler --test ailc_check` proves the composed-service
directory succeeds, `service.ail` alone reports `AIL.MODULE.MISSING_IMPORT`, a
type error reports `AIL.TYPE.FIELD_MISMATCH`, recursion reports
`AIL.CALL.RECURSIVE_CYCLE`, and `batch-lookup` reports
`AIL.CAPABILITY.UNKNOWN_INTERFACE`.
