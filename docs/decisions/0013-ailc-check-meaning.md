# ADR 0013: Hold `ailc check` until its checker is chosen

- Status: Proposed
- Date: 2026-08-21

## Context

`ailc check` only calls `parse`. A parseable program prints `ok` even when
names, types, effects, modules, or recursion would fail the library checkers.
That lets a human or agent treat the compiler as having accepted a program it
did not check.

Two existing checkers can close that gap. They are not the same command:

1. `check_source(source, revision_id, capabilities)` checks one source unit.
   `semantics.rs` never reads `module` or `import`. Unused imports are ignored.
   A referenced imported name such as `model.Request` becomes
   `AIL.NAME.UNRESOLVED`, not a module diagnostic. `compiler/examples/composed-service/domain.ail`
   would pass. `service.ail` from the same example would fail as a unit even
   though `EvolutionWorkspace` accepts the three-file set.

2. `EvolutionWorkspace::new(workspace_id, revision_id, sources, capabilities, coverage)`
   validates paths, parses, canonicalizes, runs `validate_modules` (identity,
   imports, cycles, ambiguity, visibility), links with `link_source_set`, then
   calls `check_parsed_source`. Failure publishes no revision. The composed-service
   example matches this API. A single file with an unresolved import fails as
   `AIL.MODULE.MISSING_IMPORT`. Semantic diagnostics from the linked unit are
   currently attributed to path `<source-set>`. Source paths must pass
   `valid_source_path` (relative, no `.` / `..`, no leading `/`). Coverage is an
   explicit completeness claim.

Neither API invents capability syntax. Both take a caller-supplied
`CapabilityEnvironment`. The current driver has no way to pass one. An empty
environment would reject capability-using examples for a different reason than
parse-only `ok`.

Wiring either checker, or silently switching on "file vs directory", would
change the public meaning of `ailc check` without Austin choosing that meaning.

## Decision

Not chosen. Do not wire `check_source` or `EvolutionWorkspace` into `ailc check`
until Austin picks what `check` means.

`ailc check` stays parse-only. That is still a lie about semantics, but it is
the current command, not a new half-checker.

A draft that called `check_source` with an empty capability environment was
started and reverted. That draft is not the meaning of `check`.

## Consequences

Agents and humans who run `ailc check` still only get parse results. Library
callers continue to use `check_source` for one unit and `EvolutionWorkspace`
for an atomic source set.

The next implementation change is blocked on an explicit choice of option 1,
option 2, or a later third meaning Austin writes down. A hybrid that uses one
API for a file and the other for a directory is also a choice; it is not a
default.

## Alternatives considered

These remain open. Neither is selected.

- Option 1, `check_source`: smallest single-file wire. Reports real unit
  diagnostics. Still lies about modules and imports. Does not check the
  composed-service program as a program.
- Option 2, `EvolutionWorkspace`: same acceptance rule as the multi-file
  examples and atomic candidate validation. Requires a source-set rule for the
  CLI (explicit files, a directory of `*.ail`, or something else), a coverage
  claim, and relative path mapping. Not a unit checker.
- File vs directory hybrid: two meanings under one verb. Rejected as a silent
  default; Austin may still choose it later.
- Keep parse-only forever: leaves the original contradiction in place.

## Validation

This record is wrong if `ailc check` calls `check_source`,
`check_parsed_source`, or `EvolutionWorkspace`, or if docs claim that `check`
already means one of those.

`compiler/ail-compiler/src/bin/ailc.rs` must keep using `parse` for `check`
until a later accepted decision names the checker.
