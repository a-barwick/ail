//! Pinned HTTP host for the canonical M32 batch-lookup service.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use ail_compiler::{
    CancellationToken, CapabilityEnvironment, CapabilityInterface, CapabilityOperation,
    CapabilityProvider, EvolutionCoverage, EvolutionSource, EvolutionWorkspace, ExecutionResponse,
    ObservedCapabilityCall, OutboundCapabilityMetadata, OutboundProviderOutcome, RuntimeValue,
};
use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State, rejection::BytesRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};

pub const ENTRY_FUNCTION: &str = "batch_lookup.service.lookup_batch";
pub const REQUEST_TYPE: &str = "batch_lookup.types.LookupRequest";
pub const RESULT_TYPE: &str = "batch_lookup.types.LookupOutcome";
pub const REQUIRED_EFFECT: &str = "dependency.fetch";
pub const LIST_BOUND: u128 = 8;
pub const CONCURRENCY_LIMIT: u128 = 3;
pub const MAXIMUM_TIMEOUT_MS: u128 = 1_000;
pub const BODY_LIMIT_BYTES: usize = 16 * 1024;
pub const PINNED_SOURCE_SET_DIGEST: &str =
    "sha256:2b9ad64d61250d178d5e2217bee078ab46a668c7105db594cc2442d50ea3bc75";
pub const PINNED_CAPABILITY_ENVIRONMENT_DIGEST: &str =
    "sha256:4a81ab035f4674c115900a414e217c19e3e71eeb6c98f6fc995136adce1ebc59";

