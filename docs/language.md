# AIL language

AIL is a small typed language. A program is canonical source. The compiler
parses it, checks it, stores immutable revisions, and can execute the supported
fragment.

Exact rules live in [specs/](../specs/README.md). Examples live in
[compiler/examples/](../compiler/examples/). [STATUS.md](STATUS.md) lists what
this language still cannot do.

## Programs

Each file is one module. Modules import each other by name or alias. Functions
take explicit arguments, including capabilities. There is no ambient I/O: time,
randomness, storage, network, filesystem, and environment arrive only as
capability parameters.

```ail
module domain;

record Request {
  name: Text;
}

variant Response {
  Accepted(Text);
  Rejected;
}

fn domain_name(request: Request) -> Text {
  request.name
}
```

Records, closed variants, `let`, `if`, exhaustive `match`, field access, and
ordinary function calls are supported. Arguments are checked exactly and
evaluated left to right. Capability effects propagate through calls. Direct and
mutual recursion are rejected. Import cycles, inaccessible names, and ambiguous
imports are rejected.

## Lists

`List<T, N>` is an immutable ordered list with a compile-time maximum length.
`map item in items { ... }` evaluates the source once, then the body once per
stored element, in order, producing an aligned result with the same bound.

External list arguments are fully validated before capability checks or calls.
There are no list literals, indexing, mutation, filter, fold, nested lists, or
general loops.

## Outbound work

An outbound dependency is an ordinary capability call. The host marks timeout
and cancellation argument positions and the closed timeout/cancel results. The
interpreter depends on a cooperative provider; it cannot preempt stuck host
code or roll back remote effects.

```ail
fn lookup_batch(requests: List<types.LookupRequest, 8>, timeout: Int, cancellation: Cancellation, dependency: capability DependencyClient) -> List<types.LookupOutcome, 8> effects { dependency.fetch } {
  parallel map item in requests limit 3 {
    dependency.fetch(item, timeout, cancellation)
  }
}
```

`parallel map ... limit C` runs that one outbound operation over a bounded
list, keeps at most `C` calls in flight, and stores results in input order.
This is not general networking, threads, retries, or async.

## Compiler services

The compiler formats source canonically, checks names, types, and effects, and
stores immutable revisions with inspectable semantic facts. The
`EvolutionWorkspace` API can report schema impact and validate a multi-file
change as one transaction.

`ailc check` builds an `EvolutionWorkspace` from a directory of `.ail` files or
one file, then runs the compiler checks and any project architecture policy. It
writes no revision. `ailc publish` accepts a directory workspace and writes a
revision only after the same checks pass. A failed candidate publishes nothing.

Source and architecture diagnostics can produce structured findings. Every
finding has a code and category. It includes a file and source range,
expected and actual facts, related locations, and a constraint only when the
checker computed those fields. A finding does not name an edit. Input errors do
not produce findings: examples include a bad path, a directory with no valid
`.ail` files, and `ailc publish` given a file. One failed command can therefore
report zero or more findings. `ailc check --json` and `ailc publish --json`
render the available findings in one JSON document. See
[0016-structured-check-findings.md](decisions/0016-structured-check-findings.md).

The interpreter executes a checked entry point with caller-supplied
capabilities.

## Host

A separate Rust program serves only `POST /v1/lookups:batch` for one pinned
batch-lookup revision. It loads an immutable JSON catalog at startup and binds
loopback only. See [pinned-http-batch-lookup.md](../specs/pinned-http-batch-lookup.md)
and [private-catalog-dogfood.md](../specs/private-catalog-dogfood.md).
