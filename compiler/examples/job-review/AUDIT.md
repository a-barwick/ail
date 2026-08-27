# Job review refusal and publish records

## What changed

A Cursor agent edited the AIL files in this directory. No Rust harness wrote
the candidate. This change removes the missing-priority rejection. A request
with `PriorityOption::None` now reaches `approve`, where the existing
`selected_priority` function returns `Priority::Normal`. The scenario supplies
`None` instead of `High`. The earlier `requested_by` fields and validation
remain unchanged.

The final program is 222 lines across six AIL files. It uses existing modules,
records, closed variants, `if`, `match`, and ordinary calls. This change adds
no language feature, capability, or permission file.

## Project rule

`architecture.json` maps every module and intrinsic endpoint into the five
architecture groups required by the existing checker. Project policy holds
`transport.dispatch` to control-flow complexity 1 and minimal context 6. Those
are existing compiler metrics, not new language rules.

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

The publishable source remains in this directory. Its passing edit keeps
`transport.dispatch` as one call to `domain.review`. It removes
`ReviewReason::MissingPriority`, `validate_priority`, and the corresponding
domain rejection branch. `ailc check compiler/examples/job-review` prints
`ok`. `ailc publish compiler/examples/job-review` wrote source-set digest
`sha256:35402fbaaf456cf3846516e00b9703adf33ef0eb8c37b1c6091ef27b5ab551e9`.
The `.ail` files under `.ail/revisions/published/sources/` are byte-for-byte
copies of the live source accepted by publish.

`transcripts/passing-check.txt` and `transcripts/passing-publish.txt` contain
the command output. `.ail/revisions/published/revision.json` records each frozen
file digest.

This record covers these candidates, this project policy, and this compiler
path. It does not prove the language or execute application behavior.
