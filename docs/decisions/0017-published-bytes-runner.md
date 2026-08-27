# ADR 0017: The runner executes published bytes, not live files

- Status: Proposed
- Date: 2026-08-27

## Context

`ailc publish` freezes a checked source set under
`<dir>/.ail/revisions/published/sources/` and records each file digest plus the
ordered source-set digest in `revision.json`. Nothing executed those bytes. No
`ailc` command evaluates a program: `check`, `publish`, `format`, and
`reconstruct` all read live source and none of them runs it. The interpreter was
reachable only from Rust tests that assembled `EvolutionSource` values
in-process.

That left the frozen store unusable as the thing that runs. A caller who wanted
to run `compiler/examples/job-review` had to re-read the live `.ail` files, which
means an edit nobody published would run.

## Decision

A separate binary, `ail-run`, executes one published revision. It reads
`<dir>/.ail/current`, the `revision.json` that pointer names, and only the files
that document lists under `sources/`. It never reads a `.ail` file in `<dir>`
itself.

`ailc` gains no execution command. `check`, `publish`, `format`, and
`reconstruct` remain the whole of `ailc`. All four read live source; none of them
evaluates a program. `ail-run` executes, and it reads frozen published bytes
only.

Before running, `load_published_program` verifies:

1. `current` names a revision directory that exists.
2. `revision.json` `revision_id` equals `current`.
3. Every listed source's bytes hash to the recorded `sha256`.
4. The sources directory contains no file the revision does not list.
5. The recorded `capability_environment_digest` equals the digest of the empty
   `CapabilityEnvironment` the runner supplies.
6. The loaded set rebuilds as an `EvolutionWorkspace` whose `source_set_digest`
   equals the recorded one.

Any failure refuses with a stable `AIL.RUN.*` code and the fact the runner
measured. A refusal runs nothing and never falls back to the live files.

Execution uses the existing `EvolutionWorkspace::execute`, which is the existing
tree-walk interpreter. The capability environment is empty and the provider
supplies no instances, so a capability parameter fails with the existing
`AIL.RUNTIME.MISSING_CAPABILITY` fault.

Command-line arguments are `--text`, `--int`, and `--bytes HEX`. Records,
variants, and lists have no command-line spelling; those entry points are
reachable only through the library.

## Consequences

`ail-run compiler/examples/job-review scenarios.review_fixture` returns the
decision the published bytes compute, and keeps returning it after a live edit
changes the live file.

The runner re-runs the compiler checks on the frozen bytes rather than trusting
`revision.json`. That costs one parse and check per run and catches a store that
was edited after publication.

The runner ignores `architecture.json`. Project architecture policy is not part
of the frozen source set and does not change program behavior. The
`architecture_settings_digest` in `revision.json` therefore refers to a live file
the runner does not read.

Because `ailc publish` checks under the empty capability environment, no
capability-using source set can be published today, so no published revision can
reach the missing-capability fault through the CLI. The fault path exists for a
revision published by a library caller that supplied interfaces.

This adds no language feature, no syntax, no capability file path, no project
manifest, and no service.

## Alternatives considered

- `ailc run`: rejected. `ailc` is the non-executing compiler CLI, and a `run`
  verb under the same binary blurs the line the tests have to prove.
- Trust `revision.json` and skip re-checking the frozen bytes: rejected. Then an
  edited store runs and the runner cannot say which bytes it ran.
- Trust the digests and skip the unlisted-file check: rejected. An unlisted file
  in `sources/` means the runner cannot name the revision it loaded.
- Read the live files when no revision exists: rejected. That is the exact
  behavior this change removes.
- Invent a capability manifest beside the sources so the runner could supply
  capabilities: rejected for the reason ADR 0013 gives. No such file exists, and
  adding one would introduce ambient project I/O.
- Wire the pinned batch-lookup host in as the capability provider: rejected. It
  is one pinned revision's host, not a general provider, and the target program
  needs no capability.

## Validation

`cargo +1.87.0 test -p ail-compiler --test published_runner` — 11 checks.

Frozen-bytes provenance: `the_published_job_review_runs_from_the_frozen_bytes`,
`a_store_without_any_live_source_file_still_runs`,
`the_loaded_program_reports_the_frozen_source_set_it_will_run`.

Live edit does not run: `an_unpublished_live_edit_does_not_change_what_runs`.

Check still does not execute:
`ailc_check_does_not_run_the_program_after_a_live_edit`.

Refusal paths: `a_directory_with_no_published_revision_is_refused`,
`a_current_pointer_without_stored_sources_is_refused`,
`edited_frozen_bytes_are_refused`,
`a_frozen_source_the_revision_does_not_list_is_refused`,
`a_revision_checked_under_another_capability_environment_is_refused`.

Capabilities: `the_runner_supplies_no_capabilities`.

Recorded command output is in
[`compiler/examples/job-review/transcripts/`](../../compiler/examples/job-review/transcripts).
