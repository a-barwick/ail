# M8 baseline calibration execution plan

Status: **Accepted 2026-07-19**

Progress: P0 and M8a through M8c are complete. M8d is active. The starting-state section
records the condition when this plan was accepted.

Documentation role: operational delivery plan for M8. The
[roadmap](roadmap.md) owns milestone scope, dependencies, non-scope, and exit
criteria. The accepted [benchmark policy](benchmarks/README.md), numbered
[requirements](requirements/reference-slice.md), and frozen contracts under
`../benchmarks/contracts/` remain authoritative for experiment behavior.

## Outcome

M8 establishes the mainstream-language control group that M9 uses to set AIL
success targets. It does not implement AIL.

The complete campaign requires:

- at least 10 successful trials for each of two tasks in Rust, Go, Python, and
  TypeScript: 80 successful agent trials in total;
- separate retention and accounting for every unsuccessful and timed-out run;
- at least 30 warm-state performance runs per baseline: 120 measurements;
- at least 30 cold-start and corpus runs per baseline: 120 measurements;
- complete raw and summarized evidence for context, edits, validation, repair,
  correctness, elapsed time, latency, throughput, startup, and memory; and
- a reproducible report with no AIL comparison or target selection.

M8 is therefore an empirical campaign, not one implementation task. It is
delivered through 15 sequential submilestones, M8a through M8o. Only one
submilestone is active at a time.

## Starting state

The M7 parity implementation and candidate freeze are merged on `main`, but M7
is not formally complete. Its frozen V1 checkpoints contain working UC-001
implementations and do not provide a genuine answer-free starting workspace for
that implementation task.

The existing `codex/m8-baseline-calibration` worktree is a non-authoritative
pilot. It contains useful task-start experiments and five Python trial records,
but the agent configuration changed during those trials and the one-pass
protocol cannot measure normal tool use or repair cycles. None of that evidence
counts toward M8's required totals.

Official M8 progress begins at zero successful agent trials and zero performance
measurements.

## Prerequisite P0 — Correct and close M7

P0 is a separate milestone-closing task, not M8a.

### Scope

- Preserve `codex/m8-baseline-calibration` and its untracked pilot artifacts
  without merging them as official evidence.
- Work from a clean `main` worktree on a scoped branch.
- Create first-class, deterministic task-start packages for both tasks in all
  four languages.
- Give UC-001 agents public V1 contracts and ordinary visible tests with an
  explicit implementation hole and no completed handler or validation answer.
- Give UC-003 agents the accepted V1 implementation and ordinary visible V2
  task checks without V2 reference implementation source.
- Keep other baseline languages, private fixtures, hidden locations, completed
  reference answers, and freeze-only metadata outside each agent-visible
  workspace.
- Lock every task-start file and source tree by digest independently of the
  later trial runner.
- Add executable checks for configuration completeness, deterministic rebuilds,
  answer leakage, protected artifacts, starting-state behavior, and expected
  public failure or incompleteness.
- Re-run the complete M7 parity gate with the exact frozen tools and private
  package.

### Non-scope

- Agent trial execution
- Performance measurements
- Choosing the measured model or agent protocol
- M8 evidence schemas or reporting
- AIL implementation, syntax, targets, or illustrative source

### Focused verification

```bash
python3 benchmarks/tools/harness.py self-test
python3 benchmarks/tools/harness.py verify-all
python3 benchmarks/tools/fixtures.py manifest --check
python3 tools/check_docs.py
```

### Exit criterion

All eight answer-free task starts are independently locked and pass their
starting-state checks. Every baseline still produces identical normalized
public and private behavior under the exact M7 environment. The roadmap and
status identify M7 as Complete and M8a as the next active work.

## Preparation submilestones

### M8a — Freeze the experiment contract

Record a decision covering:

- the measured model, agent, reasoning, sampling, and version identity;
- one fixed interactive tool-use protocol for every language and task;
- the prompt wrapper around the already frozen task text;
- initial context and available normal language tools;
- tokenization, repeated-context accounting, and category attribution;
- filesystem, subprocess, and network permissions;
- time, token, retry, and termination rules;
- reference host or container identity; and
- the distinction between a configuration rejection, failed trial, timed-out
  trial, and successful trial.

