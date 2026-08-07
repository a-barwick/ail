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
  imported function calls.

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

## APIs

- `check_source` checks one source revision and returns canonical source,
  elaborated type facts, and structured diagnostics.
- `Workspace` stores immutable revisions, inspects revision-scoped handles, and
  validates atomic renames.
- `EvolutionWorkspace` stores ordered source sets, reports impact, validates a
  complete candidate, exposes it to a behavior oracle, and publishes only after
  all checks pass.
- `architecture_snapshot` derives the implemented four-scope, seven-metric
  architecture result or an explicit incomplete result.
- `ArchitectureWorkspace::validate_architecture_change` compares a candidate
  with its base, evaluates policy, and publishes one child only when behavior and
  architecture pass.

The exact protocol and language contracts are under [`../specs`](../specs/README.md).

## Unsupported

There is no iteration, general collection library, concurrency, networking,
package registry, foreign-function interface, production runtime, native
backend, JIT, LLVM lowering, or deployment system. The interpreter is for
semantic execution and tests. Recursive calls are rejected.

## Verify

From the repository root:

```bash
cargo +1.87.0 fmt --all --check
cargo +1.87.0 test --workspace
cargo +1.87.0 clippy --workspace --all-targets -- -D warnings
cargo +1.87.0 test --workspace --test m28_composition
PATH="$HOME/.cargo/bin:$PATH" python3 benchmarks/tools/harness.py verify --language ail --visibility public
```
