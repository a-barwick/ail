# Language-neutral implementation runner contract

Status: **Frozen by M2**

This contract lets the benchmark harness execute equivalent implementations
without knowing their language, build system, or internal data model. A
baseline provides a canonical JSON runner descriptor that conforms to
`benchmarks/schemas/runner-descriptor.schema.json`.

The descriptor contains an argument-vector command. The harness invokes it
directly without a shell, from the declared repository-relative working
directory.

## One-case command

```text
<command> --case <repository-relative-fixture-path>
```

The runner must:

1. read exactly the supplied public or harness-private fixture;
2. execute the fixture's declared operation once;
3. write exactly one UTF-8 JSON value to standard output;
4. use `result_format: 1` and the single-case shape in
   `benchmarks/schemas/runner-result.schema.json`;
5. place the observable response or decode result, final stored state, and
   ordered store calls under `actual`; and
6. exit zero only after producing the complete result.

Diagnostic text belongs on standard error. The harness records it but does not
parse prose to determine correctness.

## Corpus command

```text
<command> --corpus <repository-relative-fixture-manifest-path>
```

The runner must execute every manifest entry exactly once in manifest order and
write one corpus result. The result carries the SHA-256 digest of the exact
fixture manifest it consumed and one single-case result per manifest entry, in
the same order.

The corpus command may load fixtures incrementally, but it must not omit,
duplicate, or reorder results. Fixture file I/O and process startup are excluded
from later warm handler measurements; this command is the functional
correctness interface.

## Process behavior

- Standard input is closed.
- The runner must not install dependencies or use the network.
- Paths supplied by the harness are repository-relative and must not be
  rewritten to infer hidden package locations.
- A signal exit, non-zero exit, timeout, extra standard-output text, malformed
  JSON, schema-invalid result, missing case, extra case, or observable mismatch
  is a failed verification.
- The runner must not return benchmark timing as part of functional `actual`
  data. Measurement output is captured separately by later milestones.

The runner contract exposes accepted application behavior only. It does not
prescribe language APIs, source layout, build tools, or an AIL compiler
transport.
