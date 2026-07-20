# Current status

Last updated: 2026-07-19

## Active milestone

M8 — Baseline agent calibration (M8e: performance measurement)

## Current goal

Implement equivalent warm-state and cold-process measurement adapters for all
four baselines. Non-official pilots are allowed; no official agent or
performance evidence exists yet.

The next agent should:

- follow M8e in the
  [accepted M8 execution plan](m8-execution-plan.md);
- implement readiness, warm-up, shared-corpus, monotonic timing, latency,
  throughput, percentile, variance, load, and affinity recording;
- implement process creation, cold readiness, idle and peak RSS, package and
  dependency identity, external-access recording, and correctness gating;
- run only the bounded non-official warm and cold pilots allowed by M8e; and
- stop before the M8f readiness freeze or any official evidence.

## Starting point

The following work is accepted and should not be redesigned in M8e:

- [UC-001 request validation and persistence](use-cases/UC-001-request-validation-and-persistence.md)
- [UC-003 public schema evolution](use-cases/UC-003-public-schema-evolution.md)
- [Initial requirements](requirements/reference-slice.md)
- [Benchmark policy](benchmarks/README.md)
- [JSON fixture format](benchmarks/job-service-fixtures.md)
- [Executable benchmark artifact guide](../benchmarks/README.md)
- [Runner contract](../benchmarks/contracts/runner-contract.md)
- [Run and repair classifications](../benchmarks/contracts/run-classification.md)
- [Hidden behavior and seed contract](../benchmarks/contracts/hidden-contract.json)
- [Final UC-001 task](../benchmarks/tasks/uc001-implement-create-job.md)
- [Final UC-003 task](../benchmarks/tasks/uc003-add-priority.md)
- [Answer-free task starts](../benchmarks/task-starts/README.md)
- [M8 execution plan](m8-execution-plan.md)
- [M8 agent experiment contract](decisions/0002-m8-agent-experiment-contract.md)
- [M8 calibration evidence contract](../benchmarks/calibration/README.md)
- [Rust baseline](../benchmarks/baselines/rust/README.md)
- [Go baseline](../benchmarks/baselines/go/README.md)
- [Python baseline](../benchmarks/baselines/python/README.md)
- [TypeScript baseline](../benchmarks/baselines/typescript/README.md)
- [ADR 0001 deferring stack selection](decisions/0001-implementation-stack.md)

Key decisions:

- shared fixtures, task text, runner protocol, normalized results, and failure
  classifications remain frozen;
- each baseline has distinct digest-locked V1 and V2 source checkpoints;
- each language/task pair has an independently digest-locked, answer-free
  workspace with protected task, test, fixture, and tool artifacts;
- normal language tooling remains available without custom semantic advantages;
- hidden cases may combine only already accepted UC-001 and UC-003 behavior;
- every agent trial uses one pinned Codex CLI agent with explicit
  `gpt-5.6-sol` at High reasoning, an interactive local-tool loop, no subagents,
  and no external agent tools;
- the agent wall limit is 600 seconds and the amended cumulative input-token
  safety limit is 500,000 tokens including cached and repeated delivery;
- provider request capture, exact categorical accounting, least-privilege
  filesystem access, network denial, no trial retries, and complete
  public/private final-revision correctness are required before a trial can be
  successful;
- official M8 trial and performance counts still begin at zero; and
- no M8 configuration or evidence becomes official before its reviewed freeze.

## Completed

- M0 — Reference workload and requirements
- M1 — Frozen job-service fixture corpus
- M2 — Benchmark harness and frozen task contract
- M3 — Rust baseline
- M4 — Go baseline
- M5 — Python baseline
- M6 — TypeScript baseline
- M7 — Cross-baseline parity and freeze
- M8a — Frozen agent experiment contract
- M8b — Calibration evidence contracts and verifier
- M8c — Interactive agent runner
- M8d — Correctness verification and replay

M1 delivered 37 canonical public JSON cases, a machine-readable schema, a
dependency-free semantic checker and formatter, negative-path tool tests, and a
SHA-256 manifest with complete UC-001/UC-003 traceability.

M2 delivered the language-neutral one-case and corpus runner protocol, common
result and run-manifest schemas, final task text, run/repair classifications,
hidden behavior and seed rules, a 13-artifact contract lock, and a self-test
that proves 14 pass and failure outcomes.

M3 and M4 delivered the distinct Rust and Go V1/V2 baselines, ordinary language
tools and tests, frozen checkpoints, hidden seed locations, and verified shared
runner results.

