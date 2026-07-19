/** Transport-independent create-job handler. */

import {
  InsertOutcome,
  type CreateJobRequest,
  type CreateJobResult,
  type Job,
  type JobStore,
  type Priority,
  type ValidationIssue,
} from "./domain.js";

const jobIdPattern = /^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$/u;
const controlCharacterPattern = /\p{Cc}/u;

export function createJob(
  request: CreateJobRequest,
  store: JobStore,
): CreateJobResult {
  const { priority, issues } = validate(request);
  if (issues.length > 0) {
    return { kind: "invalid", issues };
  }
  if (priority === undefined) {
    throw new Error("validated request must have a priority");
  }

  const job: Job = {
    jobId: request.jobId,
    task: request.task,
    payload: Uint8Array.from(request.payload),
    priority,
  };
  const outcome = store.insertIfAbsent(job);
  switch (outcome) {
    case InsertOutcome.inserted:
      return { kind: "created", job };
    case InsertOutcome.duplicate:
      return { kind: "already_exists", jobId: job.jobId };
    case InsertOutcome.unavailableBeforeCommit:
      return { kind: "persistence_unavailable" };
    default:
      return assertNever(outcome);
  }
}

function validate(request: CreateJobRequest): Readonly<{
  priority: Priority | undefined;
  issues: readonly ValidationIssue[];
}> {
  const issues: ValidationIssue[] = [];
  if (request.jobId.length === 0) {
    issues.push({ field: "job_id", reason: "missing" });
  } else if (!jobIdPattern.test(request.jobId)) {
    issues.push({ field: "job_id", reason: "invalid_format" });
  }

  if (request.task.length === 0) {
    issues.push({ field: "task", reason: "missing" });
    // Array.from counts Unicode code points, which are the fixture's scalar unit.
  } else if (Array.from(request.task).length > 80) {
    issues.push({ field: "task", reason: "too_long" });
  } else if (controlCharacterPattern.test(request.task)) {
    issues.push({ field: "task", reason: "control_character" });
  }

  if (request.payload.byteLength > 4096) {
    issues.push({ field: "payload", reason: "payload_too_large" });
  }
  if (request.priority === undefined) {
    issues.push({ field: "priority", reason: "missing" });
  }
  return { priority: request.priority, issues };
}

function assertNever(value: never): never {
  throw new Error(`invalid insert outcome: ${String(value)}`);
}