const TYPES: &str = include_str!("../../compiler/examples/batch-lookup/types.ail");
const SERVICE: &str = include_str!("../../compiler/examples/batch-lookup/service.ail");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedServiceConfig {
    pub revision_id: String,
    pub source_set_digest: String,
    pub capability_environment_digest: String,
    pub entry_function: String,
    pub request_type: String,
    pub result_type: String,
    pub list_bound: u128,
    pub concurrency_limit: u128,
    pub required_effect: String,
    pub maximum_timeout_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupError {
    pub field: &'static str,
    pub expected: String,
    pub actual: String,
}

impl std::fmt::Display for StartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "pinned {} mismatch: expected {}, got {}",
            self.field, self.expected, self.actual
        )
    }
}
impl std::error::Error for StartupError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRecord {
    pub revision_id: String,
    pub source_set_digest: String,
    pub calls: Vec<CallRecord>,
    pub failure_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallRecord {
    pub batch_index: usize,
    pub start_order: usize,
    pub completion_order: Option<usize>,
    pub timeout_ms: u64,
    pub outcome: Option<String>,
    pub result: Option<String>,
}

struct Inner {
    workspace: EvolutionWorkspace,
    provider: Mutex<Box<dyn CapabilityProvider + Send>>,
    config: PinnedServiceConfig,
    records: Mutex<Vec<ExecutionRecord>>,
    token_counter: AtomicU64,
}

#[derive(Clone)]
pub struct ServiceHost(Arc<Inner>);

impl ServiceHost {
    /// Validates every pinned field before returning a host from which a router can be made.
    ///
    /// # Errors
    /// Returns the first configured or compiler-inspected field that differs from the M32 pins.
    #[allow(clippy::too_many_lines)]
    pub fn new(
        workspace: &EvolutionWorkspace,
        provider: Box<dyn CapabilityProvider + Send>,
        config: PinnedServiceConfig,
    ) -> Result<Self, StartupError> {
        exact("revision_id", "r1", &config.revision_id)?;
        exact("entry_function", ENTRY_FUNCTION, &config.entry_function)?;
        exact("request_type", REQUEST_TYPE, &config.request_type)?;
        exact("result_type", RESULT_TYPE, &config.result_type)?;
        exact("required_effect", REQUIRED_EFFECT, &config.required_effect)?;
        equal("list_bound", LIST_BOUND, config.list_bound)?;
        equal(
            "concurrency_limit",
            CONCURRENCY_LIMIT,
            config.concurrency_limit,
        )?;
        equal(
            "maximum_timeout_ms",
            MAXIMUM_TIMEOUT_MS,
            config.maximum_timeout_ms,
        )?;
        let pinned_workspace = {
            let revision = workspace
                .revision(&config.revision_id)
                .ok_or_else(|| startup("revision_id", &config.revision_id, "not retained"))?;
            exact(
                "source_set_digest",
                &revision.source_set_digest,
                &config.source_set_digest,
            )?;
            exact(
                "capability_environment_digest",
                &revision.capability_environment_digest,
                &config.capability_environment_digest,
            )?;
            let inspection = workspace
                .inspect_function(&config.revision_id, &config.entry_function)
                .map_err(|error| startup("entry_function", ENTRY_FUNCTION, error.code))?;
            let request = inspection
                .parameters
                .first()
                .and_then(|p| p.bounded_list.as_ref())
                .ok_or_else(|| startup("request_type", REQUEST_TYPE, "missing bounded request"))?;
            exact("request_type", REQUEST_TYPE, &request.element_type)?;
            equal("list_bound", LIST_BOUND, request.max_length)?;
            let result = inspection
                .result_list
                .as_ref()
                .ok_or_else(|| startup("result_type", RESULT_TYPE, "missing bounded result"))?;
            exact("result_type", RESULT_TYPE, &result.element_type)?;
            equal("list_bound", LIST_BOUND, result.max_length)?;
            if inspection.effects.as_slice() != [REQUIRED_EFFECT] {
                return Err(startup(
                    "required_effect",
                    REQUIRED_EFFECT,
                    &inspection.effects.join(","),
                ));
            }
            let map = inspection
                .bounded_parallel_maps
                .first()
                .filter(|_| inspection.bounded_parallel_maps.len() == 1)
                .ok_or_else(|| {
                    startup(
                        "concurrency_limit",
                        "one bounded map",
                        "missing or multiple",
                    )
                })?;
            equal(
                "concurrency_limit",
                CONCURRENCY_LIMIT,
                map.concurrency_limit,
            )?;
            exact("required_effect", REQUIRED_EFFECT, &map.effect)?;
            exact(
                "capability_environment_digest",
                &config.capability_environment_digest,
                &map.capability_environment_digest,
            )?;
            let outbound = inspection
                .outbound_requests
                .first()
                .filter(|_| inspection.outbound_requests.len() == 1)
                .ok_or_else(|| {
                    startup(
                        "required_effect",
                        "one outbound request",
                        "missing or multiple",
                    )
                })?;
            equal(
                "maximum_timeout_ms",
                MAXIMUM_TIMEOUT_MS,
                outbound.maximum_timeout_ms,
            )?;
            workspace.clone()
        };
        Ok(Self(Arc::new(Inner {
            workspace: pinned_workspace,
            provider: Mutex::new(provider),
            config,
            records: Mutex::new(Vec::new()),
            token_counter: AtomicU64::new(1),
        })))
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route("/v1/lookups:batch", post(handle))
            .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES))
            .with_state(self.clone())
    }
    #[must_use]
    pub fn execution_records(&self) -> Vec<ExecutionRecord> {
        self.0
            .records
            .lock()
            .map_or_else(|_| Vec::new(), |v| v.clone())
    }
    #[must_use]
    pub fn pinned_config(&self) -> &PinnedServiceConfig {
        &self.0.config
    }
}

