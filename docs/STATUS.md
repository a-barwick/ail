# Current status

Last updated: 2026-08-21

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

`ailc check` uses an empty capability environment. Capability-using source
sets fail until a library caller supplies the interfaces. The driver does not
invent capability syntax or a project manifest.

The architecture checker implements the existing metric and policy contract. It
does not make AIL a general application platform.

[language.md](language.md) describes what the compiler does implement.

## Checks after compiler changes

```bash
cargo +1.87.0 fmt --all --check
cargo +1.87.0 test --workspace
cargo +1.87.0 clippy --workspace --all-targets -- -D warnings
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
