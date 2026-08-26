# Job review publish record

## Program

The accepted M19 `r2` job-service workspace is the largest canonical AIL source
set that was already on `main`: 181 lines across five files. It predates module
imports and requires `JobsStore`, so the current CLI cannot check it with its
empty capability environment.

This example keeps the job-request validation work but stops before storage. Its
210 lines use current modules, records, closed variants, `if`, `match`, and
ordinary calls. It has no capability or permission file.

## Project rule

`architecture.json` maps every module and intrinsic endpoint into the five
architecture groups required by the existing checker. Project policy holds
`transport.dispatch` to control-flow complexity 1 and minimal context 6. Those
are existing compiler metrics, not new language rules.

## Refused change

The test moves the five validation decisions and job construction into
`transport.dispatch`. With architecture policy withheld, `ailc check` accepts
the same source as a typed AIL program. With project policy restored,
`ailc check` and `ailc publish` report `AIL.ARCH.HOTSPOT_GROWTH`. The compiler
measures control-flow complexity 6 and minimal context 9 against limits 1 and 6.
Publish writes no revision.

## Published change

The checked source keeps `transport.dispatch` as one call to `domain.review`.
`ailc check` prints `ok` and writes nothing. `ailc publish` runs the same checks
and writes revision
`sha256:ac683406f6ae5f3f36d3d0fb36a8d0f6c5579910dc8b9b23ddf13547c5a8b6a4`
under `.ail/revisions/published`. The test compares every checked-in frozen
`.ail` file with live source, then republishes a temporary copy and compares
every frozen file byte for byte with the source read after the successful check.

This record proves only this compiler path, project policy, candidate, and
published source set. It does not prove the language or execute application
behavior.