The one-pass file-generation pilot is not the default protocol because it
prevents normal source discovery, validation attempts, and repair cycles. A
different protocol requires an explicit reviewed decision before official
evidence exists.

Exit criterion: two reviewers can independently determine exactly how a trial
starts, proceeds, stops, and counts. The 100,000-input-token safety rule is
feasible or is amended through a reviewed requirement decision before official
collection.

### M8b — Build the evidence contracts and verifier

Define schemas and digest locks for:

- agent trial records;
- raw model and tool events;
- warm-state measurements;
- cold-start and memory measurements;
- campaign configuration and ordering;
- evidence indexes; and
- the final calibration report.

Implement `python3 benchmarks/tools/harness.py verify-calibration` before
collecting official evidence. Synthetic fixtures must prove acceptance and
rejection for missing counts, duplicate trial identities, changed inputs,
invalid hashes, incomplete token categories, missing raw events, unaccounted
exclusions, incorrect summaries, and mixed configurations.

Exit criterion: the empty, pilot, partial, malformed, and complete synthetic
campaigns receive the expected stable results.

### M8c — Implement the interactive agent runner

The runner must:

- build a fresh locked workspace;
- enforce the M2 manifest-start gate before invoking the agent;
- expose only the locked normal language tools;
- capture every model input and tool result;
- count agent edits, validation attempts, incomplete validations, and repair
  cycles using the frozen definitions;
- enforce permissions and limits;
- retain raw events and final source by digest; and
- never apply a model-proposed change outside the recorded agent workflow.

Exit criterion: fake and dry event streams prove correct accounting and stable
classification for success, failure, timeout, permission violation, token
limit, and incomplete evidence.

### M8d — Implement correctness verification and replay

The private package remains outside every agent-readable path. After an agent
finishes, the verifier runs the full public and private correctness oracle,
seeded-consumer checks, protected-artifact checks, permissions checks, and
final-revision completion-evidence checks.

Exit criterion: deliberately incomplete, answer-exposing, revision-mismatched,
and seeded-regression runs cannot be classified successful. A clean run can be
functionally replayed from its locked manifest.

### M8e — Implement performance measurement

Add equivalent per-language adapters for:

- readiness and warm-up;
- the selected shared corpus;
- monotonic timing;
- per-handler latency samples;
- throughput;
- p50, p95, and p99 latency;
- variance, load, and affinity; and
- functional correctness before a measurement counts.

The same harness must also define and measure process creation and readiness,
cold startup time, idle and peak resident memory, package and dependency
identity, attempted external access, corpus exit status, and functional output
and trace correctness.

Exit criterion: deterministic warm-state and process-measurement tests and one
non-official warm and cold pilot per baseline pass or return the expected stable
safety-limit classification.

### M8f — Run readiness pilots and freeze the campaign

Run one non-official agent pilot for every language/task pair and one warm and
cold pilot for every baseline. Freeze all configuration artifacts, digests, and
the predeclared balanced trial order.

Exit criterion: every configuration demonstrates that a successful run under
the locked limits is possible and every evidence path passes
`verify-calibration`. If not, stop and revise the design before official
collection.

## Official agent campaign

M8g through M8k each run one balanced two-round batch. A round contains one
attempt for each of the eight language/task configurations in the predeclared
order. Batching two rounds keeps each task bounded while avoiding one task per
statistical repetition.

| Submilestone | Official agent rounds | Attempts |
| --- | ---: | ---: |
| M8g | 1–2 | 16 |
| M8h | 3–4 | 16 |
| M8i | 5–6 | 16 |
| M8j | 7–8 | 16 |
| M8k | 9–10 | 16 |

Each round is evidence-only:

- verify every frozen input before the first attempt;
- run attempts sequentially on the reference host;
- record every success, failure, and timeout;
- add raw evidence and indexes without changing code or configuration; and
- run `verify-calibration` against the accumulated campaign.

After rounds 9–10, M8k runs balanced make-up rounds until every configuration
has at least 10 successful trials. If more than one additional task is needed,
name them M8k.1, M8k.2, and so on. Failures remain in the evidence set. M8k also
checks the complete agent counts before performance collection begins.

