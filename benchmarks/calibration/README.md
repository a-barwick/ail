# M8 calibration evidence contract

Status: **M8e performance measurement contract; no official evidence**

This directory defines the machine-checkable evidence boundary for the M8
baseline campaign. It implements the accepted
[M8 agent experiment decision](../../docs/decisions/0002-m8-agent-experiment-contract.md)
without running an agent or measuring a baseline.

`experiment-contract.json` locks the M8a treatment, the eight M7 task starts,
the two rendered prompts, the six token categories, permissions, limits,
terminal classes, performance safety rules, and evidence schema registry.
`contract-lock.json` locks that configuration, every calibration schema, the
verifiers, the interactive and performance runners, the four performance
adapters, their tests, and the synthetic campaign recipes by SHA-256.

## Interactive runner

`../tools/agent_runner.py` and `../tools/responses_recorder.py` implement the
M8a agent workflow up to the M8d correctness boundary. The runner rebuilds and
verifies a locked task workspace, checks
the exact prompt and pre-start observations, derives a least-privilege
permission profile, and generates an isolated Codex configuration whose
Responses provider is the loopback recorder. Optional tools, network access,
environment inheritance, retries, delegation, history, and update checks are
disabled. UC-003 may create files only under the selected language's V2 source
roots; tests, fixtures, task text, tool configuration, parent paths, private
inputs, and evidence remain unwritable. UC-001 remains limited to its explicitly
editable source files. The generated keys follow the official
[Codex configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference#configtoml);
The loopback recorder binds only `127.0.0.1`, authenticates the Codex control
process with a trial-local token, and retains the upstream API credential
outside the trial tool environment. Before it forwards each Responses request,
it calls the input-token endpoint for the protocol skeleton, fixed request,
and every cumulative input prefix. It rejects the request before forwarding
when the cumulative 500,000-token limit would be exceeded, streams the provider
response back to Codex, and reconciles the completed provider usage with zero
tolerance.

Run one non-official live readiness trial with:

```bash
export OPENAI_API_KEY='...'
python3 benchmarks/tools/agent_runner.py run-live-trial \
  --language python \
  --task UC-001 \
  --output /tmp/ail-m8f-python-uc001
```

Repeat with each of `rust`, `go`, `python`, and `typescript` and both `UC-001`
and `UC-003`. The output directory is required to be new or empty and receives
the redacted request evidence, Codex JSONL, deterministic final-source archive,
and `live-trial.json`. The command verifies the pinned Codex version and binary
digest, runs the locked starting-state check, probes the live input-token
endpoint, and refuses to spawn Codex when the API credential is absent. A
deterministic fake upstream integration test also runs the exact pinned Codex
binary with `--strict-config`; this proves the recorder path and generated
configuration without recording a model pilot.

The command verifies its completed bundle before returning. A retained bundle
can be checked again without a credential or model call:

```bash
python3 benchmarks/tools/agent_runner.py verify-live-trial \
  /tmp/ail-m8f-python-uc001/live-trial.json
```

The runner records complete model, tool, edit, validation, permission, process,
and terminal payloads. It enforces the 500,000 cumulative-input-token and
600-second wall limits, derives the frozen M2 activity counts, terminates the
whole process group under the locked grace rule, and retains deterministic
final source without local Git, dependency, cache, or build artifacts.

M8c uses only fake and dry streams. Its runner outcome is not a successful M8
trial until M8d supplies matching public/private correctness, seeded-consumer,
protected-artifact, and completion-evidence results for the same final revision.

## Correctness and replay

`../tools/correctness.py` owns the post-run verifier boundary. The separately
held private ZIP enters only this process. The verifier checks its M7 digest and
canonical manifest, validates the retained M8c archive and source revision,
rechecks every protected file, and rejects retained files outside the task's
editable files and authorized language source roots.

The functional oracle must return every applicable public case, private
behavior category, and frozen seeded role with a result for the retained
revision. UC-001 requires its UC-001 public cases and the private categories and
seeds that apply to UC-001. UC-003 requires the complete evolved corpus and its
applicable private categories and seeds. A success is replayed from a second
fresh extraction of the same archive; the complete observation must match
before revision-bound completion evidence is emitted.

## Performance measurement

`../tools/performance.py` owns one equivalent measurement policy around four
persistent adapters. Each adapter loads the shared public corpus before
readiness, runs cases in manifest order through the accepted V2 fixture
boundary, and uses its runtime's monotonic nanosecond clock. The harness checks
the complete functional result and ordered store trace before a measurement can
be included.

Warm records retain per-handler samples and record readiness, three corpus
warm-up iterations, elapsed time, request count, throughput, p50/p95/p99,
integer population variance, host load, and affinity. The campaign verifier
recomputes the declared statistics from the retained sample artifact.

Cold records separate process creation from readiness, observe idle and peak
resident memory, run the functional corpus, record exit status, identify every
checkpoint and adapter file plus the dependency lock, and run beneath the
network-denial policy. A network denial is an attempted external access and
excludes the record. Readiness over two seconds or peak RSS over 512 MiB is
retained with the corresponding stable exclusion.

The adapters are outside the M7 checkpoint file lists; instrumentation does not
change the accepted baseline source digests. The eight records under
`pilots/m8e/` are one non-official warm and cold pilot per baseline. They all
pass, but count as zero official measurements.

## Evidence layout

One campaign directory contains:

```text
campaign.json
evidence-index.json
evidence-index.lock.json
report.json
events/*.jsonl
records/*.json
artifacts/*
```

The external evidence-index lock is checked before any indexed artifact is
read. The index then locks every campaign record, raw event stream, report, and
referenced blob. JSON records use canonical two-space formatting. Raw events
use one canonical compact JSON object per line.

The schemas under `../schemas/` cover:

- agent trial records;
- raw model and tool events;
- campaign configuration and ordering;
- evidence indexes;
- warm-state latency and throughput measurements;
- cold-start, readiness, memory, package, and external-access measurements; and
- the fact-only final report.

The verifier additionally enforces relationships that JSON Schema cannot
express: unique identities, digest closure, one configuration, exact prompt
and task-start locks, raw-event continuity, request completeness, token
reconciliation, schedule coverage, terminal-class consistency, exclusions,
minimum complete-campaign counts, and report derivation.

## Token accounting

The provider's per-request usage remains the authoritative total. Before a
request is forwarded, the recorder will send the same request shape to
`POST /v1/responses/input_tokens`. The endpoint returns the exact input count,
including request structure, tools, and schemas; local plain-text estimates do
not. See OpenAI's
[token-counting guide](https://developers.openai.com/api/docs/guides/token-counting).

Category attribution uses `ordered-cumulative-prefix-delta-v1`: count the fixed
instructions and tools, then count each cumulative delivered input prefix in
wire order and assign each delta to that item's one declared category. The six
category totals plus explicit protocol overhead must equal both the preflight
count and provider-reported usage with zero tolerance. Cached input is recorded
as a subset and never deducted. Repeated delivery is counted every time.

M8f must still prove this exact rule against the selected Codex binary and all
eight configurations before official collection. A mismatch is incomplete
evidence, not a permitted approximation.

## Verification

Run the locked contract and all synthetic outcomes:

```bash
python3 benchmarks/tools/harness.py verify-calibration
```

To validate a materialized campaign during later M8 work:

```bash
python3 benchmarks/tools/harness.py verify-calibration \
  --campaign /path/to/evidence-index.json
```

Valid campaigns may report `empty`, `pilot`, or `partial` while evidence is
being prepared or accumulated. A campaign marked `complete` must satisfy the
locked official counts. The synthetic fixtures use one record per
configuration or language so the same rules can be tested without creating
official evidence.

The recipes under `synthetic/` cover valid empty, pilot, partial, and complete
campaigns plus stable rejection cases for malformed evidence, missing counts,
duplicate trial identities, changed inputs, invalid hashes, incomplete token
categories, missing raw events, unaccounted exclusions, incorrect reports, and
mixed configurations.

The no-argument verifier also runs the M8c fake/dry matrix for normal success,
non-zero exit, wall timeout, permission violation, input-token limit, and
incomplete evidence. It checks M2 repair accounting, pre-start rejection,
permission boundaries, deterministic source capture, isolated Codex
configuration, and process-group termination without invoking Codex.
The M8d matrix additionally proves that a complete dry oracle is replayable and
that incomplete source, unexpected answer material, stale completion revision,
seeded regression, and replay divergence cannot be successful.
The M8e matrix derives a deterministic warm record and cold process record from
a fake persistent adapter and validates their schemas, sample statistics,
functional trace, RSS, package, and external-access fields.
