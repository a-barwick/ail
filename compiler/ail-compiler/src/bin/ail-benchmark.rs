use std::{env, fs, path::Path, process::ExitCode};

use ail_compiler::{
    CapabilityEnvironment, CapabilityInterface, CapabilityOperation, CapabilityProvider,
    ExecutionRequest, ExecutionResponse, RuntimeFault, RuntimeValue, Workspace, source_digest,
};
use serde_json::{Value, json};

const SERVICE_SOURCE: &str = include_str!("../../../../specs/runtime-fixtures/job-service.ail");
const REVISION_ID: &str = "m17-job-service-r1";

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match run(&arguments) {
        Ok(result) => {
            println!("{result}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("AIL benchmark runner: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[String]) -> Result<Value, String> {
    let runtime = ServiceRuntime::new()?;
    match arguments {
        [flag, path] if flag == "--case" => runtime.run_case_file(Path::new(path)),
        [flag, path] if flag == "--corpus" => runtime.run_corpus(Path::new(path)),
        _ => Err("expected exactly --case <fixture> or --corpus <manifest>".to_owned()),
    }
}

struct ServiceRuntime {
    workspace: Workspace,
}

impl ServiceRuntime {
    fn new() -> Result<Self, String> {
        let mut jobs = CapabilityInterface::new();
        jobs.insert_operation(
            "insert_if_absent",
            CapabilityOperation::new(["Job"], "InsertOutcome"),
        );
        let mut environment = CapabilityEnvironment::new();
        environment.insert_interface("JobsStore", jobs);
        let workspace = Workspace::new(
            "m17-job-service",
            REVISION_ID,
            "specs/runtime-fixtures/job-service.ail",
            SERVICE_SOURCE,
            environment,
        )
        .map_err(|failure| format!("reference service parse failed: {failure:?}"))?;
        Ok(Self { workspace })
    }

    fn run_case_file(&self, path: &Path) -> Result<Value, String> {
        let fixture = read_json(path)?;
        self.run_case(&fixture)
    }

    fn run_corpus(&self, path: &Path) -> Result<Value, String> {
        let bytes = fs::read(path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let manifest: Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("could not parse {}: {error}", path.display()))?;
        let entries = manifest
            .get("fixtures")
            .and_then(Value::as_array)
            .ok_or("fixture manifest has no fixtures array")?;
        let results = entries
            .iter()
            .map(|entry| {
                let fixture_path = required_str(entry, "path")?;
                self.run_case_file(Path::new(fixture_path))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let manifest_text = String::from_utf8(bytes)
            .map_err(|error| format!("fixture manifest is not UTF-8: {error}"))?;
        Ok(json!({
            "result_format": 1,
            "fixture_manifest_sha256": source_digest(&manifest_text).trim_start_matches("sha256:"),
            "results": results,
        }))
    }

    fn run_case(&self, fixture: &Value) -> Result<Value, String> {
        let case_id = required_str(fixture, "case_id")?;
        let operation = required_str(fixture, "operation")?;
        let actual = match operation {
            "create_job" => self.run_create_job(fixture)?,
            "decode_stored_job" => self.run_decode_stored_job(fixture)?,
            _ => return Err(format!("unsupported operation {operation:?}")),
        };
        Ok(json!({
            "result_format": 1,
            "case_id": case_id,
            "operation": operation,
            "actual": actual,
        }))
    }

    fn run_create_job(&self, fixture: &Value) -> Result<Value, String> {
        let request = required_object(fixture, "request")?;
        let initial_jobs = fixture
            .get("initial_jobs")
            .and_then(Value::as_array)
            .ok_or("create_job fixture has no initial_jobs array")?
            .clone();
        let api_version = required_u64(request, "api_version")?;
        let decoded = match self.decode_request(request, api_version)? {
            DecodedRequest::Value(value) => value,
            DecodedRequest::UnknownPriority(value) => {
                return Ok(json!({
                    "decode_error": {
                        "code": "unknown_priority",
                        "field": "priority",
                        "value": value,
                    },
                    "final_jobs": initial_jobs,
                    "store_calls": [],
                }));
            }
        };

        let insert_version = required_u64(fixture, "service_version")?;
        let outcome = fixture
            .get("store_outcome")
            .and_then(Value::as_str)
            .unwrap_or("inserted");
        let mut store = JobStoreProvider::new(initial_jobs, outcome, insert_version)?;
        let response = self.workspace.execute(
            ExecutionRequest {
                revision_id: REVISION_ID.to_owned(),
                function: "create_job".to_owned(),
                arguments: vec![decoded],
            },
            &mut store,
        );
        let success = completed(response)?;
        if success.calls.len() != store.calls.len() {
            return Err("interpreter and store capability call traces differ".to_owned());
        }
        Ok(json!({
            "response": {
                "api_version": api_version,
                "result": project_result(&success.value, api_version)?,
            },
            "final_jobs": store.jobs,
            "store_calls": store.calls,
        }))
    }

    fn decode_request(&self, request: &Value, api_version: u64) -> Result<DecodedRequest, String> {
        let job_id = required_str(request, "job_id")?;
        let task = required_str(request, "task")?;
        let payload = decode_base64(required_str(request, "payload_base64")?)?;
        match api_version {
            1 => {
                let value = RuntimeValue::record(
                    "CreateJobRequestV1",
                    [
                        ("job_id", RuntimeValue::Text(job_id.to_owned())),
                        ("task", RuntimeValue::Text(task.to_owned())),
                        ("payload", RuntimeValue::Bytes(payload)),
                    ],
                );
                let success = completed(self.workspace.execute(
                    ExecutionRequest {
                        revision_id: REVISION_ID.to_owned(),
                        function: "adapt_create_job_request_v1".to_owned(),
                        arguments: vec![value],
                    },
                    &mut NoCapabilities,
                ))?;
                Ok(DecodedRequest::Value(success.value))
            }
            2 => {
                let priority = match request.get("priority") {
                    None => RuntimeValue::variant("PriorityOption", "None", None),
                    Some(Value::String(value))
                        if matches!(value.as_str(), "low" | "normal" | "high") =>
                    {
                        RuntimeValue::variant(
                            "PriorityOption",
                            "Some",
                            Some(priority_value(value)?),
                        )
                    }
                    Some(Value::String(value)) => {
                        return Ok(DecodedRequest::UnknownPriority(value.clone()));
                    }
                    Some(_) => return Err("priority must be text".to_owned()),
                };
                Ok(DecodedRequest::Value(RuntimeValue::record(
                    "CreateJobRequest",
                    [
                        ("job_id", RuntimeValue::Text(job_id.to_owned())),
                        ("task", RuntimeValue::Text(task.to_owned())),
                        ("payload", RuntimeValue::Bytes(payload)),
                        ("priority", priority),
                    ],
                )))
            }
            _ => Err(format!("unsupported request api_version {api_version}")),
        }
    }

    fn run_decode_stored_job(&self, fixture: &Value) -> Result<Value, String> {
        let stored = required_object(fixture, "stored_job")?;
        let version = required_u64(stored, "record_version")?;
        let job = match version {
            1 => {
                let value = RuntimeValue::record(
                    "StoredJobV1",
                    [
                        (
                            "job_id",
                            RuntimeValue::Text(required_str(stored, "job_id")?.to_owned()),
                        ),
                        (
                            "task",
                            RuntimeValue::Text(required_str(stored, "task")?.to_owned()),
                        ),
                        (
                            "payload",
                            RuntimeValue::Bytes(decode_base64(required_str(
                                stored,
                                "payload_base64",
                            )?)?),
                        ),
                    ],
                );
                completed(self.workspace.execute(
                    ExecutionRequest {
                        revision_id: REVISION_ID.to_owned(),
                        function: "adapt_stored_job".to_owned(),
                        arguments: vec![value],
                    },
                    &mut NoCapabilities,
                ))?
                .value
            }
            2 => decode_v2_job(stored)?,
            _ => return Err(format!("unsupported stored record_version {version}")),
        };
        Ok(json!({ "decoded_job": project_stored_job(&job, 2)? }))
    }
}

enum DecodedRequest {
    Value(RuntimeValue),
    UnknownPriority(String),
}

struct NoCapabilities;

impl CapabilityProvider for NoCapabilities {
    fn supports(&self, _receiver: &str, _interface: &str) -> bool {
        false
    }

    fn call(
        &mut self,
        _receiver: &str,
        _interface: &str,
        _operation: &str,
        _arguments: &[RuntimeValue],
    ) -> Result<RuntimeValue, RuntimeFault> {
        Err(RuntimeFault::new(
            "AIL.RUNTIME.UNEXPECTED_CAPABILITY",
            ail_compiler::Span::empty(0),
            std::iter::empty::<(&str, &str)>(),
            std::iter::empty::<(&str, &str)>(),
        ))
    }
}

struct JobStoreProvider {
    jobs: Vec<Value>,
    calls: Vec<Value>,
    outcome: &'static str,
    insert_version: u64,
}

impl JobStoreProvider {
    fn new(jobs: Vec<Value>, outcome: &str, insert_version: u64) -> Result<Self, String> {
        let outcome = match outcome {
            "inserted" => "Inserted",
            "duplicate" => "Duplicate",
            "unavailable_before_commit" => "UnavailableBeforeCommit",
            _ => return Err(format!("unsupported store_outcome {outcome:?}")),
        };
        if !matches!(insert_version, 1 | 2) {
            return Err(format!("unsupported service_version {insert_version}"));
        }
        Ok(Self {
            jobs,
            calls: Vec::new(),
            outcome,
            insert_version,
        })
    }
}

impl CapabilityProvider for JobStoreProvider {
    fn supports(&self, receiver: &str, interface: &str) -> bool {
        receiver == "jobs" && interface == "JobsStore"
    }

    fn call(
        &mut self,
        receiver: &str,
        interface: &str,
        operation: &str,
        arguments: &[RuntimeValue],
    ) -> Result<RuntimeValue, RuntimeFault> {
        if (receiver, interface, operation) != ("jobs", "JobsStore", "insert_if_absent")
            || arguments.len() != 1
        {
            return Err(host_fault("AIL.RUNTIME.INVALID_CAPABILITY_CALL"));
        }
        let stored = project_stored_job(&arguments[0], self.insert_version)
            .map_err(|_| host_fault("AIL.RUNTIME.INVALID_CAPABILITY_ARGUMENT"))?;
        self.calls.push(json!({
            "operation": "insert_if_absent",
            "job": stored,
        }));
        if self.outcome == "Inserted" {
            let job_id = required_str(&stored, "job_id")
                .map_err(|_| host_fault("AIL.RUNTIME.INVALID_CAPABILITY_ARGUMENT"))?;
            if self
                .jobs
                .iter()
                .any(|job| job.get("job_id").and_then(Value::as_str) == Some(job_id))
            {
                return Err(host_fault("AIL.RUNTIME.CAPABILITY_POSTCONDITION"));
            }
            self.jobs.push(stored);
        }
        Ok(RuntimeValue::variant("InsertOutcome", self.outcome, None))
    }
}

fn completed(response: ExecutionResponse) -> Result<ail_compiler::ExecutionSuccess, String> {
    match response {
        ExecutionResponse::Completed(success) => Ok(success),
        ExecutionResponse::Failed(failure) => Err(format!(
            "{} failed with {} at {}..{}",
            failure.function, failure.fault.code, failure.fault.span.start, failure.fault.span.end
        )),
    }
}

fn host_fault(code: &'static str) -> RuntimeFault {
    RuntimeFault::new(
        code,
        ail_compiler::Span::empty(0),
        std::iter::empty::<(&str, &str)>(),
        std::iter::empty::<(&str, &str)>(),
    )
}

fn project_result(value: &RuntimeValue, api_version: u64) -> Result<Value, String> {
    let (type_name, case, payload) = variant_parts(value)?;
    if type_name != "CreateJobResult" {
        return Err(format!("expected CreateJobResult, received {type_name}"));
    }
    match case {
        "Created" => Ok(json!({
            "kind": "created",
            "job": project_response_job(required_payload(payload)?, api_version)?,
        })),
        "Invalid" => Ok(json!({
            "kind": "invalid",
            "issues": project_issues(required_payload(payload)?)?,
        })),
        "AlreadyExists" => Ok(json!({
            "kind": "already_exists",
            "job_id": runtime_text(required_payload(payload)?)?,
        })),
        "PersistenceUnavailable" if payload.is_none() => {
            Ok(json!({ "kind": "persistence_unavailable" }))
        }
        _ => Err(format!("invalid CreateJobResult case {case}")),
    }
}

fn project_issues(value: &RuntimeValue) -> Result<Vec<Value>, String> {
    let mut issues = Vec::new();
    let mut current = value;
    loop {
        let (type_name, case, payload) = variant_parts(current)?;
        if type_name != "ValidationIssues" {
            return Err(format!("expected ValidationIssues, received {type_name}"));
        }
        match case {
            "Empty" if payload.is_none() => return Ok(issues),
            "More" => {
                let node = required_payload(payload)?;
                let issue = runtime_field(node, "issue")?;
                issues.push(project_issue(issue)?);
                current = runtime_field(node, "rest")?;
            }
            _ => return Err(format!("invalid ValidationIssues case {case}")),
        }
    }
}

fn project_issue(value: &RuntimeValue) -> Result<Value, String> {
    let field = unit_variant_case(runtime_field(value, "field")?)?;
    let reason = unit_variant_case(runtime_field(value, "reason")?)?;
    let field = match field {
        "JobId" => "job_id",
        "Task" => "task",
        "Payload" => "payload",
        "Priority" => "priority",
        _ => return Err(format!("unknown ValidationField case {field}")),
    };
    let reason = match reason {
        "Missing" => "missing",
        "InvalidFormat" => "invalid_format",
        "TooLong" => "too_long",
        "ControlCharacter" => "control_character",
        "PayloadTooLarge" => "payload_too_large",
        _ => return Err(format!("unknown ValidationReason case {reason}")),
    };
    Ok(json!({ "field": field, "reason": reason }))
}

fn project_response_job(value: &RuntimeValue, api_version: u64) -> Result<Value, String> {
    let mut job = json!({
        "job_id": runtime_text(runtime_field(value, "job_id")?)?,
        "task": runtime_text(runtime_field(value, "task")?)?,
        "payload_base64": encode_base64(runtime_bytes(runtime_field(value, "payload")?)?),
    });
    if api_version == 2 {
        job["priority"] =
            Value::String(priority_name(runtime_field(value, "priority")?)?.to_owned());
    }
    Ok(job)
}

fn project_stored_job(value: &RuntimeValue, record_version: u64) -> Result<Value, String> {
    let mut job = project_response_job(value, record_version)?;
    job["record_version"] = Value::Number(record_version.into());
    Ok(job)
}

fn decode_v2_job(value: &Value) -> Result<RuntimeValue, String> {
    Ok(RuntimeValue::record(
        "Job",
        [
            (
                "job_id",
                RuntimeValue::Text(required_str(value, "job_id")?.to_owned()),
            ),
            (
                "task",
                RuntimeValue::Text(required_str(value, "task")?.to_owned()),
            ),
            (
                "payload",
                RuntimeValue::Bytes(decode_base64(required_str(value, "payload_base64")?)?),
            ),
            (
                "priority",
                priority_value(required_str(value, "priority")?)?,
            ),
        ],
    ))
}

fn priority_value(value: &str) -> Result<RuntimeValue, String> {
    let case = match value {
        "low" => "Low",
        "normal" => "Normal",
        "high" => "High",
        _ => return Err(format!("unknown priority {value:?}")),
    };
    Ok(RuntimeValue::variant("Priority", case, None))
}

fn priority_name(value: &RuntimeValue) -> Result<&'static str, String> {
    match unit_variant_case(value)? {
        "Low" => Ok("low"),
        "Normal" => Ok("normal"),
        "High" => Ok("high"),
        case => Err(format!("unknown Priority case {case}")),
    }
}

fn runtime_field<'a>(value: &'a RuntimeValue, name: &str) -> Result<&'a RuntimeValue, String> {
    value
        .field(name)
        .ok_or_else(|| format!("{} has no field {name}", value.type_name()))
}

fn variant_parts(value: &RuntimeValue) -> Result<(&str, &str, Option<&RuntimeValue>), String> {
    let RuntimeValue::Variant {
        type_name,
        case,
        payload,
    } = value
    else {
        return Err(format!("expected variant, received {}", value.type_name()));
    };
    Ok((type_name, case, payload.as_deref()))
}

fn unit_variant_case(value: &RuntimeValue) -> Result<&str, String> {
    let (_, case, payload) = variant_parts(value)?;
    if payload.is_some() {
        return Err(format!("expected unit case, received payload for {case}"));
    }
    Ok(case)
}

fn required_payload(payload: Option<&RuntimeValue>) -> Result<&RuntimeValue, String> {
    payload.ok_or_else(|| "expected variant payload".to_owned())
}

fn runtime_text(value: &RuntimeValue) -> Result<&str, String> {
    let RuntimeValue::Text(value) = value else {
        return Err(format!("expected Text, received {}", value.type_name()));
    };
    Ok(value)
}

fn runtime_bytes(value: &RuntimeValue) -> Result<&[u8], String> {
    let RuntimeValue::Bytes(value) = value else {
        return Err(format!("expected Bytes, received {}", value.type_name()));
    };
    Ok(value)
}

fn read_json(path: &Path) -> Result<Value, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("could not parse {}: {error}", path.display()))
}