## Official performance campaign

One performance round records one warm and one cold/corpus measurement for each
baseline: eight records. Each submilestone contains 15 balanced rounds.

| Submilestone | Performance rounds | Cumulative records |
| --- | ---: | ---: |
| M8l | 1–15 | 120 |
| M8m | 16–30 | 240 |

These submilestones are evidence-only. An invalid host-load, readiness,
affinity, or correctness result remains recorded with its exclusion reason and
is replaced under the same frozen configuration.

## Completion submilestones

### M8n — Audit and report

- Replay the required sample from every agent configuration.
- Verify raw evidence hashes, artifact locks, counts, exclusions, and
  configuration identity.
- Report successful, unsuccessful, and timed-out agent distributions
  separately.
- Report token categories, edits, validations, repairs, elapsed agent time,
  throughput, latency percentiles, startup, idle and peak memory, variance, and
  environment.
- Publish raw counts and distributions without comparing against AIL or
  selecting AIL targets.

Exit criterion: an independent reviewer can reproduce every reported count and
distribution from the locked raw evidence.

### M8o — Close M8

Run the focused verifier, all relevant repository checks, and the documentation
check. Update the roadmap and status only after the parent M8 exit criterion
passes.

Exit criterion: M8 is Complete, M9 is Active, and the accepted M8 branch is
merged and pushed.

## Agentic operating workflow

### Task and branch discipline

1. P0 uses a fresh worktree and scoped branch from clean `main`. It never
   modifies or deletes the pilot worktree.
2. After P0 is reviewed and merged, create one M8 integration branch from the
   updated `main`.
3. Run one submilestone task at a time against the current integration-branch
   tip.
4. Each task reads the repository guidance, current status, this plan, and only
   the authoritative artifacts relevant to its scope.
5. Each task delivers one coherent commit with its focused checks passing and a
   concise handoff naming the next submilestone.
6. Do not run concurrent tasks against the integration branch or reference
   performance host.
7. Do not merge the M8 integration branch until M8o.

### Coordinator model guidance

These settings govern the Codex task coordinating repository work. They do not
select the agent measured inside the benchmark; M8a freezes that configuration
separately.

| Work | Coordinator model | Reasoning |
| --- | --- | --- |
| P0 M7 correction and closure | GPT-5.6 Sol | Max |
| M8a experiment decision | GPT-5.6 Sol | Max |
| M8b–M8d evidence and correctness machinery | GPT-5.6 Sol | High |
| M8e bounded performance measurement | GPT-5.6 Terra | High |
| M8f readiness and freeze | GPT-5.6 Sol | High |
| M8g–M8m evidence-only batches | GPT-5.6 Terra | Medium |
| M8n independent audit and report | GPT-5.6 Sol | Max |
| M8o closure and release checks | GPT-5.6 Sol | High |

Sol is used where cross-language judgment, experimental validity, or final audit
quality dominates cost. Terra is used for bounded implementation and repetitive
evidence collection after the protocol is frozen.

Ultra is not the default. It may be used only for an explicitly approved,
read-only independent audit whose work divides cleanly into non-overlapping
questions. It must not introduce subagents or configuration changes during the
official campaign, and it must never silently become the measured agent.

The initial recommendation for M8a to evaluate is a single GPT-5.6 Sol agent at
High reasoning. M8a must confirm or replace that choice through readiness
evidence before it becomes an experiment lock.

## Invariants after M8f

- No official evidence exists before the M8f freeze.
- No task, test, model, agent, prompt, tool, manifest, runner, permission, or
  environment change is allowed during M8g–M8m.
- A required change after official collection creates a new configuration and
  invalidates affected results for aggregation.
- Failed and timed-out runs remain append-only evidence.
- Evidence tasks do not modify implementation or measurement code.
- M8n reports facts only. M9 owns target selection.

## Launch directives

The phrases `Launch P0` and `Launch M8a` through `Launch M8o` are complete task
directives. An agent receiving one must not require a copied prompt from a prior
conversation.

For every launch directive, the agent must:

