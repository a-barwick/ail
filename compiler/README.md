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

## Command line

`ailc check <dir>` builds an `EvolutionWorkspace` from the `.ail` files in
that directory whose names pass `valid_source_path`. `ailc check <file>`
builds a one-file workspace. Both use an empty capability environment and the
same complete-coverage claim as the composed-service example. When
`architecture.json` is present next to the named path, check also evaluates
that project policy through `ArchitectureWorkspace` and fails on a denied or
incomplete result. The command prints `ok` only when that workspace is
accepted. Check writes no revision. Diagnostics go to stderr.

`ailc publish <dir>` runs the same checks and writes one revision under
`<dir>/.ail/revisions/published` only when they pass. A failing candidate
writes no revision and leaves an existing store unchanged.

`check_source` is not the meaning of `ailc check`. A file with an unresolved
import fails as `AIL.MODULE.MISSING_IMPORT`. Capability-using examples fail
as `AIL.CAPABILITY.UNKNOWN_INTERFACE` unless a library caller supplies the
environment.

`ailc format <source.ail>` writes canonical source. `ailc reconstruct
<source.ail>` writes the lossless token reconstruction.

`check`, `publish`, `format`, and `reconstruct` all read live source, and no
`ailc` command executes a program. `ail-run <dir> <function>` is the only command
that executes. It runs the published revision `<dir>/.ail/current` names, reading
only the frozen bytes under that revision's `sources/` directory. It verifies
every recorded file digest, the source-set digest, and the capability-environment
digest, rebuilds the frozen set as an `EvolutionWorkspace`, and refuses with an
`AIL.RUN.*` code otherwise. It supplies no capabilities and takes `--text`,
`--int`, and `--bytes HEX` arguments in declaration order. See
[docs/published-bytes-runner.md](../docs/published-bytes-runner.md).

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

The repository gate is `./tools/check` from the repository root. It runs every
compiler check in [docs/STATUS.md](../docs/STATUS.md) and stops on the first
failure. CI runs the same command.

```bash
./tools/check
```

That command runs:

```bash
cargo +1.87.0 fmt --all --check
cargo +1.87.0 test --workspace
cargo +1.87.0 clippy --workspace --all-targets -- -D warnings
cargo +1.87.0 test -p ail-compiler --test ailc_findings
cargo +1.87.0 test -p ail-compiler --test published_runner
cargo +1.87.0 test -p ail-service-host --test m32_pinned_http_service
cargo +1.87.0 test -p ail-service-host --test m33_private_catalog_dogfood
python3 specs/tools/architecture_acceptance.py check
python3 specs/tools/architecture_contract.py check
python3 specs/tools/core_contract.py check
python3 specs/tools/bounded_list_contract.py check
python3 specs/tools/outbound_request_contract.py check
python3 specs/tools/bounded_outbound_workflow_contract.py check
PATH="$HOME/.cargo/bin:$PATH" python3 benchmarks/tools/harness.py verify --language ail --visibility public
python3 benchmarks/tools/fixtures.py check
python3 tools/check_docs.py
```

Focused extra runs already covered by `cargo +1.87.0 test --workspace`:

```bash
cargo +1.87.0 test --workspace --test m28_composition
cargo +1.87.0 test --workspace --test m29_bounded_lists
cargo +1.87.0 test --workspace --test m30_outbound_requests
cargo +1.87.0 test --workspace --test m31_bounded_outbound_workflows
```
