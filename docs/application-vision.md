# What AIL should build

AIL targets greenfield software primarily built and maintained by agents:
backend services, APIs, workers, command-line tools, scheduled jobs, and data
processing applications. Backend services and workers are the first proving
ground, not the permanent limit.

## Target system

AIL should combine:

- predictable execution and resource behavior;
- strong static checks and no undefined behavior in ordinary code;
- compact local implementations with explicit public contracts;
- compiler-visible types, effects, authority, state, and dependencies;
- deterministic tests driven by supplied external inputs; and
- structural operations that validate the complete change before publication.

The compiler should solve mechanical problems and expose the result. Source
should not repeat facts the compiler can determine, but it must show anything
another module depends on.

## Agent operating model

An agent working on a large AIL program should ask the compiler for the contract
of a symbol, callers, affected schemas, inferred effects, capability paths,
architecture constraints, diagnostics, and validated edits. It should not have
to reconstruct those facts by repeatedly loading raw files.

Canonical text remains the version-controlled program. Compiler queries make
that text cheaper and safer to change. Humans use the same source and compiler
facts to review the result.

## First workload

The current reference system is a transport-independent job service:

```text
ordered request
  + explicit jobs-store capability
  -> closed result + final state + ordered store calls
```

It tests bounded request validation, closed domain outcomes, conditional state
change, schema evolution, exact impact analysis, revision-safe edits, and
architecture enforcement. Rust, Go, Python, and TypeScript implementations use
the same language-independent fixtures.

The next useful workloads should add real pressure one capability at a time:
time, outbound calls, retries, cancellation, bounded parallel work, and repeated
schema changes. Each addition needs exact observable behavior and tests before
it needs syntax.

## Current gap

The compiler can run the job service and a three-module composed example. It
cannot yet express a normal production service. Missing fundamentals include
iteration, general collections, concurrency, networking, package management,
foreign interfaces, production memory and resource semantics, native execution,
and deployment.

These are technical gaps, not a checklist to implement blindly. The next change
should close the smallest gap required by a real executable workload and prove
it with deterministic tests.

## Evaluation

Compare AIL with strong mainstream tools on complete changes, including:

- context delivered to the agent;
- semantic queries and source reads;
- first-pass compile and test results;
- repair iterations and failure causes;
- missed downstream changes and regressions;
- textual and semantic diff size;
- elapsed agent work;
- runtime latency, throughput, startup, and memory when a production runtime
  exists; and
- evidence a human needs to approve the change.

Record model and agent versions, tools, task text, supplied context, retries,
correctness checks, and environment. A fast result that fails behavior or cannot
be audited is a failed result.

## Outside the current target

The current implementation does not target kernels, hard real-time control,
device drivers, browser or mobile UI ecosystems, arbitrary legacy foreign code,
or unrestricted build scripts. Supporting those systems would require explicit
memory, concurrency, platform, and authority semantics that do not exist today.
