//! Pinned HTTP host for the canonical M32 batch-lookup service.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use ail_compiler::{
    CancellationToken, CapabilityEnvironment, CapabilityInterface, CapabilityOperation,
    CapabilityProvider, EvolutionCoverage, EvolutionSource, EvolutionWorkspace, ExecutionResponse,
    ObservedCapabilityCall, OutboundCapabilityMetadata, OutboundProviderOutcome, RuntimeValue,
    source_digest,
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
pub const AUDIT_CAPACITY: usize = 256;
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
    pub catalog_digest: String,
    pub calls: Vec<CallRecord>,
    pub failure_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditUnavailable;

impl std::fmt::Display for AuditUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("execution audit is unavailable")
    }
}
impl std::error::Error for AuditUnavailable {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallRecord {
    pub batch_index: usize,
    pub start_order: usize,
    pub completion_order: Option<usize>,
    pub timeout_ms: u64,
    pub outcome: Option<OutboundProviderOutcome>,
    pub result: Option<RuntimeValue>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogDocument {
    entries: Vec<CatalogEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogEntry {
    key: String,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogError {
    message: String,
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl std::error::Error for CatalogError {}

/// Immutable operator-supplied lookup catalog used by the executable M32 host.
pub struct CatalogProvider {
    entries: BTreeMap<String, String>,
    catalog_digest: String,
    ready: BTreeMap<ail_compiler::OutboundRequestHandle, OutboundProviderOutcome>,
    next_handle: u64,
}

impl CatalogProvider {
    /// Parses one strict catalog document and rejects duplicate keys.
    ///
    /// # Errors
    /// Returns a catalog error for malformed JSON, unknown fields, or duplicate keys.
    pub fn from_json(json: &str) -> Result<Self, CatalogError> {
        let document =
            serde_json::from_str::<CatalogDocument>(json).map_err(|error| CatalogError {
                message: format!("invalid catalog: {error}"),
            })?;
        let mut entries = BTreeMap::new();
        for entry in document.entries {
            if entries.insert(entry.key.clone(), entry.value).is_some() {
                return Err(CatalogError {
                    message: format!("duplicate catalog key: {}", entry.key),
                });
            }
        }
        let canonical_entries = serde_json::to_string(&entries).map_err(|error| CatalogError {
            message: format!("cannot canonicalize catalog: {error}"),
        })?;
        Ok(Self {
            entries,
            catalog_digest: source_digest(&format!("ail.catalog.v1\0{canonical_entries}")),
            ready: BTreeMap::new(),
            next_handle: 0,
        })
    }

    /// Returns the digest of the parsed key/value semantics, independent of JSON formatting and
    /// entry order.
    #[must_use]
    pub fn catalog_digest(&self) -> &str {
        &self.catalog_digest
    }
}

/// Dependency provider whose immutable data snapshot identifies itself for response and audit
/// binding.
pub trait CatalogBoundProvider: CapabilityProvider + Send {
    fn catalog_digest(&self) -> &str;
}

impl CatalogBoundProvider for CatalogProvider {
    fn catalog_digest(&self) -> &str {
        self.catalog_digest()
    }
}

impl CapabilityProvider for CatalogProvider {
    fn supports(&self, receiver: &str, interface: &str) -> bool {
        receiver == "dependency" && interface == "DependencyClient"
    }

    fn call(
        &mut self,
        _: &str,
        _: &str,
        _: &str,
        _: &[RuntimeValue],
    ) -> Result<RuntimeValue, ail_compiler::RuntimeFault> {
        Err(catalog_fault("M32.CATALOG.ORDINARY_CALL"))
    }

    fn supports_outbound_batch(&self, receiver: &str, interface: &str, operation: &str) -> bool {
        receiver == "dependency" && interface == "DependencyClient" && operation == "fetch"
    }

    fn start_outbound(
        &mut self,
        request: &ail_compiler::OutboundCapabilityRequest,
    ) -> Result<ail_compiler::OutboundRequestHandle, ail_compiler::RuntimeFault> {
        let Some(RuntimeValue::Text(key)) = request
            .arguments
            .first()
            .and_then(|value| value.field("key"))
        else {
            return Err(catalog_fault("M32.CATALOG.INVALID_REQUEST"));
        };
        let outcome = self.entries.get(key).map_or_else(
            || RuntimeValue::variant(RESULT_TYPE, "NotFound", None),
            |value| {
                RuntimeValue::variant(
                    RESULT_TYPE,
                    "Found",
                    Some(RuntimeValue::Text(value.clone())),
                )
            },
        );
        let handle = ail_compiler::OutboundRequestHandle(format!("catalog-{}", self.next_handle));
        self.next_handle = self
            .next_handle
            .checked_add(1)
            .ok_or_else(|| catalog_fault("M32.CATALOG.HANDLE_EXHAUSTED"))?;
        self.ready
            .insert(handle.clone(), OutboundProviderOutcome::Returned(outcome));
        Ok(handle)
    }

    fn check_outbound(
        &mut self,
        handles: &[ail_compiler::OutboundRequestHandle],
    ) -> Result<ail_compiler::OutboundBatchCheck, ail_compiler::RuntimeFault> {
        Ok(ail_compiler::OutboundBatchCheck {
            completed: handles.first().cloned().into_iter().collect(),
            cancelled: false,
        })
    }

    fn cancel_outbound(
        &mut self,
        handle: &ail_compiler::OutboundRequestHandle,
    ) -> Result<(), ail_compiler::RuntimeFault> {
        self.ready.remove(handle);
        Ok(())
    }

    fn collect_outbound(
        &mut self,
        handle: &ail_compiler::OutboundRequestHandle,
    ) -> Result<OutboundProviderOutcome, ail_compiler::RuntimeFault> {
        self.ready
            .remove(handle)
            .ok_or_else(|| catalog_fault("M32.CATALOG.UNKNOWN_HANDLE"))
    }
}

fn catalog_fault(code: &'static str) -> ail_compiler::RuntimeFault {
    ail_compiler::RuntimeFault::new(
        code,
        ail_compiler::Span::empty(0),
        std::iter::empty::<(&str, &str)>(),
        std::iter::empty::<(&str, &str)>(),
    )
}

struct Inner {
    workspace: EvolutionWorkspace,
    provider: Mutex<Box<dyn CatalogBoundProvider>>,
    config: PinnedServiceConfig,
    catalog_digest: String,
    audit: Mutex<AuditStore>,
    token_counter: AtomicU64,
}

#[derive(Default)]
struct AuditStore {
    next_reservation: u64,
    entries: Vec<(u64, Option<ExecutionRecord>)>,
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
        provider: Box<dyn CatalogBoundProvider>,
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
        let catalog_digest = provider.catalog_digest().to_owned();
        Ok(Self(Arc::new(Inner {
            workspace: pinned_workspace,
            provider: Mutex::new(provider),
            config,
            catalog_digest,
            audit: Mutex::new(AuditStore::default()),
            token_counter: AtomicU64::new(1),
        })))
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route("/v1/lookups:batch", post(handle))
            .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES))
            .with_state(self.clone())
    }
    /// Returns completed execution records in admission order.
    ///
    /// # Errors
    /// Returns `AuditUnavailable` when the process-local audit lock is poisoned.
    pub fn execution_records(&self) -> Result<Vec<ExecutionRecord>, AuditUnavailable> {
        self.0
            .audit
            .lock()
            .map(|audit| {
                audit
                    .entries
                    .iter()
                    .filter_map(|(_, record)| record.clone())
                    .collect()
            })
            .map_err(|_| AuditUnavailable)
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
    timeout_ms: serde_json::Number,
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
    catalog_digest: String,
    outcomes: Vec<JsonOutcome>,
}
#[derive(Serialize)]
#[serde(tag = "case")]
enum JsonOutcome {
    Found { key: String, value: String },
    NotFound { key: String },
    Unavailable { key: String },
    TimedOut { key: String },
    Cancelled { key: String },
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
    let Some(timeout_ms) = input.timeout_ms.as_u64().map(u128::from) else {
        return unprocessable("timeout_bounds");
    };
    if timeout_ms == 0 || timeout_ms > host.0.config.maximum_timeout_ms {
        return unprocessable("timeout_bounds");
    }

    let audit_slot = match reserve_audit(&host) {
        Ok(slot) => slot,
        Err(AuditReservationError::Full) => return service_unavailable("audit_capacity"),
        Err(AuditReservationError::Unavailable) => {
            return service_unavailable("audit_unavailable");
        }
    };
    let keys = input
        .requests
        .into_iter()
        .map(|request| request.key)
        .collect::<Vec<_>>();
    let requests = RuntimeValue::list(
        keys.iter()
            .cloned()
            .map(|key| RuntimeValue::record(REQUEST_TYPE, [("key", RuntimeValue::Text(key))])),
    );
    let token = host.0.token_counter.fetch_add(1, Ordering::Relaxed);
    let arguments = vec![
        requests,
        RuntimeValue::Int(timeout_ms),
        RuntimeValue::Cancellation(CancellationToken::new(format!("m32-{token}"))),
    ];
    let response = if let Ok(mut provider) = host.0.provider.lock() {
        host.0.workspace.execute(
            &host.0.config.revision_id,
            &host.0.config.entry_function,
            arguments,
            provider.as_mut(),
        )
    } else {
        if !release_audit(&host, audit_slot) {
            return service_unavailable("audit_unavailable");
        }
        return gateway_error("host_lock_failure");
    };
    let (calls, failure_code) = match &response {
        ExecutionResponse::Completed(value) => (&value.calls, None),
        ExecutionResponse::Failed(value) => (&value.calls, Some(value.fault.code.to_owned())),
    };
    let record = ExecutionRecord {
        revision_id: host.0.config.revision_id.clone(),
        source_set_digest: host.0.config.source_set_digest.clone(),
        catalog_digest: host.0.catalog_digest.clone(),
        calls: calls.iter().filter_map(call_record).collect(),
        failure_code,
    };
    if !complete_audit(&host, audit_slot, record) {
        return service_unavailable("audit_unavailable");
    }
    let ExecutionResponse::Completed(success) = response else {
        return gateway_error("execution_failed");
    };
    let RuntimeValue::List(values) = success.value else {
        return gateway_error("invalid_result");
    };
    if values.len() != keys.len() {
        return gateway_error("invalid_result");
    }
    let Some(outcomes) = keys
        .into_iter()
        .zip(values.iter())
        .map(|(key, value)| json_outcome(key, value))
        .collect::<Option<Vec<_>>>()
    else {
        return gateway_error("invalid_result");
    };
    (
        StatusCode::OK,
        axum::Json(BatchResponse {
            revision_id: host.0.config.revision_id.clone(),
            source_set_digest: host.0.config.source_set_digest.clone(),
            catalog_digest: host.0.catalog_digest.clone(),
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
fn service_unavailable(error: &'static str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(ErrorBody { error }),
    )
        .into_response()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuditReservationError {
    Full,
    Unavailable,
}

fn reserve_audit(host: &ServiceHost) -> Result<u64, AuditReservationError> {
    let mut audit = host
        .0
        .audit
        .lock()
        .map_err(|_| AuditReservationError::Unavailable)?;
    if audit.entries.len() >= AUDIT_CAPACITY {
        return Err(AuditReservationError::Full);
    }
    let reservation = audit.next_reservation;
    audit.next_reservation = audit.next_reservation.wrapping_add(1);
    audit.entries.push((reservation, None));
    Ok(reservation)
}

fn complete_audit(host: &ServiceHost, reservation: u64, record: ExecutionRecord) -> bool {
    let Ok(mut audit) = host.0.audit.lock() else {
        return false;
    };
    let Some((_, reserved_record)) = audit
        .entries
        .iter_mut()
        .find(|(reserved, _)| *reserved == reservation)
    else {
        return false;
    };
    if reserved_record.is_some() {
        return false;
    }
    *reserved_record = Some(record);
    true
}

fn release_audit(host: &ServiceHost, reservation: u64) -> bool {
    let Ok(mut audit) = host.0.audit.lock() else {
        return false;
    };
    let Some(index) = audit
        .entries
        .iter()
        .position(|(reserved, record)| *reserved == reservation && record.is_none())
    else {
        return false;
    };
    audit.entries.remove(index);
    true
}

fn json_outcome(key: String, value: &RuntimeValue) -> Option<JsonOutcome> {
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
            key,
            value: value.clone(),
        }),
        ("NotFound", None) => Some(JsonOutcome::NotFound { key }),
        ("Unavailable", None) => Some(JsonOutcome::Unavailable { key }),
        ("TimedOut", None) => Some(JsonOutcome::TimedOut { key }),
        ("Cancelled", None) => Some(JsonOutcome::Cancelled { key }),
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
        outcome: outbound.outcome.clone(),
        result: call.result.clone(),
    })
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

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::{CatalogProvider, ServiceHost, canonical_config, canonical_workspace};

    #[tokio::test]
    async fn poisoned_audit_lock_is_visible_and_fails_closed_before_execution() {
        let provider = CatalogProvider::from_json(r#"{"entries":[]}"#).unwrap();
        let workspace = canonical_workspace().unwrap();
        let config = canonical_config(&workspace).unwrap();
        let host = ServiceHost::new(&workspace, Box::new(provider), config).unwrap();
        let poison_host = host.clone();
        assert!(
            catch_unwind(AssertUnwindSafe(move || {
                let _guard = poison_host.0.audit.lock().unwrap();
                panic!("synthetic audit lock poison");
            }))
            .is_err()
        );

        assert!(host.execution_records().is_err());
        let response = host
            .router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/lookups:batch")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"requests":[{"key":"must-not-run"}],"timeout_ms":1}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn poisoned_provider_lock_releases_pre_execution_audit_reservation() {
        let provider = CatalogProvider::from_json(r#"{"entries":[]}"#).unwrap();
        let workspace = canonical_workspace().unwrap();
        let config = canonical_config(&workspace).unwrap();
        let host = ServiceHost::new(&workspace, Box::new(provider), config).unwrap();
        let poison_host = host.clone();
        assert!(
            catch_unwind(AssertUnwindSafe(move || {
                let _guard = poison_host.0.provider.lock().unwrap();
                panic!("synthetic provider lock poison");
            }))
            .is_err()
        );

        for _ in 0..2 {
            let response = host
                .router()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/lookups:batch")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            r#"{"requests":[{"key":"must-not-run"}],"timeout_ms":1}"#,
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        }
        assert!(host.execution_records().unwrap().is_empty());
    }
}
