import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { describe, it } from "node:test";

import { FixtureError, runCase } from "../v2/fixture.js";
import { repositoryRoot } from "./support.js";

async function load(path: string): Promise<unknown> {
  return JSON.parse(await readFile(path, "utf8")) as unknown;
}

function validFixture(): Record<string, unknown> {
  return {
    fixture_format: 1,
    case_id: "test",
    service_version: 2,
    operation: "create_job",
    request: {
      api_version: 2,
      job_id: "job-1",
      task: "task",
      payload_base64: "",
      priority: "high",
    },
    initial_jobs: [],
    store_outcome: "inserted",
  };
}

describe("fixture adapter", () => {
  it("matches all 37 shared public oracles", async () => {
    const manifest = (await load(
      join(repositoryRoot, "benchmarks/fixtures/manifest.json"),
    )) as { fixtures: readonly { path: string }[] };
    assert.equal(manifest.fixtures.length, 37);
    for (const entry of manifest.fixtures) {
      const fixture = (await load(join(repositoryRoot, entry.path))) as Record<
        string,
        unknown
      >;
      const result = runCase(fixture);
      assert.equal(result.case_id, fixture.case_id);
      assert.equal(result.operation, fixture.operation);
      assert.deepEqual(result.actual, fixture.expected, entry.path);
    }
  });

  for (const [change, message] of [
    [{ operation: "remove_job" }, /unsupported operation/u],
    [{ service_version: 3 }, /unsupported service or record version/u],
    [{ store_outcome: "maybe" }, /unsupported store outcome/u],
    [{ initial_jobs: "not-array" }, /must be an array/u],
  ] as const) {
    it(`rejects ${JSON.stringify(change)}`, () => {
      assert.throws(() => runCase({ ...validFixture(), ...change }), message);
    });
  }

  it("rejects malformed Base64", () => {
    const fixture = validFixture();
    fixture.request = {
      ...(fixture.request as Record<string, unknown>),
      payload_base64: "***",
    };
    assert.throws(() => runCase(fixture), /Base64/u);
  });

  it("rejects V2 requests at a V1 service", () => {
    assert.throws(
      () => runCase({ ...validFixture(), service_version: 1 }),
      /accepts only API version 1/u,
    );
  });

  it("rejects unsupported request versions", () => {
    const fixture = validFixture();
    fixture.request = {
      ...(fixture.request as Record<string, unknown>),
      api_version: 3,
    };
    assert.throws(() => runCase(fixture), /unsupported request API version/u);
  });

  for (const [storedJob, message] of [
    [
      {
        record_version: 2,
        job_id: "job-1",
        task: "task",
        payload_base64: "",
      },
      /missing priority/u,
    ],
    [
      {
        record_version: 2,
        job_id: "job-1",
        task: "task",
        payload_base64: "",
        priority: "urgent",
      },
      /unknown priority/u,
    ],
  ] as const) {
    it(`rejects invalid stored record ${JSON.stringify(storedJob)}`, () => {
      assert.throws(
        () =>
          runCase({
            case_id: "bad-stored",
            operation: "decode_stored_job",
            stored_job: storedJob,
          }),
        message,
      );
    });
  }

  for (const fixture of [
    [],
    { operation: 2 },
    {
      operation: "decode_stored_job",
      case_id: "bad",
      stored_job: { record_version: true },
    },
  ]) {
    it(`checks boundary type ${JSON.stringify(fixture)}`, () => {
      assert.throws(() => runCase(fixture), FixtureError);
    });
  }

  it("returns unknown priority as a zero-effect decode result", () => {
    const fixture = validFixture();
    fixture.request = {
      ...(fixture.request as Record<string, unknown>),
      priority: "urgent",
    };
    const result = runCase(fixture);
    assert.deepEqual(result.actual, {
      decode_error: {
        code: "unknown_priority",
        field: "priority",
        value: "urgent",
      },
      final_jobs: [],
      store_calls: [],
    });
  });
});
