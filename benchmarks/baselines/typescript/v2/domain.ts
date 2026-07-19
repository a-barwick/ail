/** Public contracts for the evolved job service. */

export const ApiVersion = {
  v1: 1,
  v2: 2,
} as const;
export type ApiVersion = (typeof ApiVersion)[keyof typeof ApiVersion];

export const Priority = {
  low: "low",
  normal: "normal",
  high: "high",
} as const;
export type Priority = (typeof Priority)[keyof typeof Priority];

export function parsePriority(value: string): Priority | undefined {
  return Object.values(Priority).find((priority) => priority === value);
}

export type CreateJobRequest = Readonly<{
  jobId: string;
  task: string;
  payload: Uint8Array;
  priority: Priority | undefined;
}>;

export type Job = Readonly<{
  jobId: string;
  task: string;
  payload: Uint8Array;
  priority: Priority;
}>;

export type ValidationField = "job_id" | "task" | "payload" | "priority";
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
  /** The sole external capability available to the handler. */
  insertIfAbsent(job: Job): InsertOutcome;
};
