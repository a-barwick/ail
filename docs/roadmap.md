# Roadmap

This roadmap orders uncertainty before implementation volume. Dates and release
labels should be added only after the core semantics and stack have been chosen.

## Phase 0: executable language core

Deliverables:

- glossary and explicit observable-determinism contract;
- 20–30 core constructs;
- lexical grammar, concrete grammar, and canonical formatting rules;
- static semantics for names, types, effects, capabilities, and typed holes;
- dynamic semantics for evaluation, faults, and errors;
- module and visibility rules;
- ten to twenty canonical example programs; and
- conformance fixtures with expected structured diagnostics.

Exit criterion: a small program's canonical source, result, effects, and primary
diagnostics can be predicted without implementation-specific assumptions.

## Phase 1: stack selection

Run the common spikes in [stack-evaluation.md](stack-evaluation.md), record
results, and accept an architecture decision record.

Exit criterion: the team commits to one authoritative implementation language
through the semantic-oracle milestone.

## Phase 2: semantic oracle

Build:

- parser with recovery and source preservation;
- canonical formatter;
- name resolver;
- type and minimal effect checker;
- typed-hole support;
- structured diagnostic engine;
- deterministic interpreter; and
- snapshot/conformance test harness.

Exit criterion: all core fixtures parse, format idempotently, type-check or
produce their expected primary diagnostic, and execute deterministically.

## Phase 3: agent protocol

Build:

- versioned protocol schema;
- workspace revisions and revision-scoped handles;
- semantic queries;
- elaborated views;
- validated rename as the first structural transaction;
- identity maps; and
- bounded context slices using semantic graph edges.

Exit criterion: a client can inspect, diagnose, rename, and revalidate a program
without coordinating raw multi-file text replacements.

## Phase 4: safety and controlled execution

Extend the core with:

- finalized ownership or managed-memory model;
- structured concurrency;
- cancellation and resource-limit semantics;
- bounded and explicitly unbounded constructs;
- recordable nondeterministic events; and
- replay log versioning and redaction policy.

Exit criterion: concurrency, resource faults, and replay outcomes are covered by
portable conformance tests.

## Phase 5: production lowering

Select one authoritative bytecode or native backend. Define artifact
reproducibility, target support, debugging metadata, foreign primitives, and
runtime packaging.

Exit criterion: representative programs produce reproducible supported-target
artifacts that agree with the semantic oracle.

## Phase 6: ecosystem features

Consider only after the core and backend are stable:

- compatible target-source emission;
- third-party lowering packages;
- advanced refactoring transactions;
- semantic review reports;
- package registry and supply-chain policy; and
- empirical agent-ergonomics benchmark suite.
