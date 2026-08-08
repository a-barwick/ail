# Bounded batch-cancellation workload

The service accepts zero through 32 job identifiers, processes stored positions
in order, calls the existing single-item cancellation path once per position,
and returns one closed outcome at the matching position. Duplicates remain
distinct. Oversized or malformed input makes zero capability calls; unexpected
provider faults stop processing without a partial successful result.

The canonical example is `compiler/examples/batch-cancellation/`. Exact
language, inspection, and runtime rules are in
[specs/bounded-lists.md](../../specs/bounded-lists.md). This workload does not
add general iteration, mutable collections, parallelism, retries, or rollback.
