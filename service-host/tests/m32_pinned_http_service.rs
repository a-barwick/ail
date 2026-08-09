use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use ail_compiler::{
    CapabilityProvider, EvolutionCoverage, EvolutionSource, OutboundBatchCheck,
    OutboundCapabilityRequest, OutboundProviderOutcome, OutboundRequestHandle, RuntimeFault,
    RuntimeValue, Span,
};
use ail_service_host::{
    BODY_LIMIT_BYTES, MAXIMUM_TIMEOUT_MS, ServiceHost, canonical_config, canonical_environment,
    canonical_workspace,
};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

type ConfigMutation = Box<dyn Fn(&mut ail_service_host::PinnedServiceConfig)>;

#[derive(Debug, Default)]
struct ProviderTrace {
    starts: Vec<usize>,
    cancelled: Vec<usize>,
    max_active: usize,
}

#[derive(Default)]
struct Provider {
    active: BTreeMap<OutboundRequestHandle, usize>,
    outcomes: BTreeMap<OutboundRequestHandle, OutboundProviderOutcome>,
    order: VecDeque<usize>,
    fail_at: Option<usize>,
    fail_collect_at: Option<usize>,
    trace: Arc<Mutex<ProviderTrace>>,
}
impl Provider {
    fn ordered(order: impl IntoIterator<Item = usize>) -> Self {
        Self {
            order: order.into_iter().collect(),
            ..Self::default()
        }
    }
}
impl CapabilityProvider for Provider {
    fn supports(&self, receiver: &str, interface: &str) -> bool {
        receiver == "dependency" && interface == "DependencyClient"
    }
    fn call(
        &mut self,
        _: &str,
        _: &str,
        _: &str,
        _: &[RuntimeValue],
    ) -> Result<RuntimeValue, RuntimeFault> {
        unreachable!()
    }
    fn supports_outbound_batch(&self, _: &str, _: &str, _: &str) -> bool {
        true
    }
    fn start_outbound(
        &mut self,
        request: &OutboundCapabilityRequest,
    ) -> Result<OutboundRequestHandle, RuntimeFault> {
        let index = match request.arguments[0].field("key") {
            Some(RuntimeValue::Text(key)) => key.parse().unwrap(),
            _ => unreachable!(),
        };
        if self.fail_at == Some(index) {
            return Err(RuntimeFault::new(
                "TEST.START.FAILED",
                Span::empty(0),
                [("start", "ok")],
                [("start", "failed")],
            ));
        }
        let handle = OutboundRequestHandle(format!("h{index}"));
        self.active.insert(handle.clone(), index);
        {
            let mut trace = self.trace.lock().unwrap();
            trace.starts.push(index);
            trace.max_active = trace.max_active.max(self.active.len());
        }
        let result = if index == 1 {
            RuntimeValue::variant("batch_lookup.types.LookupOutcome", "NotFound", None)
        } else {
            RuntimeValue::variant(
                "batch_lookup.types.LookupOutcome",
                "Found",
                Some(RuntimeValue::Text(format!("v{index}"))),
            )
        };
        self.outcomes
            .insert(handle.clone(), OutboundProviderOutcome::Returned(result));
        Ok(handle)
    }
    fn check_outbound(
        &mut self,
        _: &[OutboundRequestHandle],
    ) -> Result<OutboundBatchCheck, RuntimeFault> {
        let position = self
            .order
            .iter()
            .position(|i| self.active.values().any(|v| v == i))
            .unwrap_or(0);
        let index = self
            .order
            .remove(position)
            .or_else(|| self.active.values().next().copied())
            .unwrap();
        Ok(OutboundBatchCheck {
            completed: vec![OutboundRequestHandle(format!("h{index}"))],
            cancelled: false,
        })
    }
    fn cancel_outbound(&mut self, handle: &OutboundRequestHandle) -> Result<(), RuntimeFault> {
        if let Some(index) = self.active.remove(handle) {
            self.trace.lock().unwrap().cancelled.push(index);
        }
        Ok(())
    }
    fn collect_outbound(
        &mut self,
        handle: &OutboundRequestHandle,
    ) -> Result<OutboundProviderOutcome, RuntimeFault> {
        let index = self.active[handle];
        if self.fail_collect_at == Some(index) {
            return Err(RuntimeFault::new(
                "TEST.COLLECT.FAILED",
                Span::empty(0),
                [("collect", "ok")],
                [("collect", "failed")],
            ));
        }
        self.active.remove(handle);
        Ok(self.outcomes.remove(handle).unwrap())
    }
}

