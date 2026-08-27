# Published-bytes runner

`ail-run` executes a published revision. It reads only the frozen bytes under
`<dir>/.ail/revisions/<current>/sources/`. It never opens the live `.ail` files
next to that store, so an edit that was never published cannot change what runs.

`ailc` still does not execute anything. `check` and `publish` are the only
`ailc` commands that read live source, and neither evaluates a program.

## Command

```bash
target/debug/ail-run compiler/examples/job-review scenarios.review_fixture \
  --bytes 6a6f622d7061796c6f6164
```

```
completed
revision_id=published
source_set_digest=sha256:d04ad0c8928eab29b6d8e5e069d86ea702ebe928031100ab5e500ab3b92cfb88
function=scenarios.review_fixture
value=contracts.ReviewDecision::Approved(contracts.ApprovedJob { job_id: "fixture-job", payload: bytes:6a6f622d7061796c6f6164, priority: contracts.Priority::Normal, requested_by: "queue-agent", reviewer: "release-bot", task: "compile" })
calls=0
```

Arguments are `--text VALUE`, `--int VALUE`, and `--bytes HEX`, in declaration
order. There is no command-line spelling for a record, variant, or list
argument; an entry point that needs one is reachable only through
`load_published_program` in Rust.

## What the runner verifies before it runs

| Refusal | Cause |
| --- | --- |
| `AIL.RUN.NO_PUBLISHED_REVISION` | No `.ail` store, no `current` pointer, or `current` names a revision with no stored sources. |
| `AIL.RUN.UNREADABLE_STORE` | `revision.json` is unreadable, disagrees with `current`, lists an invalid or duplicate path, or the sources directory holds a file the revision does not list. |
| `AIL.RUN.FROZEN_SOURCE_DIGEST` | One frozen file's bytes disagree with the `sha256` `ailc publish` recorded. |
| `AIL.RUN.FROZEN_SOURCE_SET_DIGEST` | The loaded set disagrees with the recorded `source_set_digest`. |
| `AIL.RUN.CAPABILITY_ENVIRONMENT_DIGEST` | The revision was checked under a capability environment other than the empty one the runner supplies. |
| `AIL.RUN.FROZEN_SOURCE_REJECTED` | The frozen bytes no longer pass the compiler checks. |

A refusal writes nothing to stdout and exits 1. The runner never falls back to
the live files.

## Capabilities

The runner supplies no capability instances and builds an empty
`CapabilityEnvironment`. There is no capability file path, project manifest, or
host config, and this change did not invent one. A published function that
declares a capability parameter fails with the existing
`AIL.RUNTIME.MISSING_CAPABILITY` fault at that parameter.

Today no published revision can reach that fault through `ailc`: `ailc publish`
also checks under the empty environment, so a capability-using source set fails
`AIL.CAPABILITY.UNKNOWN_INTERFACE` before anything is frozen. The runner does
not use the pinned lookup or batch-lookup host.

## Reproduction

Published bytes, from the checked-in example:

```bash
cargo +1.87.0 build --workspace
target/debug/ail-run compiler/examples/job-review scenarios.review_fixture \
  --bytes 6a6f622d7061796c6f6164
```

Live edit that was never published:

```bash
cp -r compiler/examples/job-review /tmp/job-review-live-edit
sed -i 's/fixture-job/live-edit-job/' /tmp/job-review-live-edit/scenarios.ail
target/debug/ail-run /tmp/job-review-live-edit scenarios.review_fixture \
  --bytes 6a6f622d7061796c6f6164
target/debug/ailc check /tmp/job-review-live-edit
```

The run prints `job_id: "fixture-job"`, the published value. `ailc check`
prints `ok` and `behavior: not-run 0/6` and no program value, so the live
`live-edit-job` never appears anywhere.

No published revision:

```bash
cp -r compiler/examples/job-review /tmp/job-review-unpublished
rm -rf /tmp/job-review-unpublished/.ail
target/debug/ail-run /tmp/job-review-unpublished scenarios.review_fixture \
  --bytes 6a6f622d7061796c6f6164
```

Recorded output is in
[`compiler/examples/job-review/transcripts/`](../compiler/examples/job-review/transcripts):
`published-run.txt`, `live-edit-run.txt`, `live-edit-check.txt`, and
`unpublished-run.txt`.

## Executable checks

```bash
cargo +1.87.0 test -p ail-compiler --test published_runner
```

- `the_published_job_review_runs_from_the_frozen_bytes` — the checked-in
  published example runs and reports the frozen revision and source-set digest.
- `a_store_without_any_live_source_file_still_runs` — a directory holding only
  `.ail/` runs, so the value cannot have come from a live file.
- `an_unpublished_live_edit_does_not_change_what_runs` — the live
  `scenarios.ail` returns `live-edit-job` on disk and the run still returns
  `fixture-job`.
- `ailc_check_does_not_run_the_program_after_a_live_edit` — on that same folder
  `ailc check` prints exactly `ok` and `behavior: not-run 0/6`, prints neither
  job id, and `ailc` has no command that executes.
- `a_directory_with_no_published_revision_is_refused`,
  `a_current_pointer_without_stored_sources_is_refused`,
  `edited_frozen_bytes_are_refused`,
  `a_frozen_source_the_revision_does_not_list_is_refused`,
  `a_revision_checked_under_another_capability_environment_is_refused` — each
  refusal path.
- `the_loaded_program_reports_the_frozen_source_set_it_will_run` and
  `the_runner_supplies_no_capabilities` — the loaded facts and the empty
  capability environment.

## What this does not do

- It does not prove the language. It proves one runner reads frozen bytes and
  one already-published example produces one value.
- It is not a production service. There is no server, request path,
  concurrency, persistence, retry, deployment, or operator surface.
- It adds no language feature and no syntax. The interpreter is the existing
  tree-walk interpreter.
- It runs only the revision `<dir>/.ail/current` names. There is no revision
  selection flag, no revision history, and no rollback.
- It does not freeze `architecture.json`. Project architecture policy is a live
  file that `ailc check` and `ailc publish` evaluate; it is not part of the
  frozen source set and does not change program behavior, so the runner ignores
  it. The `architecture_settings_digest` recorded in `revision.json` therefore
  refers to a file the runner does not read.
- It has no `--json` output. Its stdout is the plain `key=value` form above.
- It does not check that `ailc publish` produced the store. Any directory whose
  `.ail` store satisfies the digests above will run.
