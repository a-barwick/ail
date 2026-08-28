# AIL

AIL is a small programming language for software written and operated by
agents. Canonical source is the program. The compiler checks types, effects,
and authority so a change can be reviewed without trusting hidden model
reasoning.

The Rust compiler works today on a narrow language. It is not a general
production platform. See [the language](docs/language.md) and
[current limits](docs/STATUS.md).

```ail
module service;
import domain as model;
import validation as checks;

fn handle(request: model.Request) -> model.Response {
  if checks.is_invalid(request) {
    model.Response::Rejected
  } else {
    model.Response::Accepted(model.domain_name(request))
  }
}
```

That program is under `compiler/examples/composed-service/`.

## Build and test

The repository gate is `./tools/check`. It needs Rust 1.87.0 via rustup (with
rustfmt and clippy) and Python 3. It runs every compiler check listed in
[docs/STATUS.md](docs/STATUS.md) and stops on the first failure. CI runs the
same command on a clean checkout.

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

See [compiler/README.md](compiler/README.md) for the Rust APIs and focused
commands.

## Run the pinned lookup host

```bash
cargo +1.87.0 run -p ail-service-host -- service-host/examples/catalog.json 3000
```

```bash
curl --fail-with-body \
  --header 'content-type: application/json' \
  --data '{"requests":[{"key":"ail"},{"key":"missing"}],"timeout_ms":100}' \
  http://127.0.0.1:3000/v1/lookups:batch
```

The catalog is `{"entries":[{"key":"...","value":"..."}]}`. Unknown fields and
duplicate keys stop startup. The host binds only `127.0.0.1`; the port argument
is optional and defaults to `3000`. The endpoint has no authentication.

## Repository map

```text
compiler/      Rust compiler, interpreter, and examples
service-host/  Loopback batch-lookup HTTP host
specs/         Language and compiler contracts plus fixtures
benchmarks/    Job-service cases, baselines, and harnesses
poc/           Measured experiments; see docs/poc.md
docs/          Language overview, limits, workloads, and decisions
tools/         Repository checks
```

Specs and fixtures define required behavior. If prose disagrees with a fixture,
the fixture wins.

The repository does not yet have a license. All rights remain with the copyright
holder until one is added.
