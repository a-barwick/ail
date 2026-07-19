use std::fmt;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    domain::{
        ApiVersion, CreateJobRequest, CreateJobResult, InsertOutcome, Job, Priority,
        ValidationField, ValidationIssue, ValidationReason,
    },
    service::create_job,
    store::{DeterministicJobStore, RecordVersion, StoreCall, StoredJob},
};

#[derive(Debug)]
pub struct RunnerError(String);

impl RunnerError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for RunnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for RunnerError {}

#[derive(Debug, Serialize)]
pub struct SingleCaseResult {
    pub result_format: u8,
    pub case_id: String,
    pub operation: String,
    pub actual: Value,
}

#[derive(Debug, Deserialize)]
struct CaseHeader {
    operation: String,
}

#[derive(Debug, Deserialize)]
struct CreateCase {
    case_id: String,
    service_version: u8,
    request: RawRequest,
    #[serde(default)]
    initial_jobs: Vec<RawStoredJob>,
    store_outcome: Option<RawStoreOutcome>,
}

#[derive(Debug, Deserialize)]
struct DecodeStoredCase {
    case_id: String,
    stored_job: RawStoredJob,
}

#[derive(Debug, Deserialize)]
struct RawRequest {
    api_version: u8,
    job_id: String,
    task: String,
    payload_base64: String,
    priority: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct RawStoredJob {
    record_version: u8,
    job_id: String,
    task: String,
    payload_base64: String,
    priority: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RawStoreOutcome {
    Inserted,
    Duplicate,
    UnavailableBeforeCommit,
}

#[derive(Debug)]
enum RequestDecode {
    Decoded {
        version: ApiVersion,
        request: CreateJobRequest,
    },
    UnknownPriority(String),
}

/// Executes one parsed fixture through the Rust service boundary.
///
/// # Errors
///
/// Returns an error when the fixture shape, version, encoding, or operation is
/// outside the frozen runner contract.
pub fn run_case_value(value: Value) -> Result<SingleCaseResult, RunnerError> {
    let header: CaseHeader = serde_json::from_value(value.clone())
        .map_err(|error| RunnerError::new(format!("invalid fixture header: {error}")))?;
    match header.operation.as_str() {
        "create_job" => run_create_case(value),
        "decode_stored_job" => run_decode_stored_case(value),
        operation => Err(RunnerError::new(format!(
            "unsupported operation {operation:?}"
        ))),
    }
}

fn run_create_case(value: Value) -> Result<SingleCaseResult, RunnerError> {
    let raw: CreateCase = serde_json::from_value(value)
        .map_err(|error| RunnerError::new(format!("invalid create_job fixture: {error}")))?;
    let insert_version = parse_service_version(raw.service_version)?;
    let initial_jobs = raw
        .initial_jobs
        .into_iter()
        .map(parse_stored_job)
        .collect::<Result<Vec<_>, _>>()?;

    let decoded = decode_request(raw.service_version, raw.request)?;
    let actual = match decoded {
        RequestDecode::UnknownPriority(value) => json!({
            "decode_error": {
                "code": "unknown_priority",
                "field": "priority",
                "value": value,
            },
            "final_jobs": initial_jobs.iter().map(stored_job_json).collect::<Vec<_>>(),
            "store_calls": [],
        }),
        RequestDecode::Decoded { version, request } => {
            let outcome = raw
                .store_outcome
                .map_or(InsertOutcome::UnavailableBeforeCommit, InsertOutcome::from);
            let mut store = DeterministicJobStore::new(initial_jobs, outcome, insert_version);
            let result = create_job(request, &mut store);
            json!({
                "response": response_json(version, &result),
                "final_jobs": store.jobs().iter().map(stored_job_json).collect::<Vec<_>>(),
                "store_calls": store.calls().iter().map(store_call_json).collect::<Vec<_>>(),
            })
        }
    };

    Ok(SingleCaseResult {
        result_format: 1,
        case_id: raw.case_id,
        operation: "create_job".into(),
        actual,
    })
}

fn run_decode_stored_case(value: Value) -> Result<SingleCaseResult, RunnerError> {
    let raw: DecodeStoredCase = serde_json::from_value(value)
        .map_err(|error| RunnerError::new(format!("invalid decode_stored_job fixture: {error}")))?;
    let stored = parse_stored_job(raw.stored_job)?;
    let decoded = stored.adapt_to_v2();
    Ok(SingleCaseResult {
        result_format: 1,
        case_id: raw.case_id,
        operation: "decode_stored_job".into(),
        actual: json!({"decoded_job": job_v2_json(&decoded)}),
    })
}

fn parse_service_version(version: u8) -> Result<RecordVersion, RunnerError> {
    match version {
        1 => Ok(RecordVersion::V1),
        2 => Ok(RecordVersion::V2),
        other => Err(RunnerError::new(format!(
            "unsupported service version {other}"
        ))),
    }
}

fn decode_request(service_version: u8, raw: RawRequest) -> Result<RequestDecode, RunnerError> {
    let version = match raw.api_version {
        1 => ApiVersion::V1,
        2 => ApiVersion::V2,
        other => {
            return Err(RunnerError::new(format!(
                "unsupported request API version {other}"
            )));
        }
    };
    if service_version == 1 && version != ApiVersion::V1 {
        return Err(RunnerError::new(
            "service version 1 accepts only API version 1",
        ));
    }

    let priority = match version {
        ApiVersion::V1 => Some(Priority::Normal),
        ApiVersion::V2 => match raw.priority.as_deref() {
            None => None,
            Some(value) => match parse_priority(value) {
                Some(priority) => Some(priority),
                None => return Ok(RequestDecode::UnknownPriority(value.into())),
            },
        },
    };
    let payload = decode_payload(&raw.payload_base64)?;
    Ok(RequestDecode::Decoded {
        version,
        request: CreateJobRequest {
            job_id: raw.job_id,
            task: raw.task,
            payload,
            priority,
        },
    })
}

fn parse_stored_job(raw: RawStoredJob) -> Result<StoredJob, RunnerError> {
    let payload = decode_payload(&raw.payload_base64)?;
    match raw.record_version {
        1 => Ok(StoredJob::V1 {
            job_id: raw.job_id,
            task: raw.task,
            payload,
        }),
        2 => {
            let raw_priority = raw
                .priority
                .as_deref()
                .ok_or_else(|| RunnerError::new("version-two stored job is missing priority"))?;
            let priority = parse_priority(raw_priority).ok_or_else(|| {
                RunnerError::new(format!(
                    "version-two stored job has unknown priority {raw_priority:?}"
                ))
            })?;
            Ok(StoredJob::V2 {
                job_id: raw.job_id,
                task: raw.task,
                payload,
                priority,
            })
        }
        version => Err(RunnerError::new(format!(
            "unsupported stored record version {version}"
        ))),
    }
}

fn decode_payload(encoded: &str) -> Result<Vec<u8>, RunnerError> {
    STANDARD
        .decode(encoded)
        .map_err(|error| RunnerError::new(format!("invalid payload Base64: {error}")))
}

fn encode_payload(payload: &[u8]) -> String {
    STANDARD.encode(payload)
}

fn parse_priority(value: &str) -> Option<Priority> {
    match value {
        "low" => Some(Priority::Low),
        "normal" => Some(Priority::Normal),
        "high" => Some(Priority::High),
        _ => None,
    }
}

fn priority_name(priority: Priority) -> &'static str {
    match priority {
        Priority::Low => "low",
        Priority::Normal => "normal",
        Priority::High => "high",
    }
}

fn response_json(version: ApiVersion, result: &CreateJobResult) -> Value {
    let result = match result {
        CreateJobResult::Created(job) => match version {
            ApiVersion::V1 => json!({
                "kind": "created",
                "job": job_v1_json(job),
            }),
            ApiVersion::V2 => json!({
                "kind": "created",
                "job": job_v2_response_json(job),
            }),
        },
        CreateJobResult::Invalid(issues) => json!({
            "kind": "invalid",
            "issues": issues.iter().copied().map(validation_issue_json).collect::<Vec<_>>(),
        }),
        CreateJobResult::AlreadyExists(job_id) => json!({
            "kind": "already_exists",
            "job_id": job_id,
        }),
        CreateJobResult::PersistenceUnavailable => json!({
            "kind": "persistence_unavailable",
        }),
    };
    let api_version = match version {
        ApiVersion::V1 => 1,
        ApiVersion::V2 => 2,
    };
    json!({
        "api_version": api_version,
        "result": result,
    })
}

fn validation_issue_json(issue: ValidationIssue) -> Value {
    let field = match issue.field {
        ValidationField::JobId => "job_id",
        ValidationField::Task => "task",
        ValidationField::Payload => "payload",
        ValidationField::Priority => "priority",
    };
    let reason = match issue.reason {
        ValidationReason::Missing => "missing",
        ValidationReason::InvalidFormat => "invalid_format",
        ValidationReason::TooLong => "too_long",
        ValidationReason::ControlCharacter => "control_character",
        ValidationReason::PayloadTooLarge => "payload_too_large",
    };
    json!({
        "field": field,
        "reason": reason,
    })
}

fn job_v1_json(job: &Job) -> Value {
    json!({
        "job_id": job.job_id,
        "task": job.task,
        "payload_base64": encode_payload(&job.payload),
    })
}

fn job_v2_response_json(job: &Job) -> Value {
    json!({
        "job_id": job.job_id,
        "task": job.task,
        "payload_base64": encode_payload(&job.payload),
        "priority": priority_name(job.priority),
    })
}

fn job_v2_json(job: &Job) -> Value {
    json!({
        "record_version": 2,
        "job_id": job.job_id,
        "task": job.task,
        "payload_base64": encode_payload(&job.payload),
        "priority": priority_name(job.priority),
    })
}

fn stored_job_json(job: &StoredJob) -> Value {
    match job {
        StoredJob::V1 {
            job_id,
            task,
            payload,
        } => json!({
            "record_version": 1,
            "job_id": job_id,
            "task": task,
            "payload_base64": encode_payload(payload),
        }),
        StoredJob::V2 {
            job_id,
            task,
            payload,
            priority,
        } => json!({
            "record_version": 2,
            "job_id": job_id,
            "task": task,
            "payload_base64": encode_payload(payload),
            "priority": priority_name(*priority),
        }),
    }
}

fn store_call_json(call: &StoreCall) -> Value {
    json!({
        "operation": "insert_if_absent",
        "job": stored_job_json(&call.job),
    })
}

impl From<RawStoreOutcome> for InsertOutcome {
    fn from(value: RawStoreOutcome) -> Self {
        match value {
            RawStoreOutcome::Inserted => Self::Inserted,
            RawStoreOutcome::Duplicate => Self::Duplicate,
            RawStoreOutcome::UnavailableBeforeCommit => Self::UnavailableBeforeCommit,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::Value;

    use super::*;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .and_then(std::path::Path::parent)
            .and_then(std::path::Path::parent)
            .expect("v2 crate is nested under benchmarks/baselines/rust")
            .to_path_buf()
    }

    #[test]
    fn every_public_fixture_matches_the_shared_oracle() {
        let root = repo_root();
        let manifest: Value = serde_json::from_slice(
            &fs::read(root.join("benchmarks/fixtures/manifest.json"))
                .expect("read fixture manifest"),
        )
        .expect("parse fixture manifest");
        let entries = manifest["fixtures"]
            .as_array()
            .expect("fixture entries are an array");
        assert_eq!(entries.len(), 37);

        for entry in entries {
            let path = entry["path"].as_str().expect("fixture path");
            let fixture: Value =
                serde_json::from_slice(&fs::read(root.join(path)).expect("read fixture"))
                    .expect("parse fixture");
            let expected = fixture["expected"].clone();
            let result = run_case_value(fixture).expect("run fixture");
            assert_eq!(result.actual, expected, "shared oracle mismatch for {path}");
        }
    }

    #[test]
    fn unknown_priority_is_a_zero_effect_boundary_error() {
        let fixture = json!({
            "case_id": "unknown",
            "service_version": 2,
            "operation": "create_job",
            "request": {
                "api_version": 2,
                "job_id": "job-1",
                "task": "task",
                "payload_base64": "",
                "priority": "urgent",
            },
            "initial_jobs": [],
        });
        let result = run_case_value(fixture).expect("run fixture");
        assert_eq!(
            result.actual,
            json!({
                "decode_error": {
                    "code": "unknown_priority",
                    "field": "priority",
                    "value": "urgent",
                },
                "final_jobs": [],
                "store_calls": [],
            })
        );
    }

    #[test]
    fn v1_response_projection_omits_internal_priority() {
        let response = response_json(
            ApiVersion::V1,
            &CreateJobResult::Created(Job {
                job_id: "job-1".into(),
                task: "task".into(),
                payload: Vec::new(),
                priority: Priority::High,
            }),
        );
        assert!(response["result"]["job"].get("priority").is_none());
    }

    #[test]
    fn stored_v1_decoder_sets_normal_explicitly() {
        let fixture = json!({
            "case_id": "legacy",
            "operation": "decode_stored_job",
            "stored_job": {
                "record_version": 1,
                "job_id": "job-legacy",
                "task": "legacy-task",
                "payload_base64": "bGVnYWN5",
            },
        });
        let result = run_case_value(fixture).expect("run fixture");
        assert_eq!(result.actual["decoded_job"]["priority"], "normal");
        assert_eq!(result.actual["decoded_job"]["record_version"], 2);
    }
}
