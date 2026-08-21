# AIL compiler

Rust compiler and deterministic interpreter. Language overview:
[docs/language.md](../docs/language.md). Exact rules: [specs/](../specs/README.md).

## Calls and modules

Each source file declares `module name;` and may import with
`import dependency;` or `import dependency as alias;`. Bare names work when
unique. Qualification such as `domain.Request` or `alias.validate` resolves
collisions. Missing, duplicate, ambiguous, inaccessible, or cyclic imports are
rejected.

Calls use `function(arguments)`. Arguments are checked exactly and evaluated
left to right. Same-named capability parameters propagate through calls. The
caller must declare every transitively reachable capability effect. Direct and
mutual recursion are rejected.

`EvolutionWorkspace::execute` runs a checked source-set entry point. See
`examples/composed-service/` for a working three-file program.

## Bounded lists and map

`List<T, N>` stores zero through `N` immutable values in order. `map item in
items { ... }` evaluates its source once and its body sequentially for every
stored index. External list arguments are completely validated before
capability checks or calls. See `examples/batch-cancellation/`.

## Cooperative outbound requests

An outbound operation is an ordinary capability call. Host-supplied metadata
identifies timeout and `Cancellation` arguments and the timeout/cancel result
cases. The interpreter uses a separate outbound provider path. It does not
provide URLs, retries, asynchronous execution, hard preemption, or remote
rollback. See `examples/outbound-request/`.

## Bounded outbound workflow

`parallel map item in requests limit 3 { dependency.fetch(...) }` runs exactly
one outbound operation over a bounded-list parameter. Arguments may use values,
construction, built-ins, and effect-free helpers; capability operations in
arguments are rejected. The interpreter prepares the whole batch, keeps at most
the fixed limit active, and stores completions at input positions. See
`examples/batch-lookup/`.

## APIs

- `check_source` checks one source revision and returns canonical source,
  elaborated type facts, and structured diagnostics.
- `Workspace` stores immutable revisions, inspects revision-scoped handles, and
  validates atomic renames.
- `EvolutionWorkspace` stores ordered source sets, reports impact, validates a
  complete candidate, and publishes only after all checks pass.
- `EvolutionWorkspace::inspect_function` exposes linked function, module,
  list, effect, capability, and dependency facts.
- `EvolutionWorkspace::validate_source_architecture_change` derives architecture
  facts from a checked AIL candidate, evaluates policy, and publishes only when
  behavior and architecture pass.
- `architecture_snapshot` derives the implemented architecture result or an
  explicit incomplete result.
- `ArchitectureWorkspace::validate_architecture_change` compares a candidate
  with its base, evaluates policy, and publishes one child only when both
  behavior and architecture pass.

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
