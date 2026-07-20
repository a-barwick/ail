# Current status

Last updated: 2026-07-19

## Active milestone

M11 — Compiler-stack spike contract

## Current goal

Write the smallest normative contract that lets Rust and TypeScript implement
and compare the same compiler work. The project is now on the direct path to
the implementation-stack decision:

```text
M11 five-construct contract
  -> M12 Rust and TypeScript spikes
  -> M13 stack decision
  -> semantic-oracle implementation
```

The next agent should:

- read [ADR 0003](decisions/0003-prioritize-stack-decision.md);
- select exactly five constructs that exercise lossless parsing, canonical
  formatting, local inference, one capability error, and symbol rename;
- write numbered proposed language and protocol rules for only that subset;
- add canonical positive, formatting, recovery, type, capability, rename, and
  stale-revision fixtures;
- define the transport-independent revision, handle, diagnostic, rename, and
  identity-map shapes;
- implement `python3 specs/tools/core_contract.py check`; and
- stop before implementing either candidate spike.

## Why the sequence changed

M0 through M7 already established the accepted workload, behavior oracle, four
strong language baselines, private regressions, and answer-free task starts.
M8a through M8f added reusable agent-runner, evidence, correctness, replay, and
performance infrastructure, but no official campaign result.

The remaining M8 plan required at least 80 successful agent trials plus 240
performance measurements before any AIL language contract could be written.
That evidence is useful for validating AIL later, but it does not determine
whether Rust or TypeScript is the better compiler implementation stack.

The first M8g launch also found that the M8f freeze is not launchable:

```text
ERROR [task_start_check_failed]:
typescript/UC-001/public-task-tests: output omitted 'TODO(UC-001)'
```

No official trial started. Per ADR 0003, M8g through M8o, M9 numeric targets,
and M10 illustrative syntax variants are deferred. Do not repair the
calibration campaign while executing M11 through M13.

## Accepted foundation

- UC-001 request validation and conditional persistence
- UC-003 public and persisted schema evolution
- Accepted `APP-*`, `LANG-*`, `PROTO-*`, and benchmark requirements
- 37 public behavior fixtures and five separately held private behaviors
- Rust, Go, Python, and TypeScript V1/V2 reference implementations
- Cross-language normalized parity
- Eight digest-locked answer-free task starts
- Stack-evaluation weights and Rust/TypeScript candidate set
- M8a–M8f calibration infrastructure and non-official pilot evidence

## Completed

- M0 — Reference workload and requirements
- M1 — Frozen job-service fixture corpus
- M2 — Benchmark harness and frozen task contract
- M3 — Rust baseline
- M4 — Go baseline
- M5 — Python baseline
- M6 — TypeScript baseline
- M7 — Cross-baseline parity and freeze
- M8a–M8f — Calibration preparation and readiness infrastructure

## Deferred

- M8g–M8o — Official baseline calibration campaign
- M9 — Numeric AIL benchmark targets
- M10 — Illustrative AIL variants
- UC-007 — Architectural regression control

These items require an explicit maintainer decision to resume. They do not
block M11, M12, M13, or the first semantic-oracle implementation milestone.

## Do not start yet

- Rust or TypeScript spike implementation before the M11 contract passes
- A production source tree or root package manager before M13 selects the stack
- The broader 20–30 construct language core
- Native code generation, production runtime work, or general concurrency
- Official agent or performance evidence

## Blockers

None. M11 can proceed from the accepted requirements and M7 fixtures without
the deferred calibration campaign.

## Handoff checklist

After meaningful work:

- keep the M11 subset to exactly five constructs;
- distinguish proposed rules from illustrative examples;
- make every fixture deterministic and machine-checkable;
- keep candidate-specific choices out of the shared contract;
- add executable checks for every delivered contract rule;
- run `python3 specs/tools/core_contract.py check`;
- run `python3 tools/check_docs.py`; and
- update this file and the roadmap only when the M11 exit criterion passes.
