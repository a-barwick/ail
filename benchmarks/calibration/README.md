# M8 calibration evidence contract

Status: **M8b contract; no official evidence**

This directory defines the machine-checkable evidence boundary for the M8
baseline campaign. It implements the accepted
[M8 agent experiment decision](../../docs/decisions/0002-m8-agent-experiment-contract.md)
without running an agent or measuring a baseline.

`experiment-contract.json` locks the M8a treatment, the eight M7 task starts,
the two rendered prompts, the six token categories, permissions, limits,
terminal classes, performance safety rules, and evidence schema registry.
`contract-lock.json` locks that configuration, every calibration schema, the
verifier, and the synthetic campaign recipes by SHA-256.

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
