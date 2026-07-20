# Job-service benchmark

Status: **Accepted benchmark policy**

This benchmark compares the same two engineering tasks in Rust, Go, Python,
TypeScript, and eventually AIL:

1. implement the accepted create-job behavior; and
2. add priority without missing any affected code or breaking version-one
   behavior.

The benchmark tests whether AIL helps an agent finish a correct change with less
searching and rework. It is not a contest to produce the shortest source file.

The fixture, baseline, and parity foundation was delivered in M1 through M7.
M8a through M8f added deferred calibration infrastructure. ADR 0003 moves the
active project work to the compiler-stack contract; the statistical campaign
resumes before comparative AIL benchmark runs, not before compiler
implementation. The active handoff is in [the status file](../STATUS.md).

## Decisions

- Shared test cases use the [job-service JSON format](job-service-fixtures.md).
- The compiler's complete-results promise covers everything in the current
  build that it can inspect.
- Impact reports separate code that definitely needs an edit from code that may
  need review.
- The four baseline languages are measured before numeric AIL targets are set.
- Correctness is required from the first run. Performance and agent-efficiency
  targets are set after baseline calibration and before comparative AIL runs.

## What “everything in the current build” means

The compiler must inspect:

- application source;
- tests and test helpers;
- checked-in test data and schemas;
- generated code used by the build;
- the editable input or schema that produced generated code; and
- locked dependency source available in the build environment.

The compiler cannot promise that it found consumers in another repository, a
deployed client, or a dependency whose source is unavailable. It must list those
as unchecked rather than silently ignoring them or claiming complete coverage.

## Impact report

The priority-change task produces three lists:

- `must_change`: every location that definitely requires an edit;
- `review`: locations that may need attention, with a reason; and
- `unchecked`: known external consumers or unavailable source.

For the first workload, `must_change` must be exact. It may not miss a required
edit or include an unnecessary one. `review` may contain at most two entries
that turn out not to need a change.

This split gives the agent a reliable work list while allowing the compiler to
show honest uncertainty separately.

## Calibration sequence

1. Freeze the public JSON cases, language-independent hidden behaviors, and
   hidden seed categories.
2. Build equivalent Rust, Go, Python, and TypeScript implementations while
   instantiating those seed categories without changing their behavior.
3. Verify cross-language parity and lock each source tree, public and hidden
   test artifact, task, tool configuration, and run manifest by digest.
4. Run at least 10 successful agent trials per task and language.
5. Measure context, repair cycles, correctness, elapsed agent time, handler
   latency, throughput, startup, and memory.
6. Review the baseline results and record the AIL success targets.
7. Lock those targets before running an AIL implementation.

The targets may not be adjusted after an AIL result is known.

## Correctness gates

A run counts as successful only when:

- all public and hidden tests pass;
- the response, final stored data, and storage calls match the shared cases;
- invalid input makes no storage call;
- the priority change misses no seeded location;
- version-one compatibility still works;
- no new external access is introduced; and
- every completion artifact refers to the final source revision.

A fast or low-context run that fails any correctness gate does not count as a
successful result.

## Pilot safety limits

These limits catch runaway tools or broken runners. They are not production AIL
targets:

- 100,000 model input tokens per run;
- 30 seconds for one functional corpus run;
- 2 seconds for runner startup; and
- 512 MiB peak resident memory.

The baseline results will be used to set the actual AIL comparison targets.

## Locked run manifest

Before a run starts, record:

- task text and starting commit;
- dependency lock and source revision;
- visible and hidden test revisions;
- model, agent, and sampling settings;
- initially supplied context;
- available tools and exact versions;
- container image by digest;
- reference host details;
- network and filesystem permissions;
- time limit and retry policy;
- token-accounting method;
- definition of a repair cycle;
- correctness checks; and
- raw result and measurement output locations.

Changing any locked field creates a new benchmark configuration. Results from
different configurations must be reported separately.

## Project boundary

This document defines benchmark policy. It does not define AIL syntax or program
behavior, choose the compiler implementation stack, or authorize a production
source tree.
