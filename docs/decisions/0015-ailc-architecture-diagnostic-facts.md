# ADR 0015: `ailc` prints the architecture facts the checker computed

- Status: Accepted
- Date: 2026-08-24

## Context

`ailc check` printed architecture findings as `code:scope:rule`. The
architecture checker already computes the numbers behind every finding:
`AIL.ARCH.HOTSPOT_GROWTH` carries `base_cfc`, `candidate_cfc`, `base_context`,
and `candidate_context`; `AIL.ARCH.BOUNDARY` carries each forbidden edge with
its source unit, target unit, and edge kind. The CLI discarded all of it.

A reader who saw `AIL.ARCH.HOTSPOT_GROWTH:transport:dispatch:M23-POL-DISPATCH-NO-GROWTH`
had to open `architecture.json`, find the budget, and reconstruct the metric by
hand to learn how far over budget the candidate was. An agent repairing the
workspace had to guess the same numbers. Source-set diagnostics already print
`key=value` details such as `expected.type=Text`, so check output was
inconsistent about facts.

## Decision

`ailc check` and `ailc publish` print each architecture finding as
`code:scope:rule:` followed by the finding's `facts` object flattened into
deterministic `key=value` pairs. Objects use their existing ordered keys,
arrays use their index, and nested keys join with `.`:

```
AIL.ARCH.HOTSPOT_GROWTH:transport:dispatch:M23-POL-DISPATCH-NO-GROWTH: base_cfc=4 base_context=12 candidate_cfc=7 candidate_context=5
AIL.ARCH.BOUNDARY:group:transport:M23-POL-GROUP-DEPENDENCY: forbidden_group_edges.0.kind=calls forbidden_group_edges.0.source=transport:dispatch forbidden_group_edges.0.source_group=transport forbidden_group_edges.0.target=domain:work forbidden_group_edges.0.target_group=domain
```

No finding is invented, reworded, or repaired into advice. The printed pairs
are the evaluator's own `facts` values.

## Consequences

Architecture diagnostics now name the measured value and the policy value, so
"what is over budget, and by how much" is a compiler fact rather than reader
inference. Findings stay bounded because the evaluator already bounds its
response.

Existing architecture diagnostic lines gain a trailing `:` and the fact pairs.
Callers that matched the whole line change; callers that match the code, scope,
or rule do not.

## Alternatives considered

- Pretty multi-line rendering with carets: rejected. Structured `key=value`
  pairs already match the source-set diagnostic format and stay greppable.
- A separate `ailc explain` command: rejected. That is a new CLI surface for
  facts the failing command already holds.
- Deriving a suggested rewrite from the facts: rejected. The checker knows the
  measured value and the budget; it does not know which repair the author
  wants.

## Validation

`cargo +1.87.0 test -p ail-compiler --test ailc_check` asserts
`compiler/examples/architecture-denied` reports
`AIL.ARCH.BOUNDARY:group:transport:M23-POL-GROUP-DEPENDENCY:` with the
forbidden edge's source group, target group, source unit, and target unit.