fn required_object<'a>(value: &'a Value, field: &str) -> Result<&'a Value, String> {
    value
        .get(field)
        .filter(|value| value.is_object())
        .ok_or_else(|| format!("missing object field {field}"))
}

fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing text field {field}"))
}

fn required_u64(value: &Value, field: &str) -> Result<u64, String> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing integer field {field}"))
}

fn decode_base64(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 4 != 0 {
        return Err("Base64 input length is not divisible by four".to_owned());
    }
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(value.len() / 4 * 3);
    for (index, chunk) in bytes.chunks_exact(4).enumerate() {
        let final_chunk = index + 1 == bytes.len() / 4;
        let padding = chunk.iter().rev().take_while(|byte| **byte == b'=').count();
        if padding > 2 || (!final_chunk && padding != 0) {
            return Err("Base64 padding is invalid".to_owned());
        }
        let a = base64_value(chunk[0])?;
        let b = base64_value(chunk[1])?;
        let c = if padding >= 2 {
            0
        } else {
            base64_value(chunk[2])?
        };
        let d = if padding >= 1 {
            0
        } else {
            base64_value(chunk[3])?
        };
        decoded.push((a << 2) | (b >> 4));
        if padding < 2 {
            decoded.push((b << 4) | (c >> 2));
        }
        if padding == 0 {
            decoded.push((c << 6) | d);
        }
    }
    if encode_base64(&decoded) != value {
        return Err("Base64 input is not canonical padded encoding".to_owned());
    }
    Ok(decoded)
}

fn base64_value(value: u8) -> Result<u8, String> {
    match value {
        b'A'..=b'Z' => Ok(value - b'A'),
        b'a'..=b'z' => Ok(value - b'a' + 26),
        b'0'..=b'9' => Ok(value - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(format!("invalid Base64 byte {value}")),
    }
}

fn encode_base64(value: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(value.len().div_ceil(3) * 4);
    for chunk in value.chunks(3) {
        encoded.push(char::from(ALPHABET[(chunk[0] >> 2) as usize]));
        encoded.push(char::from(
            ALPHABET[((chunk[0] & 0x03) << 4 | chunk.get(1).copied().unwrap_or(0) >> 4) as usize],
        ));
        if let Some(second) = chunk.get(1) {
            encoded.push(char::from(
                ALPHABET[((second & 0x0f) << 2 | chunk.get(2).copied().unwrap_or(0) >> 6) as usize],
            ));
        } else {
            encoded.push('=');
        }
        if let Some(third) = chunk.get(2) {
            encoded.push(char::from(ALPHABET[(third & 0x3f) as usize]));
        } else {
            encoded.push('=');
        }
    }
    encoded
}
