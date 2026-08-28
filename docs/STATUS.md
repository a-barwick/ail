# Current status

Last updated: 2026-08-28

## Limits

AIL cannot implement a general production service.

Repeated work is sequential `map` and one bounded outbound map over immutable
`List<T, N>`. There are no general loops, collection library, mutation,
indexing, nested lists, general concurrency, general networking or routing,
package registry, foreign-code boundary, production runtime, native backend,
TLS, authentication, or deployment system. Recursion is rejected, not bounded.

The HTTP host is one loopback adapter: `POST /v1/lookups:batch`. The lookup
provider is an immutable local catalog, not a network dependency. The endpoint
has no authentication. Audit is process-local, retains complete result values,
and requires a restart after 256 admitted executions. The interpreter cannot
preempt stuck host code, enforce a hard deadline itself, or roll back remote
effects.

`ailc check` and `ailc publish` load capability interfaces only from
`capabilities.json` at the source-set root, the same layer as
`architecture.json`. They do not search another filename, the repository root,
or `.ail/`. If the file is absent, the capability environment is empty and
capability-using source sets fail because the interfaces were not supplied. If
the file is present, they load it as a `CapabilityEnvironment`. Malformed JSON,
a shape that is not that environment, duplicate capability names, and source
that uses an undeclared interface are rejected. The loaded path and digest are
compiler facts. The driver does not invent capability syntax or a project
manifest. When `architecture.json` is present, check evaluates that project
policy and fails with an architecture diagnostic. Check writes no revision.
`ailc publish` writes one revision only after the same checks pass. Neither
command executes the program. Their architecture result reports the six-case
behavior gate as `not-run` with zero passed cases.

`ailc format` and `ailc reconstruct` also read live source, and like check and
publish they do not execute the program. `ail-run` is the only command that
executes.

`ail-run` executes only the frozen bytes of the revision `<dir>/.ail/current`
names. It cannot run a live edit, select another revision, roll back, or emit
JSON. It supplies no capabilities and takes only `--text`, `--int`, and
`--bytes` arguments; a record, variant, or list argument has no command-line
spelling. It does not freeze or read `architecture.json`, so the
`architecture_settings_digest` in `revision.json` names a live file the runner
never opens. Running one published example is not a proof of the language and
is not a production service.

Structured findings expose only facts the checkers already compute. A finding
states the constraint that must hold and the value the checker measured. It
never names the edit that would satisfy the constraint, because the checker has
no fact that chooses one repair over another. Findings have no stable
requirement text for every code: a code whose expected and actual facts name no
constraint reports none rather than guessing one. Findings are not a protocol
surface with its own conformance fixtures; they are the `ailc` check and publish
output.

The architecture checker implements the existing metric and policy contract. It
does not make AIL a general application platform.

[language.md](language.md) describes what the compiler does implement.

## Checks after compiler changes

The repository gate is `./tools/check`. Run it from the repository root. It
needs Rust 1.87.0 via rustup (with rustfmt and clippy) and Python 3. When
rustup is already present, the command installs that toolchain and those
components. It then runs every command below and stops on the first failure.
CI runs the same command on a clean checkout.

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
