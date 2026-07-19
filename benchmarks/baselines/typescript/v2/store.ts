/** Deterministic jobs capability used by the benchmark. */

import { InsertOutcome, Priority, type Job, type JobStore } from "./domain.js";

export const RecordVersion = {
  v1: 1,
  v2: 2,
} as const;
export type RecordVersion = (typeof RecordVersion)[keyof typeof RecordVersion];

export type StoredJob = Readonly<{
  recordVersion: RecordVersion;
  jobId: string;
  task: string;
  payload: Uint8Array;
  priority: Priority | undefined;
}>;

export type StoreCall = Readonly<{ job: StoredJob }>;

export function adaptStoredJobToV2(stored: StoredJob): Job {
  const priority =
    stored.recordVersion === RecordVersion.v2 && stored.priority !== undefined
      ? stored.priority
      : Priority.normal;
  return {
    jobId: stored.jobId,
    task: stored.task,
    payload: Uint8Array.from(stored.payload),
    priority,
  };
}

export class DeterministicJobStore implements JobStore {
  readonly #jobs: StoredJob[];
  readonly #calls: StoreCall[] = [];

  constructor(
    jobs: readonly StoredJob[],
    readonly outcome: InsertOutcome,
    readonly insertVersion: RecordVersion,
  ) {
    this.#jobs = jobs.map(cloneStoredJob);
  }

  get jobs(): readonly StoredJob[] {
    return this.#jobs.map(cloneStoredJob);
  }

  get calls(): readonly StoreCall[] {
    return this.#calls.map(({ job }) => ({ job: cloneStoredJob(job) }));
  }

  insertIfAbsent(job: Job): InsertOutcome {
    const stored: StoredJob = {
      recordVersion: this.insertVersion,
      jobId: job.jobId,
      task: job.task,
      payload: Uint8Array.from(job.payload),
      priority:
        this.insertVersion === RecordVersion.v2 ? job.priority : undefined,
    };
    this.#calls.push({ job: cloneStoredJob(stored) });
    if (this.outcome === InsertOutcome.inserted) {
      if (this.#jobs.some((current) => current.jobId === job.jobId)) {
        throw new Error(
          `inserted outcome violates insert-if-absent postcondition for ${JSON.stringify(job.jobId)}`,
        );
      }
      this.#jobs.push(stored);
    }
    return this.outcome;
  }
}

function cloneStoredJob(job: StoredJob): StoredJob {
  return { ...job, payload: Uint8Array.from(job.payload) };
}
