/** Frozen JSON fixture adapters for the typed service boundary. */

import {
  ApiVersion,
  InsertOutcome,
  Priority,
  parsePriority,
  type CreateJobRequest,
  type CreateJobResult,
  type Job,
} from "./domain.js";
import { createJob } from "./service.js";
import {
  DeterministicJobStore,
  RecordVersion,
  adaptStoredJobToV2,
  type RecordVersion as RecordVersionType,
  type StoredJob,
} from "./store.js";

export type JsonObject = Record<string, unknown>;

export class FixtureError extends Error {}

export function runCase(raw: unknown): JsonObject {
  const fixture = object(raw, "fixture");
  const operation = string(fixture, "operation");
  if (operation === "create_job") {
    return runCreateCase(fixture);
  }
  if (operation === "decode_stored_job") {
    return runDecodeStoredCase(fixture);
  }
  throw new FixtureError(`unsupported operation ${JSON.stringify(operation)}`);
}

function runCreateCase(fixture: JsonObject): JsonObject {
  const caseId = string(fixture, "case_id");
  const serviceVersion = recordVersion(integer(fixture, "service_version"));
  const initialJobs = array(fixture, "initial_jobs").map(parseStoredJob);
  const { decoded, unknownPriority } = decodeRequest(
    serviceVersion,
    object(fixture.request, "request"),
  );

  let actual: JsonObject;
  if (unknownPriority !== undefined) {
    actual = {
      decode_error: {
        code: "unknown_priority",
        field: "priority",
        value: unknownPriority,
      },
      final_jobs: initialJobs.map(encodeStoredJob),
      store_calls: [],
    };
  } else {
    const outcome = parseStoreOutcome(fixture.store_outcome);
    const store = new DeterministicJobStore(
      initialJobs,
      outcome,
      serviceVersion,
    );
    const result = createJob(decoded.request, store);
    actual = {
      response: encodeResponse(decoded.version, result),
      final_jobs: store.jobs.map(encodeStoredJob),
      store_calls: store.calls.map((call) => ({
        operation: "insert_if_absent",
        job: encodeStoredJob(call.job),
      })),
    };
  }
  return {
    result_format: 1,
    case_id: caseId,
    operation: "create_job",
    actual,
  };
}

function runDecodeStoredCase(fixture: JsonObject): JsonObject {
  const stored = parseStoredJob(object(fixture.stored_job, "stored_job"));
  return {
    result_format: 1,
    case_id: string(fixture, "case_id"),
    operation: "decode_stored_job",
    actual: { decoded_job: encodeJobV2(adaptStoredJobToV2(stored)) },
  };
}

function decodeRequest(
  serviceVersion: RecordVersionType,
  raw: JsonObject,
): Readonly<{
  decoded: Readonly<{ version: ApiVersion; request: CreateJobRequest }>;
  unknownPriority: string | undefined;
}> {
  const rawVersion = integer(raw, "api_version");
  if (rawVersion !== ApiVersion.v1 && rawVersion !== ApiVersion.v2) {
    throw new FixtureError(`unsupported request API version ${rawVersion}`);
  }
  const version: ApiVersion = rawVersion;
  if (serviceVersion === RecordVersion.v1 && version !== ApiVersion.v1) {
    throw new FixtureError("service version 1 accepts only API version 1");
  }

  let priority: Priority | undefined;
  let unknownPriority: string | undefined;
  if (version === ApiVersion.v1) {
    priority = Priority.normal;
  } else if ("priority" in raw) {
    const value = string(raw, "priority");
    priority = parsePriority(value);
    if (priority === undefined) {
      unknownPriority = value;
    }
  }

  return {
    decoded: {
      version,
      request: {
        jobId: string(raw, "job_id"),
        task: string(raw, "task"),
        payload: decodeBase64(string(raw, "payload_base64"), "payload"),
        priority,
      },
    },
    unknownPriority,
  };
}

