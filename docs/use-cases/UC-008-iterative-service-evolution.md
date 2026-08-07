# UC-008 — Iterative service evolution

Status: **Proposed and inactive**

UC-008 asks whether a sequence of fresh agents can maintain one service through
repeated product changes using only the repository and compiler, without relying
on conversation memory.

## System under test

The proposed system is a mature transport-independent job service with multiple
operations, persistent state, validation and compatibility rules, closed
outcomes, explicit external capabilities, transport adapters, domain handlers,
tests, and architecture policy.

The old estimate of roughly 2,000 lines across five to eight files described the
maintenance pressure sought in mainstream implementations. It is not a quota.
Padding or artificial file splitting would invalidate the workload.

## Change sequence

A useful sequence could include cancellation audit facts, retries, supplied
time, tenant policy, bounded group cancellation, status notifications, schema
compatibility, and an administrative override. No sequence has been selected.

Each selected change would need exact behavior, state transitions, ordered
effects, compatibility rules, architecture constraints, public and held-back
tests, and a known-good predecessor revision.

## Fresh-context rule

A measured run starts from one initial revision. Each later agent receives the
actual revision produced by the previous agent in that run, not a known-good
reference revision or the previous conversation. A failed or timed-out change
ends that run. Replacement runs get new identities and do not erase failures.

This exposes path dependence: an early design choice can make every later change
easier or harder.

## Facts to record per change

- behavior, prior-regression, schema, state, and ordered capability results;
- analyzed and unchecked coverage;
- source reads, compiler queries, edits, validations, and repair cycles;
- context delivered to the model and elapsed work;
- dependency, capability, effect, state, hotspot, and review-context changes;
- textual and semantic diffs; and
- the exact facts a human used to approve or reject the revision.

Rust, Go, Python, and TypeScript must use their normal compiler, language server,
formatter, linter, tests, refactoring, and static-analysis tools. Implementations
may use idiomatic source layouts while preserving equivalent behavior and
authority constraints.

## Current blocker

The workload, ordered changes, baseline tools, and expected results are not
defined. AIL also lacks iteration, collections, several capabilities, and
production runtime behavior that such a service may require.

M28 independently shipped calls and modules. That does not activate the rest of
UC-008 or select collections, concurrency, native execution, or benchmark work.
No requirement has been accepted from UC-008.
