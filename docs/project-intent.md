# Why AIL exists

AIL is an executable programming language designed for software agents as its
primary authors and operators. Humans remain responsible for review, operations,
incident analysis, and accountability, so human auditability is a hard system
requirement.

## Optimize the complete change

Agents can generate plausible code quickly. The expensive work is finding the
right context, tracing consequences, validating behavior, diagnosing failures,
repairing the change, and preventing regressions.

AIL optimizes this complete loop:

```text
generation
  + context discovery
  + consequence analysis
  + validation
  + diagnosis
  + repair
  + regression risk
```

Short source is useful only when it reduces that total. Compact syntax that
forces an agent to load more files or infer hidden authority is a loss.

## Why language semantics matter

Tools can improve navigation and refactoring in existing languages. AIL is worth
building only where language rules create guarantees that compatibility tools
cannot recover reliably:

- one canonical representation for each construct;
- explicit public types, closed errors, effects, and capabilities;
- deterministic observable execution;
- no ambient authority;
- analyzable dependencies and complete coverage reporting;
- stable structured diagnostics; and
- revision-safe, atomic structural changes.

The compiler protocol reports facts created by these rules. It cannot compensate
for ambiguous semantics or hidden runtime behavior.

## Human auditability

AIL is not an opaque agent bytecode. A reviewer must be able to inspect:

- canonical source and public contracts;
- inferred types and transitive effects;
- capability authority and ordered external operations;
- changed semantic identities and downstream consumers;
- structured diagnostics and failed validation; and
- the textual and semantic delta for the exact revision under review.

The compiler may infer local details, but it must expose the elaborated result.
An approval must not depend on undocumented model reasoning.

## Technical consequences

- Canonical formatting removes representational aliases and diff noise.
- Explicit boundaries reduce the source needed to use a module safely.
- Capabilities make external authority visible and testable.
- Fixed evaluation and effect order make failures reproducible.
- Revision-scoped handles prevent stale inspection and edits.
- Impact queries enumerate known consequences and identify unchecked boundaries.
- Atomic validation publishes either a complete valid revision or nothing.
- Architecture reports expose primitive facts and contributors instead of one
  opaque score.

## The test that matters

AIL should require less total context and fewer repair iterations, and produce
fewer regressions, than the same agent using a mainstream language with its
normal compiler and language server on the same representative change.

That comparison must preserve correctness and human review quality. It must use
strong Rust, Go, Python, and TypeScript tools, not raw text editing. The project
has built baseline services and measurement infrastructure but has not completed
this comparative test.

More syntax, a native backend, self-hosting, compiler size, or one successful
agent run does not answer the question. Those can be useful engineering outputs,
but the result is measured change cost on correct, reviewable work.

## Boundaries

AIL is not a prompt format, an agent communication protocol, a compressed model
encoding, or a replacement for human review. It does not assume memory,
concurrency, distributed systems, or resource control become easy because an
agent writes the code. It must define those semantics before claiming to support
them.

The next document, [application-vision.md](application-vision.md), turns this
goal into concrete software and change workloads.
