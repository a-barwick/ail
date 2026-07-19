# UC-003 priority-evolution task

Task contract version: 1

## Task text

Starting from the supplied accepted version-one job-service revision, add the
required closed priority field and version-two compatibility behavior.

Before editing, identify every affected producer, consumer, handler, job
constructor, store contract and implementation, persisted codec, version
adapter, response projection, test, fixture, and completion artifact that is
present in the workspace. No curated file or location list is supplied.

Version-two producers must supply exactly low, normal, or high explicitly.
Propagate the selected value unchanged through the handler, version-two stored
record, and version-two created response. Adapt version-one requests and stored
records explicitly to normal. A version-one request must receive a version-one
response without priority; a version-two request must receive a version-two
response. Missing version-two priority is an ordered invalid-request issue.
Unknown priority is a boundary decode failure. Both make no store call.

Preserve every UC-001 validation, result, final-state, authority, store-call
count, and store-call ordering guarantee. Do not change fixture or hidden
oracles, the compatibility window, task limits, tool configuration, or
benchmark files. Do not add new external authority, effects, transport,
production infrastructure, concurrency, or deployment behavior.

Use the normal compiler, static-analysis, language-server, formatter, search,
build, and test tools available in the locked run manifest. The task is complete
only when every public and hidden check passes, no seeded affected role is
missed, and completion evidence identifies the final source revision and
accounts for the complete change.
