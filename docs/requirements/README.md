# AIL requirements

Status: **Active**

Documentation layer: requirements derived from accepted use cases. The first
numbered requirements were accepted on 2026-07-18 from UC-001 and UC-003.

## Accepted requirement sets

The [initial job-submission reference-slice requirements](reference-slice.md)
contain the accepted `APP-*`, `LANG-*`, `PROTO-*`, and `NFR-*` records for
UC-001 and UC-003. The shared JSON format and measure-first benchmark policy are
documented under [benchmarks](../benchmarks/README.md).

## Proposed requirement sets

The [architectural-health requirements](architectural-health.md) derive
`APP-006`, `LANG-006`, `PROTO-006`, `PROTO-007`, `NFR-006`, and `NFR-007` from
proposed UC-007. They remain proposed until the scaling workspace, architecture
policy, metrics, fixtures, baseline comparison, and fixed budgets are reviewed.
Active M23 prepares that evidence; it does not accept the requirements or
authorize their compiler implementation.

## Requirement classes

- `APP-###`: application behavior or operability
- `LANG-###`: language semantics or source representation
- `PROTO-###`: compiler semantic-interface behavior
- `NFR-###`: measurable performance, scale, portability, security, or
  reliability constraint

An identifier classifies a requirement; it does not make the requirement
normative. Each record also has a status.

## Statuses

- **Proposed:** derived and reviewable, but not accepted.
- **Accepted:** approved as an input to design or specification.
- **Deferred:** valid but outside the current validation slice.
- **Rejected:** considered and intentionally not required.
- **Superseded:** replaced by identified later requirements.

## Requirement record

Each requirement must state:

1. identifier, title, and status;
2. source use cases;
3. precise requirement using observable terms;
4. rationale tied to total agent change cost;
5. acceptance evidence or measurement;
6. target milestone;
7. dependencies and conflicts;
8. whether it constrains the language, protocol, runtime, standard library,
   deployment system, benchmark, or governance; and
9. unresolved questions.

Avoid prescribing a language mechanism in an application requirement. For
example, “the default service runtime must satisfy an agreed tail-latency
envelope without opaque unbounded pauses” is an application or non-functional
requirement. “AIL uses ownership and borrowing” is a candidate design decision.

## Traceability

Every accepted language or protocol rule must reference at least one accepted
requirement or a foundational safety/determinism invariant. Every conformance
fixture must reference the rules it tests.

Prototype results may support, challenge, or refine a requirement. They may not
create one implicitly.
