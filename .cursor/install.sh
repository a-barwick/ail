#!/usr/bin/env bash
# Cloud Agent install for AIL.
#
# Prepares the pinned Rust toolchain and primes caches so every command in
# README.md / AGENTS.md runs unchanged: `cargo +1.87.0 fmt|test|clippy|run`,
# the specs/tools and tools Python checkers, and
# `benchmarks/tools/harness.py verify`.
#
# The script is idempotent: rustup toolchain installs, `cargo fetch`, and
# `cargo build` all converge without rewriting the lockfile, so it is safe to
# run repeatedly and against a cached or partially prepared VM.
set -euxo pipefail

TOOLCHAIN=1.87.0

# --- System rustup (RUSTUP_HOME=/usr/local/rustup) ---------------------------
# The interactive agent shell resolves `cargo +1.87.0 ...` against the system
# rustup, so install the pinned toolchain with rustfmt and clippy there and make
# it the default (the workspace pins rust-version = 1.87, which bare `cargo`
# would otherwise reject on the base image's older default).
rustup toolchain install "$TOOLCHAIN" --component rustfmt --component clippy --profile minimal
rustup default "$TOOLCHAIN"

# --- Home rustup (~/.rustup, ~/.cargo) ---------------------------------------
# `benchmarks/tools/harness.py` launches the AIL runner (`cargo run ... --bin
# ail-benchmark`) with a stripped environment: no RUSTUP_HOME, CARGO_HOME, or
# HOME. rustup and cargo therefore resolve the home directory from the passwd
# database (~/.rustup, ~/.cargo). Mirror the pinned toolchain there and prime an
# offline crate cache so the `--offline --locked` runner builds without network.
(
  export RUSTUP_HOME="$HOME/.rustup"
  export CARGO_HOME="$HOME/.cargo"
  rustup toolchain install "$TOOLCHAIN" --component rustfmt --component clippy --profile minimal
  rustup default "$TOOLCHAIN"
  cargo "+$TOOLCHAIN" fetch --locked
)

# --- Build once --------------------------------------------------------------
# Compile the workspace (compiler, service host, and the ail-benchmark binary)
# so the first checks and the lookup host start quickly.
cargo "+$TOOLCHAIN" build --workspace
