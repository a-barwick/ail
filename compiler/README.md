# AIL compiler

This directory contains the Rust compiler and deterministic interpreter.

## Implemented

- lossless UTF-8 tokenization, typed syntax, recovery, and canonical formatting;
- name resolution, local type inference, exact type checking, capability-effect
  checking, and structured diagnostics;
- immutable canonical revisions with SHA-256 identities and revision-scoped
  syntax and symbol handles;
- elaborated semantic inspection, validated rename, canonical edits, and
  complete identity maps;
- ordered multi-file source sets, schema identities, semantic relationships,
  exact impact queries, semantic diffs, and atomic candidate validation;
- deterministic interpretation with caller-supplied capabilities and ordered
  observed calls;
- architecture snapshots, compatible deltas, policy evaluation, bounded
  incomplete results, and atomic publication; and
- explicit modules, import aliases, qualified references, and checked local and
  imported function calls;
- immutable structural `List<T, N>` values and contextual sequential `map`;
- whole-list cardinality and element validation before capability checks or
  calls; and
- linked source-set function inspection for module, list element identity,
  bound, effects, capabilities, and dependencies; and
- revision-bound outbound operation contracts with explicit timeout and
  cancellation controls, closed completion results, and observed request facts;
  and
- one fixed-limit outbound `parallel map` with input-aligned results,
  cooperative batch cancellation, and separate start/completion traces.

## Calls and modules

Each source file declares one `module name;` and may import other modules with
`import dependency;` or `import dependency as alias;`. Imports control
visibility. Bare names remain available when unique. Qualification such as
`domain.Request` or `alias.validate` resolves collisions explicitly. Missing,
duplicate, ambiguous, inaccessible, or cyclic imports and references are
rejected.

AIL calls use `function(arguments)`. Arguments are checked exactly and evaluated
left to right. Same-named capability parameters propagate through calls, and the
caller must declare every transitively reachable capability effect. Direct and
mutual recursion are rejected.

`EvolutionWorkspace::execute` runs a checked source-set entry point. An
unqualified name works when unique; `module.function` selects among repeated
function names. See `examples/composed-service/` for a working three-file
program.

## Bounded lists and map

`List<T, N>` stores zero through `N` immutable values in order. The compiler
represents the element and bound structurally, resolves aliased and qualified
element types during linking, and uses exact bound equality. `map item in items
{ ... }` evaluates its source once and its body sequentially for every stored
index, producing one aligned result with the same declared bound.

External list arguments are completely validated before capability availability
checks or calls. The feature has no literals, indexing, mutation, filter, fold,
general loops, or nested lists. See
`examples/batch-cancellation/` for the executable three-module service.

## Cooperative outbound requests

An outbound operation remains an ordinary capability call in source. Its
host-supplied contract identifies the timeout and opaque `Cancellation`
arguments, maximum milliseconds, result variant, and persistent timeout/cancel
case identities. The interpreter uses a separate outbound provider path and
turns cooperative timeout or cancellation into closed AIL values. It does not
provide URLs, retries, asynchronous execution, hard preemption, or remote
rollback. See `examples/outbound-request/`.

## Bounded outbound workflow

M31 accepts `parallel map item in requests limit 3 { dependency.fetch(...) }`
for exactly one outbound operation over a bounded-list parameter. Arguments may
use values, construction, built-ins, and effect-free helpers; direct or
helper-mediated capability operations in arguments are rejected. The
interpreter prepares and validates the whole batch before work, starts in input
order with at most the fixed limit active, and stores host completions at input
positions. It records only successful starts and gives synthesized cancellation
results to started active calls without inventing host completion order. See
`examples/batch-lookup/`.

## APIs

- `check_source` checks one source revision and returns canonical source,
  elaborated type facts, and structured diagnostics.
- `Workspace` stores immutable revisions, inspects revision-scoped handles, and
  validates atomic renames.
- `EvolutionWorkspace` stores ordered source sets, reports impact, validates a
  complete candidate, exposes it to a behavior oracle, and publishes only after
  all checks pass.
- `EvolutionWorkspace::inspect_function` exposes revision-bound linked function,
  module, list, effect, capability, and dependency facts.
- `EvolutionWorkspace::validate_source_architecture_change` derives functions,
  calls, control flow, capabilities, and configured state ownership from a
  complete checked AIL source candidate, evaluates architecture policy, and
  publishes the source revision only when behavior and architecture pass.
- `architecture_snapshot` derives the implemented four-scope, seven-metric
  architecture result or an explicit incomplete result.
- `ArchitectureWorkspace::validate_architecture_change` compares a candidate
  with its base, evaluates policy, and publishes one child only when behavior and
  architecture pass.

The exact protocol and language contracts are under [`../specs`](../specs/README.md).

## Unsupported

There is no general iteration or collection library, mutation, concurrency,
general networking, package registry, foreign-function interface, production runtime,
native backend, JIT, LLVM lowering, or deployment system. Repeated work is
limited to sequential map and one direct fixed-limit outbound map over bounded
lists; unrestricted concurrency remains unsupported. The interpreter is for
semantic execution and tests. Its outbound provider is synchronous and
cooperative. Recursive calls are rejected.

## Verify

From the repository root:

```bash
cargo +1.87.0 fmt --all --check
cargo +1.87.0 test --workspace
cargo +1.87.0 clippy --workspace --all-targets -- -D warnings
cargo +1.87.0 test --workspace --test m28_composition
cargo +1.87.0 test --workspace --test m29_bounded_lists
cargo +1.87.0 test --workspace --test m30_outbound_requests
cargo +1.87.0 test --workspace --test m31_bounded_outbound_workflows
cargo +1.87.0 test -p ail-service-host --test m32_pinned_http_service
python3 specs/tools/bounded_list_contract.py check
python3 specs/tools/outbound_request_contract.py check
PATH="$HOME/.cargo/bin:$PATH" python3 benchmarks/tools/harness.py verify --language ail --visibility public
```
