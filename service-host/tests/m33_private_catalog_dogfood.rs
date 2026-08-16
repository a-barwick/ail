use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use ail_compiler::{
    CapabilityProvider, OutboundBatchCheck, OutboundCapabilityRequest, OutboundProviderOutcome,
    OutboundRequestHandle, RuntimeFault, RuntimeValue,
};
use ail_service_host::{
    AUDIT_CAPACITY, CatalogBoundProvider, CatalogProvider, RESULT_TYPE, ServiceHost,
    canonical_config, canonical_workspace,
};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn request(body: impl Into<Body>) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/v1/lookups:batch")
        .header("content-type", "application/json")
        .body(body.into())
        .unwrap()
}

fn catalog_host(provider: CatalogProvider) -> ServiceHost {
    let workspace = canonical_workspace().unwrap();
    let config = canonical_config(&workspace).unwrap();
    ServiceHost::new(&workspace, Box::new(provider), config).unwrap()
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

#[tokio::test]
async fn semantic_catalog_digest_is_stable_bound_and_snapshot_immutable() {
    let first = r#"{
        "entries": [
            {"key": "ail.full-check", "value": "cargo test --workspace"},
            {"key": "ail.repo-path", "value": "/synthetic/AIL"}
        ]
    }"#;
    let equivalent = r#"{"entries":[{"value":"/synthetic/AIL","key":"ail.repo-path"},{"value":"cargo test --workspace","key":"ail.full-check"}]}"#;
    let changed = r#"{"entries":[{"key":"ail.full-check","value":"cargo test --all"},{"key":"ail.repo-path","value":"/synthetic/AIL"}]}"#;

    let first_provider = CatalogProvider::from_json(first).unwrap();
    let first_digest = first_provider.catalog_digest().to_owned();
    assert_eq!(
        first_digest,
        CatalogProvider::from_json(equivalent)
            .unwrap()
            .catalog_digest()
    );
    assert_ne!(
        first_digest,
        CatalogProvider::from_json(changed)
            .unwrap()
            .catalog_digest()
    );

    let path = temporary_catalog_path();
    fs::write(&path, first).unwrap();
    let running_provider = CatalogProvider::from_json(&fs::read_to_string(&path).unwrap()).unwrap();
    let running_digest = running_provider.catalog_digest().to_owned();
    let host = catalog_host(running_provider);
    fs::write(&path, changed).unwrap();

    let response = host
        .router()
        .oneshot(request(
            r#"{"requests":[{"key":"ail.full-check"}],"timeout_ms":100}"#,
        ))
        .await
        .unwrap();
    fs::remove_file(&path).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["catalog_digest"], running_digest);
    assert_eq!(body["outcomes"][0]["value"], "cargo test --workspace");
    let records = host.execution_records().unwrap();
    assert_eq!(records[0].catalog_digest, running_digest);
    assert_eq!(records[0].catalog_digest, body["catalog_digest"]);
}