fn startup(field: &'static str, expected: &str, actual: &str) -> StartupError {
    StartupError {
        field,
        expected: expected.into(),
        actual: actual.into(),
    }
}
fn exact(field: &'static str, expected: &str, actual: &str) -> Result<(), StartupError> {
    if expected == actual {
        Ok(())
    } else {
        Err(startup(field, expected, actual))
    }
}
fn equal(field: &'static str, expected: u128, actual: u128) -> Result<(), StartupError> {
    exact(field, &expected.to_string(), &actual.to_string())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchRequest {
    requests: Vec<LookupRequest>,
    timeout_ms: u128,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LookupRequest {
    key: String,
}

#[derive(Serialize)]
struct BatchResponse {
    revision_id: String,
    source_set_digest: String,
    outcomes: Vec<JsonOutcome>,
}
#[derive(Serialize)]
#[serde(tag = "case")]
enum JsonOutcome {
    Found { value: String },
    NotFound,
    Unavailable,
    TimedOut,
    Cancelled,
}
#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
}

async fn handle(
    State(host): State<ServiceHost>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    if !json_content_type(&headers) {
        return bad_request("invalid_content_type");
    }
    let Ok(body) = body else {
        return bad_request("invalid_body");
    };
    let Ok(input) = serde_json::from_slice::<BatchRequest>(&body) else {
        return bad_request("invalid_json");
    };
    if input.requests.len() > usize::try_from(LIST_BOUND).unwrap_or(usize::MAX) {
        return unprocessable("request_limit");
    }
    if input.timeout_ms == 0 || input.timeout_ms > host.0.config.maximum_timeout_ms {
        return unprocessable("timeout_bounds");
    }

    let requests = RuntimeValue::list(input.requests.into_iter().map(|request| {
        RuntimeValue::record(REQUEST_TYPE, [("key", RuntimeValue::Text(request.key))])
    }));
    let token = host.0.token_counter.fetch_add(1, Ordering::Relaxed);
    let arguments = vec![
        requests,
        RuntimeValue::Int(input.timeout_ms),
        RuntimeValue::Cancellation(CancellationToken::new(format!("m32-{token}"))),
    ];
    let response = match host.0.provider.lock() {
        Ok(mut provider) => host.0.workspace.execute(
            &host.0.config.revision_id,
            &host.0.config.entry_function,
            arguments,
            provider.as_mut(),
        ),
        Err(_) => return gateway_error("host_lock_failure"),
    };
    let (calls, failure_code) = match &response {
        ExecutionResponse::Completed(value) => (&value.calls, None),
        ExecutionResponse::Failed(value) => (&value.calls, Some(value.fault.code.to_owned())),
    };
    let record = ExecutionRecord {
        revision_id: host.0.config.revision_id.clone(),
        source_set_digest: host.0.config.source_set_digest.clone(),
        calls: calls.iter().filter_map(call_record).collect(),
        failure_code,
    };
    if let Ok(mut records) = host.0.records.lock() {
        records.push(record);
    }
    let ExecutionResponse::Completed(success) = response else {
        return gateway_error("execution_failed");
    };
    let RuntimeValue::List(values) = success.value else {
        return gateway_error("invalid_result");
    };
    let Some(outcomes) = values.iter().map(json_outcome).collect::<Option<Vec<_>>>() else {
        return gateway_error("invalid_result");
    };
    (
        StatusCode::OK,
        axum::Json(BatchResponse {
            revision_id: host.0.config.revision_id.clone(),
            source_set_digest: host.0.config.source_set_digest.clone(),
            outcomes,
        }),
    )
        .into_response()
}

fn json_content_type(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            let mut parts = v.split(';');
            parts
                .next()
                .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
                && parts.all(|p| p.trim().to_ascii_lowercase().starts_with("charset="))
        })
}
fn bad_request(error: &'static str) -> Response {
    (StatusCode::BAD_REQUEST, axum::Json(ErrorBody { error })).into_response()
}
fn unprocessable(error: &'static str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        axum::Json(ErrorBody { error }),
    )
        .into_response()
}
fn gateway_error(error: &'static str) -> Response {
    (StatusCode::BAD_GATEWAY, axum::Json(ErrorBody { error })).into_response()
}