function parseStoredJob(raw: unknown): StoredJob {
  const value = object(raw, "stored job");
  const version = recordVersion(integer(value, "record_version"));
  let priority: Priority | undefined;
  if (version === RecordVersion.v2) {
    if (!("priority" in value)) {
      throw new FixtureError("version-two stored job is missing priority");
    }
    const rawPriority = string(value, "priority");
    priority = parsePriority(rawPriority);
    if (priority === undefined) {
      throw new FixtureError(
        `version-two stored job has unknown priority ${JSON.stringify(rawPriority)}`,
      );
    }
  }
  return {
    recordVersion: version,
    jobId: string(value, "job_id"),
    task: string(value, "task"),
    payload: decodeBase64(string(value, "payload_base64"), "stored payload"),
    priority,
  };
}

function parseStoreOutcome(value: unknown): InsertOutcome {
  if (value === undefined) {
    return InsertOutcome.unavailableBeforeCommit;
  }
  if (
    value === InsertOutcome.inserted ||
    value === InsertOutcome.duplicate ||
    value === InsertOutcome.unavailableBeforeCommit
  ) {
    return value;
  }
  throw new FixtureError(`unsupported store outcome ${JSON.stringify(value)}`);
}

function recordVersion(value: number): RecordVersionType {
  if (value === RecordVersion.v1 || value === RecordVersion.v2) {
    return value;
  }
  throw new FixtureError(`unsupported service or record version ${value}`);
}

function encodeResponse(
  version: ApiVersion,
  result: CreateJobResult,
): JsonObject {
  let encoded: JsonObject;
  switch (result.kind) {
    case "created":
      encoded = {
        kind: "created",
        job:
          version === ApiVersion.v1
            ? {
                job_id: result.job.jobId,
                task: result.job.task,
                payload_base64: encodeBase64(result.job.payload),
              }
            : {
                job_id: result.job.jobId,
                task: result.job.task,
                payload_base64: encodeBase64(result.job.payload),
                priority: result.job.priority,
              },
      };
      break;
    case "invalid":
      encoded = {
        kind: "invalid",
        issues: result.issues.map((issue) => ({ ...issue })),
      };
      break;
    case "already_exists":
      encoded = { kind: "already_exists", job_id: result.jobId };
      break;
    case "persistence_unavailable":
      encoded = { kind: "persistence_unavailable" };
      break;
    default:
      return assertNever(result);
  }
  return { api_version: version, result: encoded };
}

function encodeJobV2(job: Job): JsonObject {
  return {
    record_version: 2,
    job_id: job.jobId,
    task: job.task,
    payload_base64: encodeBase64(job.payload),
    priority: job.priority,
  };
}

function encodeStoredJob(job: StoredJob): JsonObject {
  const result: JsonObject = {
    record_version: job.recordVersion,
    job_id: job.jobId,
    task: job.task,
    payload_base64: encodeBase64(job.payload),
  };
  if (job.recordVersion === RecordVersion.v2) {
    if (job.priority === undefined) {
      throw new Error("version-two stored job must have priority");
    }
    result.priority = job.priority;
  }
  return result;
}

function decodeBase64(value: string, label: string): Uint8Array {
  if (
    value.length % 4 !== 0 ||
    !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/u.test(
      value,
    )
  ) {
    throw new FixtureError(`invalid ${label} Base64`);
  }
  return Uint8Array.from(Buffer.from(value, "base64"));
}

function encodeBase64(value: Uint8Array): string {
  return Buffer.from(value).toString("base64");
}

function object(value: unknown, label: string): JsonObject {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new FixtureError(`${label} must be an object`);
  }
  return value as JsonObject;
}

function array(value: JsonObject, key: string): readonly unknown[] {
  const result = value[key];
  if (!Array.isArray(result)) {
    throw new FixtureError(`${key} must be an array`);
  }
  return result;
}

function string(value: JsonObject, key: string): string {
  const result = value[key];
  if (typeof result !== "string") {
    throw new FixtureError(`${key} must be a string`);
  }
  return result;
}

function integer(value: JsonObject, key: string): number {
  const result = value[key];
  if (!Number.isInteger(result)) {
    throw new FixtureError(`${key} must be an integer`);
  }
  return result as number;
}

function assertNever(value: never): never {
  throw new Error(`unhandled create-job result ${JSON.stringify(value)}`);
}
