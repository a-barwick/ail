#!/usr/bin/env bash
# Cloud Agent install for AIL.
#
# Prepares the pinned Rust toolchain and primes caches so every command in
# README.md / AGENTS.md runs unchanged: `cargo +1.87.0 fmt|test|clippy|run`,
# the specs/tools and tools Python checkers, and
# `benchmarks/tools/harness.py verify`.
#
# The script is idempotent: the rustup install, symlinks, `cargo fetch`, and
# `cargo build` all converge without rewriting Cargo.lock, so it is safe to run
# repeatedly and against a cached or partially prepared VM.
set -euxo pipefail

TOOLCHAIN=1.87.0

# Install the pinned toolchain (with rustfmt and clippy) into the system rustup
# that the interactive agent shell uses (RUSTUP_HOME=/usr/local/rustup), and make
# it the default. The workspace pins rust-version = 1.87, which bare `cargo`
# would otherwise reject on the base image's older default.
rustup toolchain install "$TOOLCHAIN" --component rustfmt --component clippy --profile minimal
rustup default "$TOOLCHAIN"

# `benchmarks/tools/harness.py` launches the AIL runner (`cargo run ... --bin
# ail-benchmark`) with a stripped environment: no RUSTUP_HOME, CARGO_HOME, or
# HOME. rustup and cargo then resolve the home directory from the passwd
# database (~/.rustup, ~/.cargo). Point those at the single system installation
# so one toolchain and one crate cache serve both the agent shell and the
# harness. Only create the links when nothing is already there, so a re-run on a
# VM that already has a real ~/.cargo/~/.rustup is left untouched.
[ -e "$HOME/.rustup" ] || ln -s /usr/local/rustup "$HOME/.rustup"
[ -e "$HOME/.cargo" ] || ln -s /usr/local/cargo "$HOME/.cargo"

# Prime the crate cache so the `--offline --locked` benchmark runner builds
# without network access.
cargo "+$TOOLCHAIN" fetch --locked

# Build the workspace once (compiler, service host, and the ail-benchmark binary)
# so the first checks and the lookup host start quickly.
cargo "+$TOOLCHAIN" build --workspace
