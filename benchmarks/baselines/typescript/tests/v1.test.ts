import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  InsertOutcome,
  createJob,
  type CreateJobRequest,
  type Job,
  type JobStore,
} from "../v1/job-service.js";

class RecordingStore implements JobStore {
  readonly calls: Job[] = [];

  constructor(readonly outcome: InsertOutcome) {}

  insertIfAbsent(job: Job): InsertOutcome {
    this.calls.push(job);
    return this.outcome;
  }
}

function request(changes: Partial<CreateJobRequest> = {}): CreateJobRequest {
  return {
    jobId: "job-1042",
    task: "rebuild-search-index",
    payload: Buffer.from('{"tenant":"north"}'),
    ...changes,
  };
}

describe("version-one checkpoint", () => {
  it("inserts one exact job", () => {
    const store = new RecordingStore(InsertOutcome.inserted);
    const value = request();
    const result = createJob(value, store);
    assert.deepEqual(result, {
      kind: "created",
      job: {
        jobId: value.jobId,
        task: value.task,
        payload: Uint8Array.from(value.payload),
      },
    });
    assert.equal(store.calls.length, 1);
  });

  it("collects one issue per field in contract order without effects", () => {
    const store = new RecordingStore(InsertOutcome.inserted);
    const result = createJob(
      request({ jobId: "", task: "", payload: new Uint8Array(4097) }),
      store,
    );
    assert.deepEqual(result, {
      kind: "invalid",
      issues: [
        { field: "job_id", reason: "missing" },
        { field: "task", reason: "missing" },
        { field: "payload", reason: "payload_too_large" },
      ],
    });
    assert.deepEqual(store.calls, []);
  });

  for (const [jobId, valid] of [
    ["a", true],
    ["A0", true],
    [`a${"_".repeat(63)}`, true],
    ["", false],
    [`_${"a".repeat(63)}`, false],
    ["a".repeat(65), false],
    ["job.dot", false],
    ["jób", false],
  ] as const) {
    it(`classifies job id ${JSON.stringify(jobId)}`, () => {
      const result = createJob(
        request({ jobId }),
        new RecordingStore(InsertOutcome.inserted),
      );
      assert.equal(result.kind === "created", valid);
    });
  }

  for (const [task, reason] of [
    ["", "missing"],
    ["界".repeat(80), undefined],
    ["界".repeat(81), "too_long"],
    ["line\nbreak", "control_character"],
  ] as const) {
    it(`validates task case ${String(reason)}`, () => {
      const result = createJob(
        request({ task }),
        new RecordingStore(InsertOutcome.inserted),
      );
      if (reason === undefined) {
        assert.equal(result.kind, "created");
      } else {
        assert.deepEqual(result, {
          kind: "invalid",
          issues: [{ field: "task", reason }],
        });
      }
    });
  }

  for (const size of [0, 4096, 4097]) {
    it(`validates ${size} payload bytes`, () => {
      const store = new RecordingStore(InsertOutcome.inserted);
      const result = createJob(
        request({ payload: new Uint8Array(size) }),
        store,
      );
      assert.equal(result.kind === "created", size <= 4096);
      assert.equal(store.calls.length, size <= 4096 ? 1 : 0);
    });
  }

  for (const [outcome, expected] of [
    [InsertOutcome.duplicate, { kind: "already_exists", jobId: "job-1042" }],
    [
      InsertOutcome.unavailableBeforeCommit,
      { kind: "persistence_unavailable" },
    ],
  ] as const) {
    it(`maps closed outcome ${outcome} after one call`, () => {
      const store = new RecordingStore(outcome);
      assert.deepEqual(createJob(request(), store), expected);
      assert.equal(store.calls.length, 1);
    });
  }

  it("treats an out-of-contract store outcome as a fault", () => {
    const invalidStore: JobStore = {
      insertIfAbsent: () => "invalid" as InsertOutcome,
    };
    assert.throws(
      () => createJob(request(), invalidStore),
      /invalid insert outcome/u,
    );
  });
});
