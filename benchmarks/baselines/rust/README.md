# Rust job-service baseline

This directory contains the M3 Rust baseline for the frozen UC-001 and UC-003
job-service benchmark.

- `v1/` is the buildable UC-001 checkpoint used as the starting source for the
  priority-evolution task.
- `v2/` is the verified UC-003 reference implementation and benchmark runner.
- `checkpoints.json` freezes both source trees with reproducible SHA-256
  digests.
- `seed-locations.json` identifies stable Rust semantic locations where M7 can
  instantiate the frozen hidden seed categories.
- `runner.json` and the locked verification manifest connect V2 to the
  language-neutral M2 harness.

Both packages use stable Rust 1.88.0. The V2 runner uses only locked crates from
`Cargo.lock`; it does not access the network while executing fixtures.

## Setup

Install the Rust toolchain and fetch the locked dependencies once:

```bash
rustup toolchain install 1.88.0 \
  --profile minimal \
  --component clippy,rust-analyzer,rustfmt
cargo fetch --locked \
  --manifest-path benchmarks/baselines/rust/Cargo.toml
```

## Build and checks

Run these commands from the repository root:

```bash
cargo build --workspace --locked \
  --manifest-path benchmarks/baselines/rust/Cargo.toml
cargo fmt --all --manifest-path benchmarks/baselines/rust/Cargo.toml -- --check
cargo clippy --workspace --all-targets --all-features \
  --manifest-path benchmarks/baselines/rust/Cargo.toml -- -D warnings
cargo test --workspace --locked \
  --manifest-path benchmarks/baselines/rust/Cargo.toml
rustup run 1.88.0 rust-analyzer --version
python3 benchmarks/tools/harness.py verify \
  --language rust \
  --visibility public
```

The V2 tests include the complete 37-case shared fixture corpus in addition to
focused unit tests for validation order and bounds, closed outcomes, exact
effect counts, V1 adapters, persisted records, priority propagation, response
projection, and boundary decode failures.

## Coverage

With `cargo-llvm-cov` installed, reproduce the source coverage summary with:

```bash
rustup component add llvm-tools-preview --toolchain 1.88.0
rustup run 1.88.0 cargo llvm-cov \
  --workspace \
  --all-features \
  --summary-only \
  --manifest-path benchmarks/baselines/rust/Cargo.toml
```

## Run one case

```bash
cargo run --quiet --offline --locked \
  --manifest-path benchmarks/baselines/rust/Cargo.toml \
  -p ail-job-service-v2 -- \
  --case benchmarks/fixtures/public/create_job/uc003-v2-created-priority-high.json
```

## Run the corpus

```bash
cargo run --quiet --offline --locked \
  --manifest-path benchmarks/baselines/rust/Cargo.toml \
  -p ail-job-service-v2 -- \
  --corpus benchmarks/fixtures/manifest.json
```

Each command writes exactly one normalized JSON result to standard output.