1. read `AGENTS.md`, `docs/STATUS.md`, this plan, and the named task section;
2. verify that the named task's dependency and current branch tip are complete;
3. create or use the scoped worktree and branch required by the task discipline;
4. use the listed coordinator model and reasoning level when the task is being
   created with a selectable model; otherwise disclose the mismatch in its
   first update;
5. perform only the named task's scope and required checks;
6. commit one coherent checkpoint and leave a concise handoff; and
7. neither merge nor push unless the named task explicitly permits it.

If a dependency, frozen artifact, private package, toolchain, or configuration
does not match, the agent must stop before performing the named task and report
the exact blocker. It must not repair a previous task implicitly.

| Directive | Dependency and coordinator | Required work | Completion authority |
| --- | --- | --- | --- |
| `Launch P0` | M7 is active; Sol Max | Complete P0 exactly as specified above: preserve the pilot, build and lock all eight answer-free starts, rerun M7 parity, and advance status only if every P0 exit criterion passes. | Commit only; do not merge or push. |
| `Launch M8a` | P0 merged; Sol Max | Complete **M8a**: record the experiment-contract decision, select the candidate measured agent, and resolve the interactive 100,000-token safety rule without official evidence. | Commit only; do not build the runner or collect trials. |
| `Launch M8b` | M8a accepted; Sol High | Complete **M8b**: add calibration schemas, locks, synthetic fixtures, and `verify-calibration`. | Commit only; no agent or performance trials. |
| `Launch M8c` | M8b accepted; Sol High | Complete **M8c**: implement the interactive agent runner and its event, edit, validation, repair, limit, and permission accounting. | Commit only; dry and fake streams only. |
| `Launch M8d` | M8c accepted; Sol High | Complete **M8d**: implement post-run public/private correctness, seeded-role, protected-artifact, completion-evidence, and replay verification. | Commit only; no official evidence. |
| `Launch M8e` | M8d accepted; Terra High | Complete **M8e**: implement warm-state plus cold-start, RSS, package, and external-access measurement with non-official pilots. | Commit only; no campaign collection. |
| `Launch M8f` | M8e accepted; Sol High | Complete **M8f**: run readiness pilots for every configuration and freeze the campaign only if every readiness exit criterion passes. | Commit only; this is the final configuration-change boundary. |
| `Launch M8g` | M8f frozen; Terra Medium | Complete agent rounds 1–2: 16 sequential, evidence-only attempts across the eight frozen configurations. | Commit append-only evidence only. |
| `Launch M8h` | M8g accepted; Terra Medium | Complete agent rounds 3–4 under the unchanged M8f configuration. | Commit append-only evidence only. |
| `Launch M8i` | M8h accepted; Terra Medium | Complete agent rounds 5–6 under the unchanged M8f configuration. | Commit append-only evidence only. |
| `Launch M8j` | M8i accepted; Terra Medium | Complete agent rounds 7–8 under the unchanged M8f configuration. | Commit append-only evidence only. |
| `Launch M8k` | M8j accepted; Terra Medium | Complete agent rounds 9–10, then run balanced make-up rounds as `M8k.1`, `M8k.2`, and so on until every configuration has 10 successes. Verify complete agent counts. | Commit append-only evidence only. |
| `Launch M8l` | M8k accepted; Terra Medium | Complete performance rounds 1–15: 15 warm and 15 cold/corpus measurements per baseline, retaining invalid results and exclusions. | Commit append-only evidence only. |
| `Launch M8m` | M8l accepted; Terra Medium | Complete performance rounds 16–30 and any required same-configuration replacement measurements. | Commit append-only evidence only. |
| `Launch M8n` | M8m accepted; Sol Max | Complete **M8n**: independently audit locks, raw evidence, counts, replays, exclusions, and the fact-only report. | Commit report and audit evidence; do not select AIL targets. |
| `Launch M8o` | M8n accepted; Sol High | Complete **M8o**: run final verification, update status and roadmap if the M8 exit criterion passes, merge the accepted M8 branch, push it, and activate M9. | Merge and push only after every final check passes. |

The model setting is a coordinator recommendation, not the identity of the
measured benchmark agent. M8a freezes the measured agent separately. Ultra
remains reserved for an explicitly approved read-only audit and is never
implied by a launch directive.
