# Benchmark artifacts

This directory contains executable, language-independent inputs for the
job-service benchmark. The artifacts describe accepted application behavior;
they are not AIL source and do not define AIL syntax.

## Public fixture corpus

`fixtures/public/` contains one JSON case per file. The cases exercise the
accepted UC-001 request-validation and persistence behavior and the UC-003
priority evolution, including the complete UC-001 regression matrix against
service version 2.

`fixtures/manifest.json` records every public fixture and its SHA-256 digest,
traceability, and coverage tags. `schemas/job-service-fixture.schema.json`
defines the machine-readable file shape.

Run the complete fixture gate with:

```bash
python3 benchmarks/tools/fixtures.py check
```

The command validates:

- JSON schema and canonical two-space formatting;
- explicit request, response, and stored-record versions;
- canonical padded Base64 payloads;
- request bounds and validation-issue ordering;
- response, final state, and ordered store calls;
- required UC-001 and UC-003 public-case coverage; and
- the schema and fixture digests in the manifest.

To format fixture files or check the manifest independently:

```bash
python3 benchmarks/tools/fixtures.py format
python3 benchmarks/tools/fixtures.py manifest --check
```

`format` intentionally does not rewrite the manifest because formatting a case
changes its digest. After an intentional reviewed fixture change, regenerate
the manifest with `manifest --write`, review the resulting traceability, and
run the complete check.

## Public and hidden boundary

Only public behavior cases belong under `fixtures/public/`. Later milestones
may package hidden combinations and seeded language-specific consumers outside
that tree. Hidden inputs may exercise only behavior already accepted by UC-001
or UC-003, and their answers must not be disclosed by this public corpus or its
manifest.
