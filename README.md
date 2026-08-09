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
bounded lists and deterministic sequential map. M30 added one cooperative,
revision-bound outbound dependency request. M31 added one fixed-limit outbound
map. M32 added a separate revision-pinned HTTP host for the canonical batch
lookup.

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
- explicit outbound capability metadata, bounded millisecond timeout, opaque
  cancellation, and closed timeout/cancel results;
- one direct fixed-limit outbound map with input-aligned results and accurate
  start/completion traces;
- one strict `POST /v1/lookups:batch` Rust host pinned to compiler revision,
  source, settings, function, type, bound, concurrency, and permission facts;
- rejection of recursive call cycles and import cycles;
- canonical formatting and structured parse, type, import, and effect errors;
- immutable revisions, revision-scoped handles, inspected semantic facts,
  validated rename, and identity maps;
- complete impact results for the implemented schema-evolution model;
- atomic candidate validation: a failed candidate publishes no revision;
- revision-bound architecture snapshots and deltas for the implemented metrics
  and policy rules; and
- executable composed, bounded-cancellation, and outbound-request services under
  `compiler/examples/`.

The AIL job-service runner passes all 37 public cases. The architecture checker
accepts the domain-owned `CancelJob` change and rejects both the centralized and
helper-split versions that move store authority into transport.

## Hard limits

AIL is not ready for general production application development. Repeated work
is limited to sequential map and one fixed-limit outbound map over immutable
bounded lists. It has no general loops, collection library, mutation,
unrestricted concurrency, general networking or routing, package registry,
foreign-function system, production runtime, native-code backend, or deployment
toolchain. The M32 host is one fixed adapter without TLS or authentication.
Recursion is rejected rather than bounded or executed. The interpreter is a
semantic test engine, not a production runtime. Its outbound provider is
synchronous and cooperative rather than a hard-preemptive network runtime.

The broader designs for memory, concurrency, replay, resources, packages, and
foreign code remain unresolved. The implemented architecture API covers the
M24 metric and policy set.

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

Read [the product intent](docs/product.md),
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
service-host/  Revision-pinned M32 batch-lookup HTTP host
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
