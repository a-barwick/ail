# M23 architectural regression acceptance package

Status: **Accepted — M23 evidence package**

This language-independent package freezes the UC-007 acceptance evidence. It is
non-normative, defines no AIL syntax, protocol transport, or `ARCH` diagnostic,
and is not the M24 contract. The machine-readable lock is
[`architecture-acceptance.json`](../specs/architecture-acceptance.json); its
three byte-locked fixtures describe R1, the candidates, and the non-normative
oracle. [`architecture-acceptance-reviews.json`](../specs/architecture-acceptance-reviews.json)
records two independent approvals bound to both
fixture-set digest `ab362d96d89cbba779743dd8a3050b2bd4452ff6daddf3e7ae65109207f7e3ed` and review-subject digest
`fcb454729e6c2c228802d471d97c1eecd7abc793da407b0ced2bbd76fe9624cf`. The latter also binds the use case, requirements, metrics,
rules, traceability, policy, budgets, and every child path and hash.

## Workspace and behavior

R1 enumerates exactly 24 named operations; `CancelJob` is absent. A deterministic
expansion gives every operation stable contract, transport registration,
transport adapter, domain handler, and behavior-test identities, module/group
membership, CFG facts, and typed edges. Coverage includes all five project
groups and explicitly leaves `external:job-api-clients` unchecked.

`CancelJob` rejects malformed requests without a store call. Queued and running
jobs return `Cancelled` after exactly one atomic
`jobs_store.cancel_if_active` call and become cancelled. Missing jobs return
`NotFound`; completed or cancelled jobs return `NotCancellable`; each makes one
call and no state change. Clock, network, and telemetry are absent. Every
candidate passes these six cases. The valid candidate adds a domain handler and
does not grow dispatch. The centralized candidate puts decisions, jobs-store
authority, and jobs state in dispatch. The helper-split candidate moves those
responsibilities to private transport helpers, so each unit and dispatch remain
within thresholds while the transport aggregate still fails.

## Derived facts and policy

The checker derives facts from units and typed edges; fixture classifications
are never trusted. The deliberately minimal metrics are:

- per-unit control-flow complexity `E - N + 2`, with aggregate maximum, sum,
  and ordered contributors;
- unique direct dependency sets;
- capabilities directly declared, received, retained, delegated, or invoked
  by the scope (never transitive reachability);
- state read and write sets;
- dependency-component size from deterministic Tarjan SCCs over typed direct
  dependency edges; and
- minimal context node count.

For this package the minimal context closure starts with the selected unit(s),
then includes each directly incident typed-edge endpoint, directly declared
capability and state site, applicable policy selector, and matching baseline.
Nodes are deduplicated by semantic identity and sorted. It does not traverse a
second dependency hop. Aggregate scopes start with every unit assigned to the
group before applying that same closure. Behavior effects and call ordering are
kept in the behavior oracle, not turned into extra architecture metrics.

The eight `M23-POL-*` rules freeze the group dependency matrix; empty transport
capability and state sets; exact derived R1 dispatch CFC 4/context 3; new-unit
CFC <= 4 and context <= 12; no new cycle; complete policy coverage; and
unchanged policy, baseline, and exception set. A denied or budget-exhausted required analysis is
`incomplete`, never accepted. The expected fixture lists exact findings,
contributors, rules, baseline, coverage, inspection identities, and compact
text. New failures precede unchanged debt.

## Frozen budgets and comparison

Budgets were fixed before implementation results: 24 R1 operations, three
candidates, zero false findings, zero missed required findings, at most 512
semantic nodes and 2,048 typed edges per derived candidate, at most 65,536
UTF-8 bytes per canonical structured scenario result, and compact output at
most 2,048 bytes and 12 newline-terminated lines. Required contributors cannot
be truncated.

Equivalent baselines must use the strongest practical normal, pinned toolchain:

- Rust: rustc, Cargo, rustfmt, Clippy, rust-analyzer, `cargo metadata`, and a
  pinned architecture/complexity checker;
- Go: Go, gofmt, vet, gopls, tests, `go list`, and pinned depguard plus gocyclo
  (or pinned golangci-lint equivalents);
- Python: CPython, mypy, Ruff, pytest, Import Linter, and Radon; and
- TypeScript: Node, tsc, ESLint, Prettier, tests, dependency-cruiser, and pinned
  complexity rules.

Each must implement identical behavior and preserve the same classification and
policy intent, with zero false or missed findings and governance protection.
Parser-specific complexity numbers need not be equal. No baseline run has been
performed. Wall time, peak memory, repair cycles, and comparative model-context
thresholds require future baseline measurement; this package makes no claim
that those values are calibrated.

Run `python3 specs/tools/architecture_acceptance.py check`. The command derives
the complete structured and compact results, rejects 37 mutations, and reports
the pending gate until two distinct approvals bind the current review-subject
digest.
