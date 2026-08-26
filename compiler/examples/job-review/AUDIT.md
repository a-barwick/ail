# Job review publish record

## What changed

A Cursor agent edited the AIL files in this directory. No Rust harness wrote
the candidate. The passing change adds `requested_by` to the review request and
approved job, validates it in `validation.ail`, carries it through
`domain.ail`, and supplies it in `scenarios.ail`.

The final program is 241 lines across six AIL files. It uses existing modules,
records, closed variants, `if`, `match`, and ordinary calls. This change adds
no language feature, capability, or permission file.

## Project rule

`architecture.json` maps every module and intrinsic endpoint into the five
architecture groups required by the existing checker. Project policy holds
`transport.dispatch` to control-flow complexity 1 and minimal context 6. Those
are existing compiler metrics, not new language rules.

## Refused change

In commit `74fd64f`, the agent moved five validation decisions and approved-job
construction into the live `transport.ail` source. The compiler reached
architecture analysis, then `ailc check` and `ailc publish` both refused that
candidate with `AIL.ARCH.HOTSPOT_GROWTH`. It measured control-flow complexity 6
and minimal context 9 against limits 1 and 6. The failed publish left the
existing frozen revision unchanged.

`transcripts/refused-check.txt` and `transcripts/refused-publish.txt` contain
the command output.

## Published change

The passing edit keeps `transport.dispatch` as one call to `domain.review` and
puts requester validation in the domain path. `ailc check` printed `ok`.
`ailc publish` wrote source-set digest
`sha256:121702fd6b29934ec98913a7aa98b86add1bc6a1360d05376ff613f957d85775`.
The `.ail` files under `.ail/revisions/published/sources/` are byte-for-byte
copies of the live source accepted by publish.

`transcripts/passing-check.txt` and `transcripts/passing-publish.txt` contain
the command output. `.ail/revisions/published/revision.json` records each frozen
file digest.

This record covers these candidates, this project policy, and this compiler
path. It does not prove the language or execute application behavior.
