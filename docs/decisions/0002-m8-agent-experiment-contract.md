# ADR 0002: Defer the agent experiment campaign

- Status: Deferred
- Date: 2026-07-18

## Context

The repository contains locked calibration tooling for an agent-versus-baseline
measurement campaign. That campaign is not running, and it does not define
current compiler behavior.

## Decision

Keep the machine-readable contract in
`benchmarks/calibration/experiment-contract.json` and the tooling under
`benchmarks/calibration/`. Do not treat the campaign as a current language or
product plan. Do not start it by default.

This path is retained so the locked calibration contract can keep pointing at a
stable authority file.

## Consequences

Compiler work proceeds from [the language](../language.md) and
[current limits](../STATUS.md). Changing the calibration JSON still requires
the calibration verifier, because that contract is locked independently of this
prose.

## Alternatives considered

Delete the authority file: the locked experiment contract still names this
path, so removing it would force a fixture-lock change without changing
compiler behavior.

## Validation

`python3 benchmarks/tools/harness.py verify-calibration` continues to accept
the locked contract. This ADR does not authorize a live measurement run.
