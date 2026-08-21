# Architecture-control workload

The `CancelJob` workload adds one operation to a 24-operation service. Malformed
requests make no store call; valid cancellation attempts make one conditional
store call and return a closed outcome.

Three candidates pass behavior tests. The domain-owned candidate is accepted.
The centralized and helper-split candidates are rejected because transport gains
responsibility, authority, state access, or dependency concentration. Analysis
must return facts and contributors bound to the candidate revision; denied or
incomplete changes publish no child revision.

The exact semantics, policy, fixtures, and budgets are in
[specs/architecture.md](../../specs/architecture.md) and its machine-readable
acceptance fixtures.
