# Job review refusal and publish records

## What changed

A Cursor agent edited the AIL files in this directory. No Rust harness wrote
the candidate. The first live edit changed `transport.dispatch` to call
`domain.review_job` while the domain function was still named `review`. The
compiler refused that incomplete rename. The fix renamed the domain function
and updated the scenario caller. The final program changes the function name,
not job-review behavior.

The final program is 222 lines across six AIL files. It uses existing modules,
records, closed variants, `if`, `match`, and ordinary calls. This change adds
no language feature, capability, or permission file.

## Project rule

`architecture.json` maps every module and intrinsic endpoint into the five
architecture groups required by the existing checker. Project policy holds
`transport.dispatch` to control-flow complexity 1 and minimal context 6. Those
are existing compiler metrics, not new language rules.

## Live name-refused edit

Commit `24ae9e1` records the failed live edit in `transport.ail`. With that edit
present in this directory, both commands exited 1:

```bash
target/debug/ailc check compiler/examples/job-review
target/debug/ailc publish compiler/examples/job-review
```

The existing `AIL.NAME.UNKNOWN_FUNCTION` check reported that the source set did
not declare `domain.review_job`. This refusal is a name check, not concentration
or `AIL.ARCH.HOTSPOT_GROWTH`. The failed publish left the prior frozen revision
unchanged, so no revision contains the failing state.

`transcripts/live-refused-check.txt` and
`transcripts/live-refused-publish.txt` contain the compiler output from the live
folder. Commit `84bbd7e` records the source fix.

## Type-refused change

`../job-review-type-refused/` is a complete source copy of the published
candidate with one AIL edit. In `scenarios.ail`, `review_fixture` supplies
`contracts.Priority::High` to the `ReviewRequest.priority` field. That field is
declared as `contracts.PriorityOption`; the candidate does not wrap the value
in `PriorityOption::Some`. No test writes or assembles this candidate.

Run:

```bash
target/debug/ailc check compiler/examples/job-review-type-refused
target/debug/ailc publish compiler/examples/job-review-type-refused
```

Both commands exit 1 with `AIL.TYPE.FIELD_MISMATCH` at `scenarios.ail:6`.
The diagnostic reports expected type `contracts.PriorityOption` and actual type
`contracts.Priority`. Source checking refuses the candidate before architecture
analysis, so this is not a concentration or `AIL.ARCH.HOTSPOT_GROWTH` refusal.
The failed publish created no `.ail` directory and no published revision.

`transcripts/type-refused-check.txt` and
`transcripts/type-refused-publish.txt` contain the recorded command output.

## Existing architecture-refused change

`../job-review-refused/` is the complete source candidate from the refused
change. It is checked in as source, not assembled by a test. Run:

```bash
target/debug/ailc check compiler/examples/job-review-refused
```

The compiler reaches architecture analysis and refuses the candidate with
`AIL.ARCH.HOTSPOT_GROWTH`. It measures control-flow complexity 6 and minimal
context 9 against limits 1 and 6. Commit `74fd64f` records the original agent
edit that moved five validation decisions and approved-job construction into
`transport.ail`; the live candidate makes that commit unnecessary for
reproducing the refusal. The new type-refused candidate does not repeat or
modify this pile-up.

`transcripts/refused-check.txt` and `transcripts/refused-publish.txt` contain
the recorded command output. The failed publish left the existing frozen
revision unchanged.

## Published change

The publishable source remains in this directory. The completed rename declares
`domain.review_job` and uses that name from `transport.dispatch` and
`scenarios.review_fixture`. `ailc check compiler/examples/job-review` prints
`ok`. `ailc publish compiler/examples/job-review` wrote source-set digest
`sha256:d04ad0c8928eab29b6d8e5e069d86ea702ebe928031100ab5e500ab3b92cfb88`.
The `.ail` files under `.ail/revisions/published/sources/` are byte-for-byte
copies of the live source accepted by publish.

`transcripts/passing-check.txt` and `transcripts/passing-publish.txt` contain
the command output. `.ail/revisions/published/revision.json` records each frozen
file digest.

## Published-bytes run

`ail-run` executes the frozen bytes under `.ail/revisions/published/sources/`,
not the `.ail` files in this directory:

```bash
target/debug/ail-run compiler/examples/job-review scenarios.review_fixture \
  --bytes 6a6f622d7061796c6f6164
```

It reports revision `published`, source-set digest
`sha256:d04ad0c8928eab29b6d8e5e069d86ea702ebe928031100ab5e500ab3b92cfb88`, and
`contracts.ReviewDecision::Approved` with `job_id: "fixture-job"` and
`priority: contracts.Priority::Normal`. `transcripts/published-run.txt` holds
that output.

A copy of this directory whose live `scenarios.ail` returns `live-edit-job`
still runs the published `fixture-job`, and `ailc check` on that copy prints
`ok` and `behavior: not-run 0/6` with no program value.
`transcripts/live-edit-run.txt` and `transcripts/live-edit-check.txt` hold that
output. `transcripts/unpublished-run.txt` holds the
`AIL.RUN.NO_PUBLISHED_REVISION` refusal for a copy with no store.

This run executes one entry point of one published example with no
capabilities. See
[published-bytes-runner.md](../../../docs/published-bytes-runner.md).

This record covers these candidates, this project policy, this compiler path,
and this one published run. It does not prove the language.
