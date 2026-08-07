# ADR 0007: Ship language composition instead of UC-008 setup

- Status: Amended and complete
- Date: 2026-08-07
- Owners: project maintainers
- Documentation layer and scope: M28 compiler direction

## Context

The original decision selected UC-008 iterative service evolution and proposed
M28–M36 work to define a mature job service, build four baseline trajectories,
fill AIL language gaps, and run a cumulative comparison.

On 2026-08-07 the maintainer replaced that M28 plan with a concrete compiler
need: ordinary AIL composition. The compiler could not call AIL functions or
split a program into explicit modules. Those missing basics blocked larger
programs immediately; more benchmark setup did not change executable capability.

## Decision

M28 implements:

- local and imported AIL function calls;
- exact argument-count and type checks;
- explicit module and import headers;
- import-controlled visibility and ambiguity diagnostics;
- transitive capability-effect checking;
- rejection of recursive calls and import cycles; and
- deterministic left-to-right argument evaluation and nested interpretation.

The old M29–M36 UC-008 sequence is inactive. UC-008 remains a proposed use case,
not the current build plan.

## Consequences

- AIL programs can now compose checked functions across files.
- The existing revision, impact, architecture, formatting, and interpreter
  behavior continues to apply to ordered source sets.
- The compiler still lacks iteration, collections, concurrency, networking,
  packages, native execution, and deployment.
- Any future cumulative comparison must define the workload and expected results
  from the current compiler state rather than treating the old sequence as an
  instruction.

## Alternatives considered

### Build the UC-008 benchmark setup first

Rejected for M28. It would add fixtures and measurement machinery while leaving
AIL unable to express ordinary cross-function, multi-file programs.

### Add a broad conventional language core

Rejected. Calls and modules were the smallest coherent composition capability.
Collections, iteration, and concurrency require separate semantics and tests.

### Start native execution

Rejected. The deterministic interpreter can test the current semantic work, and
no production runtime contract exists.

## Validation

`cargo test --workspace --test m28_composition` must prove local and imported
calls, exact arguments, transitive effects, deterministic nested execution,
module visibility, all import failures, recursion rejection, and import-cycle
rejection. The complete workspace test suite and 37 public job-service cases
must continue to pass.
