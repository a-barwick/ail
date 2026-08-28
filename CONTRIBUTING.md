# Contributing

Read [the language](docs/language.md) and [current limits](docs/STATUS.md)
first.

To add behavior, write a failing executable check, implement the smallest
semantics that change the result, and run the repository gate. Do not treat
sequential `map` or the pinned lookup host as a path to general collections,
networking, routing, or concurrency.

```bash
./tools/check
```

That command runs every compiler check listed in [docs/STATUS.md](docs/STATUS.md)
and stops on the first failure. CI runs the same command on a clean checkout.

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

Examples illustrate behavior. Numbered specs and fixtures define it.

Record a decision in `docs/decisions/` when a change alters public semantics or
makes an expensive implementation choice.
