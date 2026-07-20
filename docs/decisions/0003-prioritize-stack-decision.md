# ADR 0003: Prioritize the compiler-stack decision

- Status: Accepted; stack-comparison sequence superseded by
  [ADR 0004](0004-rust-compiler-stack.md)
- Date: 2026-07-19
- Owners: project maintainers
- Documentation layer and scope: roadmap sequencing and benchmark governance

## Context

AIL has spent M0 through M7 defining and freezing one reference workload,
four mainstream-language implementations, a shared correctness oracle, hidden
regressions, and answer-free agent task starts. M8a through M8f then added more
than 15,000 lines of calibration schemas, runners, correctness replay,
performance adapters, provider recording, and readiness evidence.

The remaining M8 campaign would require at least 80 successful agent trials,
additional failed or timed-out attempts, 120 warm measurements, 120 cold
measurements, an audit, and a report before the project could write the first
small language contract. That work measures the eventual AIL product claim,
but it does not answer the immediate compiler-stack question.

The attempted M8g launch also showed that the M8f freeze was not actually
ready: the required task-start command fails for TypeScript UC-001 because its
public test output omits `TODO(UC-001)`. Starting official evidence would
therefore require reopening benchmark infrastructure before any language work.

The stack decision needs a much smaller evidence boundary:

1. one written five-construct semantic subset;
2. canonical fixtures and structured diagnostics for that subset;
3. a minimal revision, handle, rename, and identity-map protocol; and
4. comparable Rust and TypeScript implementations of exactly that contract.

Those are the prerequisites already stated by
[ADR 0001](0001-implementation-stack.md) and
[the stack evaluation](../stack-evaluation.md). The full baseline calibration,
numeric AIL success targets, and illustrative syntax variants are not
dependencies of the stack spikes.

## Decision

Stop M8 after the completed M8a–M8f preparation work. Preserve its code,
contracts, pilots, and failed readiness finding as non-official evidence.
Defer M8g through M8o; do not collect or represent any official M8 campaign
result.

Defer M9 numeric success targets until an executable AIL comparator exists and
the maintainers explicitly resume the statistical campaign. The baseline
configuration must be frozen before comparative AIL runs begin, but it does not
block the compiler-stack decision.

Remove M10 illustrative syntax variants from the critical path. Syntax choices
must instead be made as numbered rules and canonical fixtures in the shared
stack-spike contract.

Activate M11 with only the contract required to compare compiler stacks:

- five language constructs;
- grammar and canonical formatting for those constructs;
- the minimum type and capability checks needed by their fixtures;
- structured diagnostics;
- revision-scoped handles;
- a validated rename operation and identity map; and
- a dependency-free contract checker.

The original sequence called for Rust and TypeScript implementations followed
by a stack decision. [ADR 0004](0004-rust-compiler-stack.md) supersedes that
part of this decision: Rust is selected directly and the broader compiler work
begins as production Rust milestones.

## Consequences

- The project starts normative language work now.
- The compiler-stack conversation is driven by working, comparable compiler
  code rather than benchmark operations or illustrative syntax.
- M8a–M8f remain available if the empirical campaign is resumed.
- NFR-002 through NFR-005 remain product-validation requirements, but their
  statistical sample counts no longer gate M11 through M17.
- No production source tree or root package manager was added before the stack
  decision. ADR 0004 now authorizes the root Rust workspace.
- The failed TypeScript task-start gate is recorded rather than repaired now,
  because official calibration is deferred.

## Alternatives considered

### Finish M8g through M8o first

This would spend the next several milestones operating an experiment that
cannot compare against AIL yet. It also requires repairing a supposedly frozen
input before collection can start.

### Select Rust immediately

Rust is the leading durable-compiler candidate, but source preservation,
incremental identity, graph-heavy semantic data, and structured rewriting are
material implementation risks. A bounded TypeScript comparison is cheap enough
to test those risks directly.

### Prototype before writing the shared contract

That would let candidate implementations choose different semantics and make
the comparison meaningless. The M11 contract remains a hard gate.

## Validation

This decision is implemented when:

1. the roadmap marks M8, M9, and M10 deferred;
2. M11 is active and depends only on the accepted reference slice and M7;
3. current status gives a concrete M11 handoff;
4. deferred M8 launch directives cannot be mistaken for active authority; and
5. `python3 tools/check_docs.py` passes.
