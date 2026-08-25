# Fixture `release-review`

This is the third compiler-convergence fixture. It is a 35-module workspace,
not a larger version of either earlier fixture. One active pipeline call joins
two distinct nominal signal types whose declarations live in separate modules.
An unrelated presentation module also requests authority that the empty
capability environment does not provide.

The broken workspace already satisfies every task requirement. The repair
changes two source files; `contracts.ail` is immutable. There is no
`architecture.json`, so this fixture does not reuse fixture 1's architecture
complexity result. The point is locating two faults across a source set large
enough that an exhaustive read is material work.

The seven measured two-arm trials used the sealed blind-ledger protocol from
`7981e32`. Their result and limits are in
[RESULTS-release-review.md](../RESULTS-release-review.md).