#[tokio::test]
async fn every_closed_outcome_carries_its_original_key_in_input_order() {
    let host = outcome_host();
    let response = host
        .router()
        .oneshot(request(
            r#"{"requests":[{"key":"found"},{"key":"missing"},{"key":"unavailable"},{"key":"timed"},{"key":"cancelled"},{"key":"found"}],"timeout_ms":100}"#,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let outcomes = body["outcomes"].as_array().unwrap();
    assert_eq!(outcomes.len(), 6);
    assert_eq!(
        outcomes
            .iter()
            .map(|outcome| outcome["key"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "found",
            "missing",
            "unavailable",
            "timed",
            "cancelled",
            "found"
        ]
    );
    assert_eq!(
        outcomes
            .iter()
            .map(|outcome| outcome["case"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "Found",
            "NotFound",
            "Unavailable",
            "TimedOut",
            "Cancelled",
            "Found"
        ]
    );
    assert_eq!(outcomes[0]["value"], "synthetic-value");
    assert_eq!(outcomes[5]["value"], "synthetic-value");
}

#[tokio::test]
async fn audit_capacity_rejects_the_257th_valid_request_before_provider_work() {
    let starts = Arc::new(AtomicUsize::new(0));
    let provider = CountingProvider {
        starts: Arc::clone(&starts),
    };
    let workspace = canonical_workspace().unwrap();
    let config = canonical_config(&workspace).unwrap();
    let host = ServiceHost::new(&workspace, Box::new(provider), config).unwrap();

    for _ in 0..AUDIT_CAPACITY {
        let response = host
            .router()
            .oneshot(request(r#"{"requests":[],"timeout_ms":1}"#))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let response = host
        .router()
        .oneshot(request(
            r#"{"requests":[{"key":"must-not-start"}],"timeout_ms":1}"#,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = response_json(response).await;
    assert_eq!(body["error"], "audit_capacity");
    assert_eq!(starts.load(Ordering::SeqCst), 0);
    assert_eq!(host.execution_records().unwrap().len(), AUDIT_CAPACITY);

    let malformed = host.router().oneshot(request("{")).await.unwrap();
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    let out_of_bounds = host
        .router()
        .oneshot(request(r#"{"requests":[],"timeout_ms":0}"#))
        .await
        .unwrap();
    assert_eq!(out_of_bounds.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn concurrent_valid_requests_cannot_overbook_audit_capacity() {
    let provider = CatalogProvider::from_json(r#"{"entries":[]}"#).unwrap();
    let host = catalog_host(provider);
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..=AUDIT_CAPACITY {
        let host = host.clone();
        tasks.spawn(async move {
            host.router()
                .oneshot(request(r#"{"requests":[],"timeout_ms":1}"#))
                .await
                .unwrap()
                .status()
        });
    }
    let mut ok = 0;
    let mut unavailable = 0;
    while let Some(status) = tasks.join_next().await {
        match status.unwrap() {
            StatusCode::OK => ok += 1,
            StatusCode::SERVICE_UNAVAILABLE => unavailable += 1,
            other => panic!("unexpected status {other}"),
        }
    }
    assert_eq!(ok, AUDIT_CAPACITY);
    assert_eq!(unavailable, 1);
    assert_eq!(host.execution_records().unwrap().len(), AUDIT_CAPACITY);
}

fn temporary_catalog_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ail-m33-synthetic-catalog-{}-{nonce}.json",
        std::process::id()
    ))
}

struct CountingProvider {
    starts: Arc<AtomicUsize>,
}

impl CatalogBoundProvider for CountingProvider {
    fn catalog_digest(&self) -> &'static str {
        "sha256:synthetic-counting-catalog"
    }
}

impl CapabilityProvider for CountingProvider {
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
        _: &OutboundCapabilityRequest,
    ) -> Result<OutboundRequestHandle, RuntimeFault> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        unreachable!("a full audit log must reject before provider work")
    }
}

#[derive(Default)]
struct OutcomeProvider {
    next_handle: usize,
    ready: BTreeMap<OutboundRequestHandle, OutboundProviderOutcome>,
}

impl CatalogBoundProvider for OutcomeProvider {
    fn catalog_digest(&self) -> &'static str {
        "sha256:synthetic-outcome-catalog"
    }
}

impl CapabilityProvider for OutcomeProvider {
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
        let RuntimeValue::Text(key) = request.arguments[0].field("key").unwrap() else {
            unreachable!()
        };
        let outcome = match key.as_str() {
            "found" => OutboundProviderOutcome::Returned(RuntimeValue::variant(
                RESULT_TYPE,
                "Found",
                Some(RuntimeValue::Text("synthetic-value".into())),
            )),
            "missing" => OutboundProviderOutcome::Returned(RuntimeValue::variant(
                RESULT_TYPE,
                "NotFound",
                None,
            )),
            "unavailable" => OutboundProviderOutcome::Returned(RuntimeValue::variant(
                RESULT_TYPE,
                "Unavailable",
                None,
            )),
            "timed" => OutboundProviderOutcome::TimedOut,
            "cancelled" => OutboundProviderOutcome::Cancelled,
            other => panic!("unexpected synthetic key {other}"),
        };
        let handle = OutboundRequestHandle(format!("m33-{}", self.next_handle));
        self.next_handle += 1;
        self.ready.insert(handle.clone(), outcome);
        Ok(handle)
    }

    fn check_outbound(
        &mut self,
        handles: &[OutboundRequestHandle],
    ) -> Result<OutboundBatchCheck, RuntimeFault> {
        Ok(OutboundBatchCheck {
            completed: handles.first().cloned().into_iter().collect(),
            cancelled: false,
        })
    }

    fn cancel_outbound(&mut self, handle: &OutboundRequestHandle) -> Result<(), RuntimeFault> {
        self.ready.remove(handle);
        Ok(())
    }

    fn collect_outbound(
        &mut self,
        handle: &OutboundRequestHandle,
    ) -> Result<OutboundProviderOutcome, RuntimeFault> {
        Ok(self.ready.remove(handle).unwrap())
    }
}

fn outcome_host() -> ServiceHost {
    let workspace = canonical_workspace().unwrap();
    let config = canonical_config(&workspace).unwrap();
    ServiceHost::new(&workspace, Box::new(OutcomeProvider::default()), config).unwrap()
}
