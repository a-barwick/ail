# Job-service JSON fixture format

Status: **Accepted benchmark format**

The Rust, Go, Python, TypeScript, and AIL implementations use the same JSON test
cases. JSON is used because every baseline supports it without extra tooling and
because engineers can review fixture changes directly.

These files describe expected service behavior. They are not AIL source and do
not define AIL syntax.

## File rules

- Store one test case per `.json` file.
- Encode the file as UTF-8 with Unix line endings and a trailing newline.
- Use two-space indentation.
- Follow the field order shown below so diffs stay consistent.
- Use JSON integers only for format and schema version numbers.
- Encode payload bytes using standard padded Base64.
- Carry request, response, and stored-record versions explicitly.
- Do not infer a version from the presence or absence of a field.
- Compare parsed values, not JSON object-key order.

The fixture format starts at version 1. A change to the meaning of an existing
fixture field requires a new fixture-format version.

## Create-job case

Each create-job case contains:

| Field | Purpose |
| --- | --- |
| `fixture_format` | Version of this test-file format |
| `case_id` | Stable name used in results |
| `service_version` | Service behavior before or after the priority change |
| `operation` | `create_job` |
| `request` | Raw versioned request data supplied to the service boundary |
| `initial_jobs` | Stored records present before the call |
| `store_outcome` | Simulated insert result when a store call is expected |
| `expected` | Response or decode error, final jobs, and ordered store calls |

Omit `store_outcome` when the expected behavior makes no storage call.

### Behavior illustration: successful version-one case

```json
{
  "fixture_format": 1,
  "case_id": "uc001-v1-created",
  "service_version": 1,
  "operation": "create_job",
  "request": {
    "api_version": 1,
    "job_id": "job-1042",
    "task": "rebuild-search-index",
    "payload_base64": "eyJ0ZW5hbnQiOiJub3J0aCJ9"
  },
  "initial_jobs": [],
  "store_outcome": "inserted",
  "expected": {
    "response": {
      "api_version": 1,
      "result": {
        "kind": "created",
        "job": {
          "job_id": "job-1042",
          "task": "rebuild-search-index",
          "payload_base64": "eyJ0ZW5hbnQiOiJub3J0aCJ9"
        }
      }
    },
    "final_jobs": [
      {
        "record_version": 1,
        "job_id": "job-1042",
        "task": "rebuild-search-index",
        "payload_base64": "eyJ0ZW5hbnQiOiJub3J0aCJ9"
      }
    ],
    "store_calls": [
      {
        "operation": "insert_if_absent",
        "job": {
          "record_version": 1,
          "job_id": "job-1042",
          "task": "rebuild-search-index",
          "payload_base64": "eyJ0ZW5hbnQiOiJub3J0aCJ9"
        }
      }
    ]
  }
}
```

## Version-two priority

Version-two requests and stored records use the field `priority` with exactly
one of these JSON strings:

| Service value | JSON value |
| --- | --- |
| Low | `"low"` |
| Normal | `"normal"` |
| High | `"high"` |

A version-two request must contain `priority`. A version-one request is converted
to `normal` internally, produces a version-two stored record, and receives a
version-one response without a priority field.

An unknown priority string produces a request decode error and no storage call.
A missing version-two priority produces the accepted invalid-request result and
no storage call.

## Stored-record conversion case

UC-003 also needs cases that read old stored records without calling the
create-job handler. These use:

```text
operation: decode_stored_job
```

The case supplies one version-one stored record. The expected result is the
version-two in-memory job with `priority` set to `normal`. This tests old-data
compatibility separately from request handling.

## Required public cases

The checked-in public corpus must include at least:

### UC-001

- successful creation with an empty payload;
- successful creation with valid non-ASCII task text;
- every `job_id` boundary and format failure;
- every `task` boundary and control-character failure;
- payload sizes of 0, 4096, and 4097 bytes;
- multiple invalid fields with the expected issue order;
- duplicate job ID; and
- storage unavailable before commit.

### UC-003

- successful V2 creation for low, normal, and high priority;
- missing V2 priority;
- unknown V2 priority;
- V1 request converted to normal priority and returned as a V1 response;
- V1 stored record converted to a V2 in-memory job;
- duplicate and storage-unavailable results for V1 and V2 requests; and
- the complete UC-001 suite as regression coverage.

Hidden cases may add combinations and seeded consumers, but may not introduce
behavior absent from the accepted use cases.

## Impact-report case

The priority-change benchmark also freezes the expected impact-report shape:

```json
{
  "report_format": 1,
  "source_revision": "R1",
  "must_change": [
    {
      "location_id": "stable-location-id",
      "reason": "constructs Job and must provide priority"
    }
  ],
  "review": [],
  "unchecked": [
    {
      "boundary_id": "external-client-contract",
      "reason": "client source is outside the current build"
    }
  ],
  "confirmed_unchanged": [
    "jobs-store authority",
    "store-call count",
    "store-call order"
  ]
}
```

The baseline repositories will replace the example location with the exact
seeded source locations before agent runs. The hidden oracle checks that
`must_change` is exact and that `review` contains at most two unnecessary
entries.

## Validation

Before baseline implementation begins, the fixture corpus must have:

- a machine-readable schema or validator;
- a formatter that produces the repository's one accepted JSON layout;
- a manifest listing every case and its SHA-256 digest;
- a language-independent reference check for Base64 and version fields; and
- review confirming that every expected behavior comes from UC-001 or UC-003.

Those tools validate benchmark data only. They do not become part of the AIL
compiler or language specification.
