/** Version-one UC-001 checkpoint. */

export type CreateJobRequest = Readonly<{
  jobId: string;
  task: string;
  payload: Uint8Array;
}>;

export type Job = Readonly<{
  jobId: string;
  task: string;
  payload: Uint8Array;
}>;

export type ValidationField = "job_id" | "task" | "payload";
export type ValidationReason =
  | "missing"
  | "invalid_format"
  | "too_long"
  | "control_character"
  | "payload_too_large";

export type ValidationIssue = Readonly<{
  field: ValidationField;
  reason: ValidationReason;
}>;

export type CreateJobResult =
  | Readonly<{ kind: "created"; job: Job }>
  | Readonly<{ kind: "invalid"; issues: readonly ValidationIssue[] }>
  | Readonly<{ kind: "already_exists"; jobId: string }>
  | Readonly<{ kind: "persistence_unavailable" }>;

export const InsertOutcome = {
  inserted: "inserted",
  duplicate: "duplicate",
  unavailableBeforeCommit: "unavailable_before_commit",
} as const;

export type InsertOutcome = (typeof InsertOutcome)[keyof typeof InsertOutcome];

export type JobStore = {
  /** The only external capability available to createJob. */
  insertIfAbsent(job: Job): InsertOutcome;
};

const jobIdPattern = /^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$/u;
const controlCharacterPattern = /\p{Cc}/u;

export function createJob(
  request: CreateJobRequest,
  store: JobStore,
): CreateJobResult {
  const issues = validate(request);
  if (issues.length > 0) {
    return { kind: "invalid", issues };
  }

  const job: Job = {
    jobId: request.jobId,
    task: request.task,
    payload: Uint8Array.from(request.payload),
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

function validate(request: CreateJobRequest): readonly ValidationIssue[] {
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
  return issues;
}

function assertNever(value: never): never {
  throw new Error(`invalid insert outcome: ${String(value)}`);
}
