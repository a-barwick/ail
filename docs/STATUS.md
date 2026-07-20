# Current status

Last updated: 2026-07-19

## Active milestone

M12 — Comparable compiler-stack spikes

## Current goal

Implement the exact M11 five-construct contract in Rust and TypeScript, then
compare the two compiler stacks using the weights frozen before implementation.
The execution path is:

```text
M11 five-construct contract (complete)
  -> M12 Rust and TypeScript spikes (active)
  -> M13 stack decision
  -> semantic-oracle implementation
```

The next agent should:

- read the [M11 contract](../specs/README.md) and
  [stack evaluation](stack-evaluation.md);
- create disposable candidates under `prototypes/rust/` and
  `prototypes/typescript/` without adding a production source tree or root
  package manager;
- implement the same lossless parser, canonical formatter, bounded checker,
  structured diagnostics, revision-scoped handles, inspection, validated
  rename, and identity-map behavior in both candidates;
- add one candidate-neutral `python3 prototypes/check.py` command that runs the
  shared fixtures against both implementations;
- record implementation time, source size, test time, memory, recovery quality,
  handle maintenance, packaging, and contributor friction; and
- stop before choosing the authoritative compiler stack.

## M11 result

M11 delivered exactly five constructs:

1. records;
2. closed variants;
3. functions with explicit public signatures;
4. local `let` inference; and
5. capability operation calls.

The contract contains 24 numbered proposed language and protocol rules, seven
canonical fixture categories, nine transport-independent protocol shapes, and
a dependency-free checker. The checker verifies rule identifiers, accepted
requirement traceability, exact construct count, fixture fields and coverage,
source digests, structured diagnostics, rename edits, stale-revision rejection,
and identity maps. Its built-in mutations prove rejection of a sixth construct,
unknown requirement, missing protocol shape, incomplete expected result, and
changed source.

The M11 subset is fixed for comparing M12 candidates. It is proposed language
and protocol material, not the accepted broader 20–30 construct AIL core.

## Accepted foundation

- UC-001 request validation and conditional persistence
- UC-003 public and persisted schema evolution
- Accepted `APP-*`, `LANG-*`, `PROTO-*`, and benchmark requirements
- 37 public behavior fixtures and five separately held private behaviors
- Rust, Go, Python, and TypeScript V1/V2 reference implementations
- Cross-language normalized parity
- Eight digest-locked answer-free task starts
- M8a–M8f calibration infrastructure and non-official pilot evidence
- M11 shared compiler-spike contract and fixtures
- Stack-evaluation weights and Rust/TypeScript candidate set

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
- M11 — Compiler-stack spike contract

## Deferred

- M8g–M8o — Official baseline calibration campaign
- M9 — Numeric AIL benchmark targets
- M10 — Illustrative AIL variants
- UC-007 — Architectural regression control

These items require an explicit maintainer decision to resume. They do not
block M12, M13, or the first semantic-oracle implementation milestone.

## Do not start yet

- M13 stack selection before both M12 candidates and their scorecard pass
- A production source tree or root package manager before M13 selects the stack
- Candidate-specific syntax, semantics, diagnostics, or fixture changes
- The broader 20–30 construct language core
- Native code generation, production runtime work, or general concurrency
- Official agent or performance evidence

## Blockers

None. Both M12 candidates can start from the same checked M11 contract.

## Handoff checklist

After meaningful work:

- keep both candidates disposable and behaviorally identical;
- run every shared fixture through both candidates;
- report candidate friction as evidence instead of changing the contract;
- run `python3 prototypes/check.py`;
- run `python3 specs/tools/core_contract.py check`;
- run `python3 tools/check_docs.py`; and
- update this file and the roadmap only when the M12 exit criterion passes.