M5 delivered a typed CPython 3.13 baseline with `uv`-locked development tools,
strict mypy, Ruff, pytest, branch-aware coverage, 61 focused and integration
tests, frozen V1/V2 checkpoints, and all 37 public fixtures accepted by the
shared harness.

M6 delivered a strict TypeScript 5.8 baseline on Node.js 23, a locked dependency
tree, TypeScript, ESLint, Prettier, the ordinary Node test runner, c8 coverage,
57 focused and integration tests, frozen V1/V2 checkpoints, and all 37 public
fixtures accepted by the shared harness.

M7 provides the candidate-neutral `verify-all` gate, validates both locked
source checkpoints and every frozen seed location for each baseline, verifies
the response, final state, and ordered store calls for public and digest-locked
private cases, and compares all normalized results across languages. It also
publishes a 42-case parity report (37 public and 5 private), eight deterministic
answer-free task starts, and a 39-artifact freeze lock. The exact frozen Rust,
Go, Python, and TypeScript tools and the separately held private ZIP were
available for the closing gate. The ZIP remains outside the repository and is
identified only by its SHA-256 digest in the locked manifests.

M8a accepted ADR 0002. It selects the candidate measured model and agent,
defines one tool-using trial state machine, fixes the prompt and initial
context, requires request-level evidence and token reconciliation, isolates the
workspace and network, defines terminal classifications, and amends NFR-002
from a 100,000-token pilot limit to a 500,000-token interactive trial limit.
The preserved pilot remains non-authoritative and official M8 counts remain
zero.

M8b added eight schemas, a canonical experiment contract, SHA-256 locks, agent
and performance record shapes, raw JSONL event records, evidence indexes, and a
fact-only report contract. `verify-calibration` checks exact task starts and
rendered prompts, complete categorical token accounting, event continuity,
artifact hashes, unique identities, exclusions, campaign counts, summaries,
and configuration consistency. Its empty, pilot, partial, malformed, complete,
and nine targeted rejection fixtures produce 14 stable outcomes. The M2 and M7
locks were re-sealed only for the added harness command; frozen tasks, fixtures,
oracles, task starts, and baseline source trees did not change.

M8b used the exact Responses input-token count endpoint with ordered
cumulative-prefix deltas for category attribution and zero reconciliation
tolerance. M8f must still prove that rule with the selected agent before the
campaign freeze.

M8c added a task-start-derived interactive runner with a complete pre-start
observation gate, isolated Codex and loopback-provider configuration,
least-privilege filesystem and network policy, raw event capture, exact token
and wall limits, process-group termination, deterministic final-source
retention, and frozen M2 activity accounting. Fake and dry streams prove stable
success, failure, timeout, permission, token-limit, and incomplete-evidence
outcomes. It invoked no model and recorded no official evidence.

The M8c correction permits UC-003 agents to create implementation files only
inside the selected language's V2 source roots. UC-001 remains limited to its
listed editable files, and tests, fixtures, task text, tool configuration,
parent paths, private inputs, and evidence remain protected.

M8d added canonical retained-source validation, a verifier-only private-package
boundary, exact task-applicable public/private/seed coverage, protected-file and
permission enforcement, completion evidence bound to the final revision, and a
second fresh functional replay. Eight fake/dry outcomes reject incomplete
source, answer exposure, protected changes, permission violations, stale
revisions, seeded regressions, and replay divergence. The campaign verifier
also rejects stale completion artifacts. No model ran and no official evidence
was recorded.

## After M8e

- M8f — Run readiness pilots and freeze the campaign
- M9 — Frozen AIL success targets

## Proposed future validation

[UC-007 architectural regression control](use-cases/UC-007-architectural-regression-control.md),
its [proposed requirements](requirements/architectural-health.md), and the
[architectural health manifest](architecture-health.md) define a later scaling
gate. They do not expand M8e or authorize implementation before review and
acceptance.

## Do not start yet

- AIL syntax design
- AIL compiler implementation
- Compiler-stack prototypes
- Agent benchmark runs
- Official performance measurements
- M8 evidence collection
- Production performance targets

Those depend on later M8 submilestones or later roadmap milestones.

## Blockers

None recorded for M8e. Performance adapters must preserve M8d correctness
gating and the frozen M8a treatment while producing evidence that satisfies the
M8b contracts.

## Handoff checklist

After meaningful work:

- update this file with what changed and what remains;
- preserve every frozen fixture, task, protocol, and oracle;
- add or update executable checks;
- record behavioral differences instead of silently weakening the oracle;
- run the active milestone's verification commands;
- run `python3 tools/check_docs.py`; and
- update the roadmap only when the milestone exit criterion passes.

For M8e, keep official counts at zero, retain every non-official pilot
classification, and leave a concise handoff for M8f.
