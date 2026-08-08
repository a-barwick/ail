# AIL technical investor brief

AIL is an executable language and Rust compiler for software written and
operated primarily by agents. The product goal is to reduce the total work of a
correct change: context discovery, consequence analysis, validation, repair,
regression control, and human review.

## The system today

The compiler ships:

- lossless parsing and canonical formatting;
- name, type, capability-effect, module, import, and function-call checking;
- immutable source revisions and revision-scoped semantic handles;
- deterministic interpretation with caller-supplied capabilities;
- exact schema-impact queries and semantic diffs;
- atomic multi-file candidate validation and rollback;
- architecture snapshots, deltas, policy enforcement, and bounded failure; and
- explicit modules, import aliases, qualified references, and cross-module
  calls;
- immutable bounded lists and deterministic sequential map; and
- complete external list validation before capability checks or calls.

M28 added calls and language composition. Arguments are checked exactly and
evaluated left to right. Effects propagate through the call graph. The compiler
rejects direct and mutual recursion, import cycles, inaccessible declarations,
and ambiguous bare imports.

The three-file example in `compiler/examples/composed-service/` runs through the
real checker and interpreter. The AIL job-service implementation passes all 37
public behavior cases.

M29 added a three-module bounded cancellation service. It accepts zero to 32
job positions, preserves order and duplicates, returns one closed outcome per
position, and rejects oversized or malformed input before effects.

## Two concrete change loops

### Schema evolution

For the locked priority change, the compiler reports 12 exact `must_change`
locations, two reasoned `review` locations, and one unavailable external
consumer. The valid transaction applies five ordered path edits, preserves
capability authority and effect order, and publishes one immutable child
revision. Stale, incomplete, incompatible, effect-changing, and behaviorally
invalid candidates publish nothing.

### Architecture enforcement

Three `CancelJob` candidates pass the same six behavior cases. The domain-owned
version publishes. The centralized version is rejected because it grows dispatch
and moves dependencies, jobs-store authority, and jobs state into transport. A
helper-split version is also rejected because aggregate transport responsibility
is unchanged.

The compiler reports primitive facts and exact contributors. Project policy
chooses thresholds. AIL does not infer one universal architecture score.

## Why this matters

Coding agents can produce source quickly, but they still spend substantial work
finding relevant context, tracing consequences, repairing failed changes, and
giving reviewers confidence that nothing was missed. AIL makes more of that work
a deterministic compiler operation.

Canonical source remains the durable program. Humans can inspect the source,
public contracts, effects, authority, semantic delta, diagnostics, and exact
revision without trusting hidden model reasoning.

## Hard limits

AIL cannot yet build a production service. Sequential map over immutable bounded
lists is its only repeated-work form. It has no general loops, collection
library, mutation, concurrency, networking, package registry, foreign-code
boundary, production runtime, native backend, or deployment system. Recursion
is rejected. The architecture API implements the M24 metric and policy set, not
the larger design catalog.

No AIL-versus-Rust/Go/Python/TypeScript agent comparison has run. The current
fixtures prove compiler behavior on specified cases; they do not prove lower
engineering cost across representative projects.

## Next engineering result

Select one real service behavior blocked by the current language. Define its
canonical source, static checks, runtime results, failure diagnostics, and
compiler-interface output. Implement the smallest missing semantics and run the
existing suite unchanged.

The first external comparison should use fresh held-back changes and give Rust,
Go, Python, and TypeScript their normal compilers, language servers, formatters,
tests, refactoring tools, and static analyzers. Measure completed-task rate,
context, source reads, semantic queries, validation attempts, repair cycles,
regressions, elapsed work, and reviewer defects. Record failures and timeouts;
do not report only successful runs.

## Current status

M29 is complete. No successor is active. The next investment in language breadth,
runtime, or measurement should produce a named executable capability, not more
planning artifacts.