fn host(provider: Provider) -> ServiceHost {
    let workspace = canonical_workspace().unwrap();
    let config = canonical_config(&workspace).unwrap();
    ServiceHost::new(&workspace, Box::new(provider), config).unwrap()
}
fn request(
    method: Method,
    path: &str,
    body: impl Into<Body>,
    content_type: Option<&str>,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(value) = content_type {
        builder = builder.header("content-type", value);
    }
    builder.body(body.into()).unwrap()
}
fn json(keys: usize, timeout: u128) -> String {
    format!(
        "{{\"requests\":[{}],\"timeout_ms\":{timeout}}}",
        (0..keys)
            .map(|i| format!("{{\"key\":\"{i}\"}}"))
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[test]
fn startup_rejects_every_mutated_pin() {
    let workspace = canonical_workspace().unwrap();
    let base = canonical_config(&workspace).unwrap();
    let mutations: Vec<(&str, ConfigMutation)> = vec![
        ("revision_id", Box::new(|c| c.revision_id = "r2".into())),
        (
            "source_set_digest",
            Box::new(|c| c.source_set_digest = "bad".into()),
        ),
        (
            "capability_environment_digest",
            Box::new(|c| c.capability_environment_digest = "bad".into()),
        ),
        (
            "entry_function",
            Box::new(|c| c.entry_function = "bad".into()),
        ),
        ("request_type", Box::new(|c| c.request_type = "bad".into())),
        ("result_type", Box::new(|c| c.result_type = "bad".into())),
        (
            "required_effect",
            Box::new(|c| c.required_effect = "bad".into()),
        ),
        ("list_bound", Box::new(|c| c.list_bound = 7)),
        ("concurrency_limit", Box::new(|c| c.concurrency_limit = 2)),
        (
            "maximum_timeout_ms",
            Box::new(|c| c.maximum_timeout_ms = 999),
        ),
    ];
    for (field, mutate) in mutations {
        let mut config = base.clone();
        mutate(&mut config);
        let error = ServiceHost::new(&workspace, Box::new(Provider::default()), config)
            .err()
            .unwrap();
        assert_eq!(error.field, field);
    }
}

#[tokio::test]
async fn eight_results_are_aligned_and_pinned() {
    let provider = Provider::ordered([2, 0, 1, 4, 3, 7, 5, 6]);
    let trace = Arc::clone(&provider.trace);
    let host = host(provider);
    let digest = host.pinned_config().source_set_digest.clone();
    let response = host
        .router()
        .oneshot(request(
            Method::POST,
            "/v1/lookups:batch",
            json(8, 100),
            Some("application/json; charset=utf-8"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(value["revision_id"], "r1");
    assert_eq!(value["source_set_digest"], digest);
    assert_eq!(value["outcomes"][1]["case"], "NotFound");
    assert_eq!(value["outcomes"][7]["value"], "v7");
    let record = &host.execution_records()[0];
    assert_eq!(record.calls.len(), 8);
    assert!(record.calls.iter().all(|c| c.start_order < 8));
    let mut completion_order = record
        .calls
        .iter()
        .map(|call| (call.completion_order.unwrap(), call.batch_index))
        .collect::<Vec<_>>();
    completion_order.sort_unstable();
    assert_eq!(
        completion_order
            .into_iter()
            .map(|(_, batch_index)| batch_index)
            .collect::<Vec<_>>(),
        [2, 0, 1, 4, 3, 7, 5, 6]
    );
    let trace = trace.lock().unwrap();
    assert_eq!(trace.starts, (0..8).collect::<Vec<_>>());
    assert_eq!(trace.max_active, 3);
}

#[tokio::test]
async fn invalid_http_inputs_do_zero_work() {
    let host = host(Provider::default());
    let cases = [
        (
            json(9, 100),
            Some("application/json"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json(1, 0),
            Some("application/json"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json(1, MAXIMUM_TIMEOUT_MS + 1),
            Some("application/json"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "{".into(),
            Some("application/json"),
            StatusCode::BAD_REQUEST,
        ),
        (
            "{\"requests\":[{\"key\":\"x\",\"revision\":\"r2\"}],\"timeout_ms\":1}".into(),
            Some("application/json"),
            StatusCode::BAD_REQUEST,
        ),
        (
            "{\"requests\":[{\"key\":1}],\"timeout_ms\":1}".into(),
            Some("application/json"),
            StatusCode::BAD_REQUEST,
        ),
        (
            "{\"requests\":\"not-a-list\",\"timeout_ms\":1}".into(),
            Some("application/json"),
            StatusCode::BAD_REQUEST,
        ),
        (
            "{\"requests\":[],\"timeout_ms\":1,\"revision_id\":\"r2\"}".into(),
            Some("application/json"),
            StatusCode::BAD_REQUEST,
        ),
        (
            "{\"requests\":[],\"timeout_ms\":1,\"capability\":\"x\"}".into(),
            Some("application/json"),
            StatusCode::BAD_REQUEST,
        ),
        (
            "{\"requests\":[],\"timeout_ms\":1,\"cancellation_token\":\"x\"}".into(),
            Some("application/json"),
            StatusCode::BAD_REQUEST,
        ),
        (json(1, 1), None, StatusCode::BAD_REQUEST),
        (json(1, 1), Some("text/plain"), StatusCode::BAD_REQUEST),
    ];
    for (body, content_type, status) in cases {
        let response = host
            .router()
            .oneshot(request(
                Method::POST,
                "/v1/lookups:batch",
                body,
                content_type,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    let response = host
        .router()
        .oneshot(request(
            Method::POST,
            "/v1/lookups:batch",
            "x".repeat(BODY_LIMIT_BYTES + 1),
            Some("application/json"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(host.execution_records().is_empty());
}

#[tokio::test]
async fn failure_is_fail_stop_and_routes_are_exact() {
    let provider = Provider {
        fail_at: Some(2),
        ..Provider::ordered([0, 1])
    };
    let trace = Arc::clone(&provider.trace);
    let host = host(provider);
    let response = host
        .router()
        .oneshot(request(
            Method::POST,
            "/v1/lookups:batch",
            json(8, 10),
            Some("application/json"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let records = host.execution_records();
    assert_eq!(
        records[0].failure_code.as_deref(),
        Some("TEST.START.FAILED")
    );
    assert_eq!(records[0].calls.len(), 2);
    assert!(records[0].calls.iter().all(|c| c.completion_order.is_none()
        && c.outcome.as_deref() == Some("Cancelled")
        && c.result.as_deref() == Some("Cancelled")));
    {
        let trace = trace.lock().unwrap();
        assert_eq!(trace.starts, [0, 1]);
        let mut cancelled = trace.cancelled.clone();
        cancelled.sort_unstable();
        assert_eq!(cancelled, [0, 1]);
    }
    assert_eq!(
        host.router()
            .oneshot(request(
                Method::GET,
                "/v1/lookups:batch",
                Body::empty(),
                None
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(
        host.router()
            .oneshot(request(Method::POST, "/other", Body::empty(), None))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn retaining_r2_does_not_move_the_pin() {
    let mut workspace = canonical_workspace().unwrap();
    let config = canonical_config(&workspace).unwrap();
    let host = ServiceHost::new(&workspace, Box::new(Provider::default()), config).unwrap();
    workspace
        .retain_revision(
            "r2",
            Some("r1".into()),
            vec![
                EvolutionSource::new(
                    "types.ail",
                    include_str!("../../compiler/examples/batch-lookup/types.ail"),
                ),
                EvolutionSource::new(
                    "service.ail",
                    include_str!("../../compiler/examples/batch-lookup/service.ail"),
                ),
            ],
            &canonical_environment(),
            EvolutionCoverage::default(),
        )
        .unwrap();
    let response = host
        .router()
        .oneshot(request(
            Method::POST,
            "/v1/lookups:batch",
            json(1, 10),
            Some("application/json"),
        ))
        .await
        .unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(value["revision_id"], "r1");
    assert_eq!(host.execution_records()[0].revision_id, "r1");
}

#[tokio::test]
async fn collect_failure_records_reported_completion_without_partial_success() {
    let provider = Provider {
        fail_collect_at: Some(0),
        ..Provider::ordered([0])
    };
    let trace = Arc::clone(&provider.trace);
    let host = host(provider);
    let response = host
        .router()
        .oneshot(request(
            Method::POST,
            "/v1/lookups:batch",
            json(8, 10),
            Some("application/json"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let records = host.execution_records();
    assert_eq!(
        records[0].failure_code.as_deref(),
        Some("TEST.COLLECT.FAILED")
    );
    assert_eq!(records[0].calls.len(), 3);
    assert_eq!(records[0].calls[0].completion_order, Some(0));
    assert!(records[0].calls[0].outcome.is_none());
    assert!(records[0].calls[0].result.is_none());
    assert!(
        records[0].calls[1..]
            .iter()
            .all(|call| call.completion_order.is_none())
    );
    let trace = trace.lock().unwrap();
    assert_eq!(trace.starts, [0, 1, 2]);
}