fn json_outcome(value: &RuntimeValue) -> Option<JsonOutcome> {
    let RuntimeValue::Variant {
        type_name,
        case,
        payload,
    } = value
    else {
        return None;
    };
    if type_name != RESULT_TYPE {
        return None;
    }
    match (case.as_str(), payload.as_deref()) {
        ("Found", Some(RuntimeValue::Text(value))) => Some(JsonOutcome::Found {
            value: value.clone(),
        }),
        ("NotFound", None) => Some(JsonOutcome::NotFound),
        ("Unavailable", None) => Some(JsonOutcome::Unavailable),
        ("TimedOut", None) => Some(JsonOutcome::TimedOut),
        ("Cancelled", None) => Some(JsonOutcome::Cancelled),
        _ => None,
    }
}
fn call_record(call: &ObservedCapabilityCall) -> Option<CallRecord> {
    let outbound = call.outbound.as_ref()?;
    Some(CallRecord {
        batch_index: outbound.batch_index?,
        start_order: outbound.start_order?,
        completion_order: outbound.completion_order,
        timeout_ms: outbound.timeout_ms,
        outcome: outbound.outcome.as_ref().map(outcome_name),
        result: call.result.as_ref().map(value_name),
    })
}
fn outcome_name(value: &OutboundProviderOutcome) -> String {
    match value {
        OutboundProviderOutcome::Returned(v) => value_name(v),
        OutboundProviderOutcome::TimedOut => "TimedOut".into(),
        OutboundProviderOutcome::Cancelled => "Cancelled".into(),
    }
}
fn value_name(value: &RuntimeValue) -> String {
    match value {
        RuntimeValue::Variant { case, .. } => case.clone(),
        _ => value.type_name().into(),
    }
}

#[must_use]
pub fn canonical_environment() -> CapabilityEnvironment {
    let mut interface = CapabilityInterface::new();
    interface.insert_operation(
        "fetch",
        CapabilityOperation::outbound(
            [REQUEST_TYPE, "Int", "Cancellation"],
            RESULT_TYPE,
            OutboundCapabilityMetadata {
                timeout_argument_index: 1,
                cancellation_argument_index: 2,
                maximum_timeout_ms: MAXIMUM_TIMEOUT_MS,
                timed_out_case_identity: "timed-out".into(),
                cancelled_case_identity: "cancelled".into(),
            },
        ),
    );
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("DependencyClient", interface);
    environment
}

/// Builds the canonical retained r1 workspace.
///
/// # Errors
/// Returns compiler build diagnostics if the embedded canonical source is invalid.
pub fn canonical_workspace() -> Result<EvolutionWorkspace, ail_compiler::EvolutionBuildFailure> {
    EvolutionWorkspace::new(
        "batch-lookup",
        "r1",
        vec![
            EvolutionSource::new("types.ail", TYPES),
            EvolutionSource::new("service.ail", SERVICE),
        ],
        &canonical_environment(),
        EvolutionCoverage::default(),
    )
}

/// Constructs the exact M32 pin set and confirms that its revision is retained.
///
/// # Errors
/// Returns an error when r1 is not retained in the supplied workspace.
pub fn canonical_config(
    workspace: &EvolutionWorkspace,
) -> Result<PinnedServiceConfig, StartupError> {
    workspace
        .revision("r1")
        .ok_or_else(|| startup("revision_id", "r1", "not retained"))?;
    Ok(PinnedServiceConfig {
        revision_id: "r1".into(),
        source_set_digest: PINNED_SOURCE_SET_DIGEST.into(),
        capability_environment_digest: PINNED_CAPABILITY_ENVIRONMENT_DIGEST.into(),
        entry_function: ENTRY_FUNCTION.into(),
        request_type: REQUEST_TYPE.into(),
        result_type: RESULT_TYPE.into(),
        list_bound: LIST_BOUND,
        concurrency_limit: CONCURRENCY_LIMIT,
        required_effect: REQUIRED_EFFECT.into(),
        maximum_timeout_ms: MAXIMUM_TIMEOUT_MS,
    })
}
