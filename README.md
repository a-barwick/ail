# AIL

AIL is an executable programming language for software agents. Agents are its
primary authors and operators. Humans must still be able to read the canonical
source, inspect compiler facts, review every change, and understand the program's
authority and observable behavior.

The Rust compiler works today. It parses and canonically formats AIL, checks
types and capability effects, stores immutable source revisions, reports
structured diagnostics, executes the supported language in a deterministic
interpreter, computes schema impact, validates atomic multi-file changes, and
enforces a small architecture policy. M28 added ordinary function calls and
explicit modules, import aliases, and qualified references. M29 added immutable
bounded lists and deterministic sequential map.

## What works now

- records, closed variants, functions, local `let` bindings, field access,
  conditionals, exhaustive matching, capability calls, and ordinary AIL calls;
- explicit `module` and `import` headers, import aliases, and qualified
  references for ordered multi-file source sets;
- exact argument and type checking for local and imported calls;
- transitive capability-effect checking;
- deterministic left-to-right argument evaluation and nested interpretation;
- exact structural `List<T, N>` types and sequential binder-style `map`;
- complete external list validation before capability checks or calls;
- rejection of recursive call cycles and import cycles;
- canonical formatting and structured parse, type, import, and effect errors;
- immutable revisions, revision-scoped handles, inspected semantic facts,
  validated rename, and identity maps;
- complete impact results for the implemented schema-evolution model;
- atomic candidate validation: a failed candidate publishes no revision;
- revision-bound architecture snapshots and deltas for the implemented metrics
  and policy rules; and
- three-file executable composed and bounded-cancellation services under
  `compiler/examples/`.

The AIL job-service runner passes all 37 public cases. The architecture checker
accepts the domain-owned `CancelJob` change and rejects both the centralized and
helper-split versions that move store authority into transport.

## Hard limits

AIL is not ready for production application development. Sequential map over an
immutable bounded list is its only repeated-work form. It has no general loops,
collection library, mutation, concurrency, networking, package registry,
foreign-function system, production runtime, native-code backend, or deployment
toolchain. Recursion is rejected rather than bounded or executed. The
interpreter is a semantic test engine, not a production runtime.

The broader designs for memory, concurrency, replay, resources, packages, and
foreign code remain unresolved. The implemented architecture API covers the
M24 metric and policy set, not the full catalog in
[docs/architecture-health.md](docs/architecture-health.md).

## Why build AIL

Generating plausible code is cheap. Finding the right context, understanding
effects and downstream consequences, validating a complete change, repairing
failures, and preventing regressions consume most of an agent's work.

AIL moves those costs into language rules and compiler operations:

- one canonical source representation cuts irrelevant variation;
- explicit public contracts expose what callers depend on;
- capabilities expose authority and external effects;
- deterministic execution makes failures reproducible;
- semantic queries replace repeated reconstruction from raw files; and
- atomic validation prevents partial multi-file changes from becoming revisions.

The project must eventually compare this workflow with Rust, Go, Python, and
TypeScript using their normal compilers and language servers. It has not yet run
that comparison. Source brevity, feature count, compiler size, LLVM integration,
and self-hosting do not answer the question.

Read [the project intent](docs/project-intent.md),
[current compiler design](docs/design-direction.md), and
[current status](docs/STATUS.md) next.

## Build and test

From the repository root:

```bash
cargo +1.87.0 fmt --all --check
cargo +1.87.0 test --workspace
cargo +1.87.0 clippy --workspace --all-targets -- -D warnings
python3 tools/check_docs.py
```

The repository also contains locked language-independent fixtures, baseline
implementations, specification checkers, and architecture-policy tests. See
[compiler/README.md](compiler/README.md) for the Rust APIs and focused commands.

## Repository map

```text
compiler/      Rust compiler, semantic APIs, interpreter, and examples
specs/         Numbered rules, protocol shapes, fixtures, and contract checkers
benchmarks/    Job-service cases, baseline implementations, and harnesses
docs/          Product intent, requirements, design, decisions, and status
tools/         Repository checks
```

The numbered rules and conformance fixtures under `specs/` define required
behavior where they apply. Examples explain behavior but do not create new
language rules.

The repository does not yet have a license. All rights remain with the copyright
holder until one is added.
