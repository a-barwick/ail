import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  InsertOutcome,
  Priority,
  type CreateJobRequest,
  type Job,
} from "../v2/domain.js";
import { createJob } from "../v2/service.js";
import {
  DeterministicJobStore,
  RecordVersion,
  adaptStoredJobToV2,
  type StoredJob,
} from "../v2/store.js";

function request(changes: Partial<CreateJobRequest> = {}): CreateJobRequest {
  return {
    jobId: "job-1042",
    task: "rebuild-search-index",
    payload: Buffer.from('{"tenant":"north"}'),
    priority: Priority.high,
    ...changes,
  };
}

function store(
  outcome: InsertOutcome = InsertOutcome.inserted,
): DeterministicJobStore {
  return new DeterministicJobStore([], outcome, RecordVersion.v2);
}

describe("evolved service", () => {
  for (const priority of Object.values(Priority)) {
    it(`propagates ${priority} unchanged`, () => {
      const jobs = store();
      const value = request({ priority });
      assert.deepEqual(createJob(value, jobs), {
        kind: "created",
        job: {
          jobId: value.jobId,
          task: value.task,
          payload: Uint8Array.from(value.payload),
          priority,
        },
      });
      assert.equal(jobs.jobs[0]?.priority, priority);
      assert.equal(jobs.calls[0]?.job.priority, priority);
    });
  }

  it("orders missing priority after every inherited issue", () => {
    const jobs = store();
    const result = createJob(
      request({
        jobId: "",
        task: "",
        payload: new Uint8Array(4097),
        priority: undefined,
      }),
      jobs,
    );
    assert.deepEqual(result, {
      kind: "invalid",
      issues: [
        { field: "job_id", reason: "missing" },
        { field: "task", reason: "missing" },
        { field: "payload", reason: "payload_too_large" },
        { field: "priority", reason: "missing" },
      ],
    });
    assert.deepEqual(jobs.calls, []);
    assert.deepEqual(jobs.jobs, []);
  });

  for (const [outcome, expected] of [
    [InsertOutcome.duplicate, { kind: "already_exists", jobId: "job-1042" }],
    [
      InsertOutcome.unavailableBeforeCommit,
      { kind: "persistence_unavailable" },
    ],
  ] as const) {
    it(`keeps state for ${outcome}`, () => {
      const jobs = store(outcome);
      assert.deepEqual(createJob(request(), jobs), expected);
      assert.equal(jobs.calls.length, 1);
      assert.deepEqual(jobs.jobs, []);
    });
  }

  it("treats an out-of-contract store outcome as a fault", () => {
    assert.throws(
      () =>
        createJob(request(), {
          insertIfAbsent: () => "invalid" as InsertOutcome,
        }),
      /invalid insert outcome/u,
    );
  });
});

describe("deterministic store", () => {
  const v1: StoredJob = {
    recordVersion: RecordVersion.v1,
    jobId: "legacy",
    task: "old",
    payload: Buffer.from("legacy"),
    priority: undefined,
  };

  it("adapts V1 records explicitly to normal", () => {
    assert.deepEqual(adaptStoredJobToV2(v1), {
      jobId: "legacy",
      task: "old",
      payload: Uint8Array.from(Buffer.from("legacy")),
      priority: Priority.normal,
    });
  });

  it("preserves V2 values and returns defensive snapshots", () => {
    const jobs = store();
    const job: Job = {
      jobId: "job-1",
      task: "task",
      payload: Buffer.from("payload"),
      priority: Priority.low,
    };
    assert.equal(jobs.insertIfAbsent(job), InsertOutcome.inserted);
    const first = jobs.jobs[0]!;
    first.payload[0] = 0;
    assert.equal(Buffer.from(jobs.jobs[0]!.payload).toString(), "payload");
    assert.equal(jobs.calls[0]!.job.priority, Priority.low);
  });

  it("rejects an inserted outcome that violates insert-if-absent", () => {
    const jobs = new DeterministicJobStore(
      [v1],
      InsertOutcome.inserted,
      RecordVersion.v2,
    );
    assert.throws(
      () =>
        jobs.insertIfAbsent({
          jobId: "legacy",
          task: "new",
          payload: new Uint8Array(),
          priority: Priority.high,
        }),
      /postcondition/u,
    );
    assert.equal(jobs.calls.length, 1);
    assert.deepEqual(jobs.jobs, [
      { ...v1, payload: Uint8Array.from(v1.payload) },
    ]);
  });
});
