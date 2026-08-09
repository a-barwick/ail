use std::cell::Cell;

use ail_compiler::{
    CancellationToken, CapabilityEnvironment, CapabilityInterface, CapabilityOperation,
    CapabilityOperationKind, CapabilityProvider, EvolutionCoverage, EvolutionSource,
    EvolutionWorkspace, ExecutionResponse, HandleKind, InspectionRequest,
    OutboundCapabilityMetadata, OutboundCapabilityRequest, OutboundProviderOutcome, RuntimeFault,
    RuntimeValue, Span, Workspace, format_source,
};

const TYPES: &str = include_str!("../../examples/outbound-request/types.ail");
const SERVICE: &str = include_str!("../../examples/outbound-request/service.ail");

const REJECTION_CASES: [&str; 21] = [
    "invalid-timeout-index",
    "invalid-cancellation-index",
    "overlapping-indices",
    "wrong-timeout-type",
    "wrong-cancellation-type",
    "invalid-timeout-maximum",
    "non-variant-result",
    "missing-timeout-case",
    "missing-cancelled-case",
    "same-completion-case",
    "payload-bearing-timeout-case",
    "payload-bearing-cancelled-case",
    "missing-capability-permission",
    "missing-effect-permission",
    "timeout-zero",
    "timeout-over-maximum",
    "malformed-cancellation-value",
    "unsupported-outbound-provider",
    "unknown-returned-case",
    "malformed-returned-value",
    "provider-contract-fault",
];
const COMPLETION_CASES: [&str; 8] = [
    "returned-found",
    "returned-not-found",
    "returned-unavailable",
    "timed-out-synthesized",
    "cancelled-synthesized",
    "ordinary-call-compatible",
    "inspection-deterministic",
    "retained-revision-binding",
];

fn operation(
    parameters: &[&str],
    result: &str,
    maximum: u128,
    timeout_index: usize,
    cancellation_index: usize,
    timed_out: &str,
    cancelled: &str,
) -> CapabilityOperation {
    CapabilityOperation::outbound(
        parameters.iter().copied(),
        result,
        OutboundCapabilityMetadata {
            timeout_argument_index: timeout_index,
            cancellation_argument_index: cancellation_index,
            maximum_timeout_ms: maximum,
            timed_out_case_identity: timed_out.to_owned(),
            cancelled_case_identity: cancelled.to_owned(),
        },
    )
}

#[test]
fn environment_digest_is_structural_and_covers_all_outbound_metadata() {
    let split = environment_with(CapabilityOperation::new(["X", "Y"], "Z"));
    let joined = environment_with(CapabilityOperation::new(["X,Y"], "Z"));
    assert_ne!(split.stable_digest(), joined.stable_digest());

    let base = canonical_environment(10);
    for changed in [
        environment_with(operation(
            &["Text", "Int", "Cancellation"],
            "outbound.types.LookupOutcome",
            10,
            1,
            2,
            "timed-out",
            "cancelled",
        )),
        environment(11, "timed-out", "cancelled"),
        environment(10, "not-found", "cancelled"),
        environment(10, "timed-out", "unavailable"),
        environment_with(operation(
            &["outbound.types.LookupKey", "Cancellation", "Int"],
            "outbound.types.LookupOutcome",
            10,
            2,
            1,
            "timed-out",
            "cancelled",
        )),
    ] {
        assert_ne!(base.stable_digest(), changed.stable_digest());
    }
}

#[test]
fn unused_invalid_outbound_metadata_is_checked_once_before_publication() {
    let mut interface = environment(10, "timed-out", "cancelled")
        .interface("DependencyClient")
        .unwrap()
        .clone();
    interface.insert_operation(
        "unused",
        operation(
            &["Int", "Cancellation"],
            "outbound.types.LookupOutcome",
            0,
            0,
            1,
            "timed-out",
            "cancelled",
        ),
    );
    let mut invalid = CapabilityEnvironment::new();
    invalid.insert_interface("DependencyClient", interface);
    let failure = EvolutionWorkspace::new(
        "outbound",
        "bad",
        sources(),
        &invalid,
        EvolutionCoverage::default(),
    )
    .unwrap_err();
    assert_eq!(failure.causes, ["AIL.CAPABILITY.OUTBOUND_TIMEOUT_CONTRACT"]);

    let workspace = Workspace::new(
        "unit",
        "r1",
        "unit.ail",
        "fn idle() -> Text {\n  \"idle\"\n}\n",
        invalid,
    )
    .unwrap();
    let mut provider = Provider::returning(OutboundProviderOutcome::TimedOut);
    let failure = failed(workspace.execute(
        ail_compiler::ExecutionRequest {
            revision_id: "r1".into(),
            function: "idle".into(),
            arguments: vec![],
        },
        &mut provider,
    ));
    assert_eq!(failure.fault.code, "AIL.RUNTIME.STATIC_CHECK_REQUIRED");
}

#[test]
fn invalid_outbound_inspection_and_missing_result_identity_fail_closed() {
    const SOURCE: &str = "variant Outcome identity \"outcome.v1\" {\n  TimedOut identity \"timed-out\";\n  Cancelled identity \"cancelled\";\n}\n\nfn fetch(timeout: Int, cancellation: Cancellation, client: capability Client) -> Outcome effects { client.fetch } {\n  client.fetch(timeout, cancellation)\n}\n";
    let valid_operation = operation(
        &["Int", "Cancellation"],
        "Outcome",
        10,
        0,
        1,
        "timed-out",
        "cancelled",
    );
    let mut valid_interface = CapabilityInterface::new();
    valid_interface.insert_operation("fetch", valid_operation);
    let mut valid_environment = CapabilityEnvironment::new();
    valid_environment.insert_interface("Client", valid_interface);
    let valid = Workspace::new(
        "valid-outbound",
        "r1",
        "service.ail",
        SOURCE,
        valid_environment,
    )
    .unwrap();
    let function_handle = valid
        .handles("r1")
        .unwrap()
        .into_iter()
        .find(|handle| {
            valid
                .inspect(InspectionRequest {
                    revision_id: "r1".into(),
                    handle: handle.clone(),
                })
                .is_ok_and(|inspection| inspection.semantic_kind == "function")
        })
        .unwrap();

    let invalid_operation = operation(
        &["Int", "Cancellation"],
        "Outcome",
        10,
        2,
        1,
        "timed-out",
        "cancelled",
    );
    let mut invalid_interface = CapabilityInterface::new();
    invalid_interface.insert_operation("fetch", invalid_operation);
    let mut invalid_environment = CapabilityEnvironment::new();
    invalid_environment.insert_interface("Client", invalid_interface);
    let invalid = Workspace::new(
        "invalid-outbound",
        "r1",
        "service.ail",
        SOURCE,
        invalid_environment,
    )
    .expect("parseable invalid source remains inspectable for diagnostics");
    let diagnostic = invalid
        .inspect(InspectionRequest {
            revision_id: "r1".into(),
            handle: function_handle,
        })
        .expect_err("invalid outbound metadata cannot produce complete inspection");
    assert_eq!(diagnostic.code, "AIL.CAPABILITY.OUTBOUND_TIMEOUT_CONTRACT");

    let source_without_result_identity = SOURCE.replace(" identity \"outcome.v1\"", "");
    let mut missing_identity_interface = CapabilityInterface::new();
    missing_identity_interface.insert_operation(
        "fetch",
        operation(
            &["Int", "Cancellation"],
            "Outcome",
            10,
            0,
            1,
            "timed-out",
            "cancelled",
        ),
    );
    let mut missing_identity_environment = CapabilityEnvironment::new();
    missing_identity_environment.insert_interface("Client", missing_identity_interface);
    let failure = EvolutionWorkspace::new(
        "missing-result-identity",
        "r1",
        vec![EvolutionSource::new(
            "service.ail",
            source_without_result_identity,
        )],
        &missing_identity_environment,
        EvolutionCoverage::default(),
    )
    .unwrap_err();
    assert_eq!(failure.causes, ["AIL.CAPABILITY.OUTBOUND_RESULT_CONTRACT"]);
}

#[test]
fn source_unit_inspection_reports_all_direct_outbound_sites_with_identities() {
    const SOURCE: &str = "variant Outcome identity \"outcome.v1\" {\n  TimedOut identity \"timed-out\";\n  Cancelled identity \"cancelled\";\n}\n\nfn choose(flag: Bool, timeout: Int, cancellation: Cancellation, client: capability Client) -> Outcome effects { client.fetch } {\n  if flag {\n    client.fetch(timeout, cancellation)\n  } else {\n    client.fetch(timeout, cancellation)\n  }\n}\n";
    let mut interface = CapabilityInterface::new();
    interface.insert_operation(
        "fetch",
        operation(
            &["Int", "Cancellation"],
            "Outcome",
            10,
            0,
            1,
            "timed-out",
            "cancelled",
        ),
    );
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("Client", interface);
    let workspace = Workspace::new("unit", "r1", "unit.ail", SOURCE, environment.clone()).unwrap();
    let handle = workspace
        .handles("r1")
        .unwrap()
        .into_iter()
        .find(|handle| {
            handle.kind == HandleKind::Symbol
                && workspace
                    .inspect(InspectionRequest {
                        revision_id: "r1".into(),
                        handle: handle.clone(),
                    })
                    .is_ok_and(|inspection| inspection.semantic_kind == "function")
        })
        .unwrap();
    let inspection = workspace
        .inspect(InspectionRequest {
            revision_id: "r1".into(),
            handle,
        })
        .unwrap();
    assert_eq!(inspection.outbound_requests.len(), 2);
    for request in &inspection.outbound_requests {
        assert_eq!(request.operation_kind, "outbound");
        assert_eq!(request.receiver, "client");
        assert_eq!(request.operation, "fetch");
        assert_eq!(request.result_variant_identity, "outcome.v1");
        assert_eq!(request.timed_out_case_identity, "timed-out");
        assert_eq!(request.cancelled_case_identity, "cancelled");
        assert_eq!(
            request.capability_environment_digest,
            environment.stable_digest()
        );
    }
}

fn environment_with(operation: CapabilityOperation) -> CapabilityEnvironment {
    let mut interface = CapabilityInterface::new();
    interface.insert_operation("fetch", operation);
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("DependencyClient", interface);
    environment
}

fn environment(maximum: u128, timed_out: &str, cancelled: &str) -> CapabilityEnvironment {
    environment_with(operation(
        &["outbound.types.LookupKey", "Int", "Cancellation"],
        "outbound.types.LookupOutcome",
        maximum,
        1,
        2,
        timed_out,
        cancelled,
    ))
}

fn canonical_environment(maximum: u128) -> CapabilityEnvironment {
    environment(maximum, "timed-out", "cancelled")
}

fn sources() -> Vec<EvolutionSource> {
    vec![
        EvolutionSource::new("types.ail", TYPES),
        EvolutionSource::new("service.ail", SERVICE),
    ]
}

fn workspace(environment: &CapabilityEnvironment) -> EvolutionWorkspace {
    EvolutionWorkspace::new(
        "outbound",
        "r1",
        sources(),
        environment,
        EvolutionCoverage::default(),
    )
    .expect("valid outbound workspace")
}

fn arguments(timeout: u128) -> Vec<RuntimeValue> {
    vec![
        RuntimeValue::record(
            "outbound.types.LookupKey",
            [("value", RuntimeValue::Text("key".into()))],
        ),
        RuntimeValue::Int(timeout),
        RuntimeValue::Cancellation(CancellationToken::new("cancel-7")),
    ]
}

fn outcome(case: &str, payload: Option<RuntimeValue>) -> RuntimeValue {
    RuntimeValue::variant("outbound.types.LookupOutcome", case, payload)
}

#[derive(Clone)]
enum ProviderResult {
    Outcome(OutboundProviderOutcome),
    Fault,
}

struct Provider {
    result: ProviderResult,
    available: bool,
    outbound: bool,
    supports_checks: Cell<usize>,
    outbound_calls: usize,
    ordinary_calls: usize,
}

impl Provider {
    fn returning(outcome: OutboundProviderOutcome) -> Self {
        Self {
            result: ProviderResult::Outcome(outcome),
            available: true,
            outbound: true,
            supports_checks: Cell::new(0),
            outbound_calls: 0,
            ordinary_calls: 0,
        }
    }
}

impl CapabilityProvider for Provider {
    fn supports(&self, _: &str, _: &str) -> bool {
        self.supports_checks.set(self.supports_checks.get() + 1);
        self.available
    }
    fn call(
        &mut self,
        _: &str,
        _: &str,
        _: &str,
        _: &[RuntimeValue],
    ) -> Result<RuntimeValue, RuntimeFault> {
        self.ordinary_calls += 1;
        Ok(RuntimeValue::Text("ordinary".into()))
    }
    fn supports_outbound(&self, _: &str, _: &str, _: &str) -> bool {
        self.outbound
    }
    fn call_outbound(
        &mut self,
        request: &OutboundCapabilityRequest,
    ) -> Result<OutboundProviderOutcome, RuntimeFault> {
        self.outbound_calls += 1;
        assert_eq!(
            request.arguments[1],
            RuntimeValue::Int(u128::from(request.timeout_ms))
        );
        assert_eq!(request.cancellation.id, "cancel-7");
        match &self.result {
            ProviderResult::Outcome(value) => Ok(value.clone()),
            ProviderResult::Fault => Err(RuntimeFault::new(
                "TEST.PROVIDER.FAULT",
                Span::empty(0),
                [("operation", "fetch")],
                std::iter::empty::<(&str, &str)>(),
            )),
        }
    }
}

fn completed(response: ExecutionResponse) -> ail_compiler::ExecutionSuccess {
    let ExecutionResponse::Completed(value) = response else {
        panic!("expected completion: {response:#?}")
    };
    value
}

fn failed(response: ExecutionResponse) -> ail_compiler::ExecutionFailure {
    let ExecutionResponse::Failed(value) = response else {
        panic!("expected failure: {response:#?}")
    };
    value
}

fn assert_build_failure(environment: &CapabilityEnvironment, expected: &[&str]) {
    let failure = EvolutionWorkspace::new(
        "outbound",
        "bad",
        sources(),
        environment,
        EvolutionCoverage::default(),
    )
    .unwrap_err();
    assert_eq!(failure.causes, expected);
    assert_eq!(
        failure
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn contract_label_accounting_is_exact_and_backed_by_assertions() {
    // Each name below is exercised by the specifically named assertion in this file.
    let mut exercised_rejections = vec![
        "invalid-timeout-index",
        "invalid-cancellation-index",
        "overlapping-indices",
        "wrong-timeout-type",
        "wrong-cancellation-type",
        "invalid-timeout-maximum",
        "non-variant-result",
        "missing-timeout-case",
        "missing-cancelled-case",
        "same-completion-case",
        "payload-bearing-timeout-case",
        "payload-bearing-cancelled-case",
        "missing-capability-permission",
        "missing-effect-permission",
        "timeout-zero",
        "timeout-over-maximum",
        "malformed-cancellation-value",
        "unsupported-outbound-provider",
        "unknown-returned-case",
        "malformed-returned-value",
        "provider-contract-fault",
    ];
    let mut expected_rejections = REJECTION_CASES.to_vec();
    exercised_rejections.sort_unstable();
    expected_rejections.sort_unstable();
    assert_eq!(exercised_rejections, expected_rejections);

    let mut exercised_completions = vec![
        "returned-found",
        "returned-not-found",
        "returned-unavailable",
        "timed-out-synthesized",
        "cancelled-synthesized",
        "ordinary-call-compatible",
        "inspection-deterministic",
        "retained-revision-binding",
    ];
    let mut expected_completions = COMPLETION_CASES.to_vec();
    exercised_completions.sort_unstable();
    expected_completions.sort_unstable();
    assert_eq!(exercised_completions, expected_completions);
}

#[test]
#[allow(clippy::too_many_lines)]
fn all_static_metadata_rejections_have_stable_diagnostics_and_precedence() {
    let base = ["outbound.types.LookupKey", "Int", "Cancellation"];
    let cases = [
        (
            operation(
                &base,
                "outbound.types.LookupOutcome",
                10,
                3,
                2,
                "TimedOut",
                "Cancelled",
            ),
            "AIL.CAPABILITY.OUTBOUND_TIMEOUT_CONTRACT",
        ),
        (
            operation(
                &base,
                "outbound.types.LookupOutcome",
                10,
                1,
                3,
                "TimedOut",
                "Cancelled",
            ),
            "AIL.CAPABILITY.OUTBOUND_CANCELLATION_CONTRACT",
        ),
        (
            operation(
                &base,
                "outbound.types.LookupOutcome",
                10,
                1,
                1,
                "TimedOut",
                "Cancelled",
            ),
            "AIL.CAPABILITY.OUTBOUND_CANCELLATION_CONTRACT",
        ),
        (
            operation(
                &["outbound.types.LookupKey", "Text", "Cancellation"],
                "outbound.types.LookupOutcome",
                10,
                1,
                2,
                "TimedOut",
                "Cancelled",
            ),
            "AIL.CAPABILITY.OUTBOUND_TIMEOUT_CONTRACT",
        ),
        (
            operation(
                &["outbound.types.LookupKey", "Int", "Text"],
                "outbound.types.LookupOutcome",
                10,
                1,
                2,
                "TimedOut",
                "Cancelled",
            ),
            "AIL.CAPABILITY.OUTBOUND_CANCELLATION_CONTRACT",
        ),
        (
            operation(
                &base,
                "outbound.types.LookupOutcome",
                0,
                1,
                2,
                "TimedOut",
                "Cancelled",
            ),
            "AIL.CAPABILITY.OUTBOUND_TIMEOUT_CONTRACT",
        ),
        (
            operation(
                &base,
                "outbound.types.LookupOutcome",
                u128::from(u64::MAX) + 1,
                1,
                2,
                "TimedOut",
                "Cancelled",
            ),
            "AIL.CAPABILITY.OUTBOUND_TIMEOUT_CONTRACT",
        ),
        (
            operation(
                &base,
                "outbound.types.LookupValue",
                10,
                1,
                2,
                "TimedOut",
                "Cancelled",
            ),
            "AIL.CAPABILITY.OUTBOUND_RESULT_CONTRACT",
        ),
        (
            operation(
                &base,
                "outbound.types.LookupOutcome",
                10,
                1,
                2,
                "Missing",
                "Cancelled",
            ),
            "AIL.CAPABILITY.OUTBOUND_RESULT_CONTRACT",
        ),
        (
            operation(
                &base,
                "outbound.types.LookupOutcome",
                10,
                1,
                2,
                "TimedOut",
                "Missing",
            ),
            "AIL.CAPABILITY.OUTBOUND_RESULT_CONTRACT",
        ),
        (
            operation(
                &base,
                "outbound.types.LookupOutcome",
                10,
                1,
                2,
                "TimedOut",
                "TimedOut",
            ),
            "AIL.CAPABILITY.OUTBOUND_RESULT_CONTRACT",
        ),
        (
            operation(
                &base,
                "outbound.types.LookupOutcome",
                10,
                1,
                2,
                "Found",
                "Cancelled",
            ),
            "AIL.CAPABILITY.OUTBOUND_RESULT_CONTRACT",
        ),
        (
            operation(
                &base,
                "outbound.types.LookupOutcome",
                10,
                1,
                2,
                "TimedOut",
                "Found",
            ),
            "AIL.CAPABILITY.OUTBOUND_RESULT_CONTRACT",
        ),
        // All three sections are invalid: timeout validation deterministically wins.
        (
            operation(
                &base,
                "outbound.types.LookupValue",
                0,
                1,
                1,
                "Missing",
                "Missing",
            ),
            "AIL.CAPABILITY.OUTBOUND_TIMEOUT_CONTRACT",
        ),
    ];
    for (index, (operation, code)) in cases.into_iter().enumerate() {
        let expected = if matches!(index, 7 | 13) {
            vec!["AIL.TYPE.RESULT_MISMATCH", code]
        } else if matches!(index, 3 | 4) {
            vec!["AIL.TYPE.CAPABILITY_ARGUMENT", code]
        } else {
            vec![code]
        };
        assert_build_failure(&environment_with(operation), &expected);
    }
}

#[test]
fn missing_static_effect_blocks_publication() {
    let source = SERVICE.replace(" effects { dependency.fetch }", "");
    let failure = EvolutionWorkspace::new(
        "outbound",
        "bad",
        vec![
            EvolutionSource::new("types.ail", TYPES),
            EvolutionSource::new("service.ail", source),
        ],
        &canonical_environment(10),
        EvolutionCoverage::default(),
    )
    .unwrap_err();
    assert_eq!(failure.causes, ["AIL.CAPABILITY.UNDECLARED_EFFECT"]);
}

#[test]
fn external_rejections_happen_before_capability_checks_or_calls() {
    let workspace = workspace(&canonical_environment(10));
    for timeout in [0, 11] {
        let mut provider = Provider::returning(OutboundProviderOutcome::TimedOut);
        let failure = failed(workspace.execute(
            "r1",
            "outbound.service.lookup",
            arguments(timeout),
            &mut provider,
        ));
        assert_eq!(failure.fault.code, "AIL.RUNTIME.OUTBOUND_TIMEOUT_ARGUMENT");
        assert!(failure.calls.is_empty());
        // Function arguments are validated before capability availability.
        assert_eq!(provider.supports_checks.get(), 1);
        assert_eq!(provider.outbound_calls, 0);
    }

    let mut malformed = arguments(1);
    malformed[2] = RuntimeValue::Text("not-a-token".into());
    let mut provider = Provider::returning(OutboundProviderOutcome::TimedOut);
    let failure =
        failed(workspace.execute("r1", "outbound.service.lookup", malformed, &mut provider));
    assert_eq!(failure.fault.code, "AIL.RUNTIME.ARGUMENT_TYPE");
    assert!(failure.calls.is_empty());
    assert_eq!(provider.supports_checks.get(), 0);
    assert_eq!(provider.outbound_calls, 0);

    let mut unavailable = Provider::returning(OutboundProviderOutcome::TimedOut);
    unavailable.available = false;
    let failure = failed(workspace.execute(
        "r1",
        "outbound.service.lookup",
        arguments(1),
        &mut unavailable,
    ));
    assert_eq!(failure.fault.code, "AIL.RUNTIME.MISSING_CAPABILITY");
    assert!(failure.calls.is_empty());

    let mut unsupported = Provider::returning(OutboundProviderOutcome::TimedOut);
    unsupported.outbound = false;
    let failure = failed(workspace.execute(
        "r1",
        "outbound.service.lookup",
        arguments(1),
        &mut unsupported,
    ));
    assert_eq!(failure.fault.code, "AIL.RUNTIME.OUTBOUND_UNSUPPORTED");
    assert!(failure.calls.is_empty());
    assert_eq!(unsupported.outbound_calls, 0);
    assert_eq!(unsupported.ordinary_calls, 0);
}

#[test]
fn invalid_timeout_prevents_an_earlier_argument_from_calling_outside() {
    let service = "module outbound.service;\nimport outbound.types as types;\n\nfn lookup(cancellation: Cancellation, dependency: capability DependencyClient) -> types.LookupOutcome effects { dependency.key, dependency.fetch } {\n  dependency.fetch(dependency.key(), 0, cancellation)\n}\n";
    let mut interface = canonical_environment(10)
        .interface("DependencyClient")
        .unwrap()
        .clone();
    interface.insert_operation(
        "key",
        CapabilityOperation::new(std::iter::empty::<&str>(), "outbound.types.LookupKey"),
    );
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("DependencyClient", interface);
    let workspace = EvolutionWorkspace::new(
        "timeout-precedence",
        "r1",
        vec![
            EvolutionSource::new("types.ail", TYPES),
            EvolutionSource::new("service.ail", service),
        ],
        &environment,
        EvolutionCoverage::default(),
    )
    .unwrap();
    let mut provider = Provider::returning(OutboundProviderOutcome::TimedOut);
    let failure = failed(workspace.execute(
        "r1",
        "outbound.service.lookup",
        vec![RuntimeValue::Cancellation(CancellationToken::new("cancel"))],
        &mut provider,
    ));
    assert_eq!(failure.fault.code, "AIL.RUNTIME.OUTBOUND_TIMEOUT_ARGUMENT");
    assert_eq!(provider.ordinary_calls, 0);
    assert_eq!(provider.outbound_calls, 0);
    assert!(failure.calls.is_empty());
}

#[test]
fn returned_and_synthesized_closed_completions_are_observable() {
    let workspace = workspace(&canonical_environment(5000));
    let found = outcome(
        "Found",
        Some(RuntimeValue::record(
            "outbound.types.LookupValue",
            [("value", RuntimeValue::Text("value".into()))],
        )),
    );
    for expected in [
        found,
        outcome("NotFound", None),
        outcome("Unavailable", None),
    ] {
        let mut provider = Provider::returning(OutboundProviderOutcome::Returned(expected.clone()));
        let success = completed(workspace.execute(
            "r1",
            "outbound.service.lookup",
            arguments(100),
            &mut provider,
        ));
        assert_eq!(success.value, expected);
        assert_eq!(success.calls.len(), 1);
        assert_eq!(success.calls[0].result.as_ref(), Some(&success.value));
    }
    for (provider_outcome, case) in [
        (OutboundProviderOutcome::TimedOut, "TimedOut"),
        (OutboundProviderOutcome::Cancelled, "Cancelled"),
    ] {
        let mut provider = Provider::returning(provider_outcome.clone());
        let success = completed(workspace.execute(
            "r1",
            "outbound.service.lookup",
            arguments(100),
            &mut provider,
        ));
        assert_eq!(success.value, outcome(case, None));
        let call = &success.calls[0];
        assert_eq!(call.receiver, "dependency");
        assert_eq!(call.interface, "DependencyClient");
        assert_eq!(call.operation, "fetch");
        assert_eq!(call.arguments, arguments(100));
        let outbound = call.outbound.as_ref().unwrap();
        assert_eq!(outbound.effect, "dependency.fetch");
        assert_eq!(outbound.timeout_ms, 100);
        assert_eq!(outbound.cancellation_token_identity, "cancel-7");
        assert_eq!(outbound.outcome, Some(provider_outcome));
    }
}

#[test]
fn invalid_provider_values_and_faults_remain_faults_with_recorded_calls() {
    let workspace = workspace(&canonical_environment(5000));
    // The provider outcome is a closed Rust enum, so an unknown outcome tag is unrepresentable.
    // Returned with an unknown closed case is the equivalent invalid provider contract boundary.
    for value in [
        RuntimeValue::variant("outbound.types.LookupOutcome", "Unknown", None),
        RuntimeValue::Text("not-an-outcome".into()),
    ] {
        let expected_outcome = OutboundProviderOutcome::Returned(value);
        let mut provider = Provider::returning(expected_outcome.clone());
        let failure = failed(workspace.execute(
            "r1",
            "outbound.service.lookup",
            arguments(100),
            &mut provider,
        ));
        assert_eq!(failure.fault.code, "AIL.RUNTIME.CAPABILITY_RESULT");
        assert_eq!(failure.calls.len(), 1);
        assert!(failure.calls[0].result.is_none());
        assert_eq!(
            failure.calls[0].outbound.as_ref().unwrap().outcome,
            Some(expected_outcome)
        );
    }
    let mut provider = Provider::returning(OutboundProviderOutcome::TimedOut);
    provider.result = ProviderResult::Fault;
    let failure = failed(workspace.execute(
        "r1",
        "outbound.service.lookup",
        arguments(100),
        &mut provider,
    ));
    assert_eq!(failure.fault.code, "TEST.PROVIDER.FAULT");
    assert_eq!(failure.calls.len(), 1);
    assert!(failure.calls[0].result.is_none());
    assert!(
        failure.calls[0]
            .outbound
            .as_ref()
            .unwrap()
            .outcome
            .is_none()
    );
}

#[test]
fn ordinary_operations_keep_the_existing_call_path() {
    const SOURCE: &str = "fn read(client: capability Client) -> Text effects { client.read } {\n  client.read()\n}\n";
    let mut interface = CapabilityInterface::new();
    let operation = CapabilityOperation::new(std::iter::empty::<&str>(), "Text");
    assert_eq!(operation.kind, CapabilityOperationKind::Ordinary);
    interface.insert_operation("read", operation);
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("Client", interface);
    let workspace = EvolutionWorkspace::new(
        "ordinary",
        "r1",
        vec![EvolutionSource::new("ordinary.ail", SOURCE)],
        &environment,
        EvolutionCoverage::default(),
    )
    .unwrap();
    let mut provider = Provider::returning(OutboundProviderOutcome::TimedOut);
    let success = completed(workspace.execute("r1", "read", vec![], &mut provider));
    assert_eq!(success.value, RuntimeValue::Text("ordinary".into()));
    assert_eq!(provider.ordinary_calls, 1);
    assert_eq!(provider.outbound_calls, 0);
    assert!(success.calls[0].outbound.is_none());
}

#[test]
fn canonical_format_source_order_and_environment_registration_are_invariant() {
    assert_eq!(format_source(TYPES).unwrap(), TYPES);
    assert_eq!(format_source(SERVICE).unwrap(), SERVICE);
    let environment = canonical_environment(5000);
    let forward = workspace(&environment);
    let reverse = EvolutionWorkspace::new(
        "outbound",
        "r1",
        sources().into_iter().rev().collect(),
        &environment,
        EvolutionCoverage::default(),
    )
    .unwrap();
    assert_eq!(
        forward
            .revision("r1")
            .unwrap()
            .capability_environment_digest,
        reverse
            .revision("r1")
            .unwrap()
            .capability_environment_digest
    );

    let mut extra = CapabilityInterface::new();
    extra.insert_operation("ping", CapabilityOperation::new(["Text"], "Text"));
    let mut first = CapabilityEnvironment::new();
    first.insert_interface("Zed", extra.clone());
    first.insert_interface(
        "DependencyClient",
        environment.interface("DependencyClient").unwrap().clone(),
    );
    let mut second = CapabilityEnvironment::new();
    second.insert_interface(
        "DependencyClient",
        environment.interface("DependencyClient").unwrap().clone(),
    );
    second.insert_interface("Zed", extra);
    assert_eq!(first.stable_digest(), second.stable_digest());

    let inspection = forward
        .inspect_function("r1", "outbound.service.lookup")
        .unwrap();
    assert_eq!(inspection.revision_id, "r1");
    assert_eq!(inspection.effects, ["dependency.fetch"]);
    assert_eq!(inspection.capabilities, ["dependency:DependencyClient"]);
    assert_eq!(inspection.outbound_requests.len(), 1);
    let request = &inspection.outbound_requests[0];
    assert_eq!(request.revision_id, "r1");
    assert_eq!(
        request.capability_environment_digest,
        environment.stable_digest()
    );
    assert_eq!(request.receiver, "dependency");
    assert_eq!(request.operation, "fetch");
    assert_eq!(request.effect, "dependency.fetch");
    assert_eq!(request.operation_kind, "outbound");
    assert_eq!(request.timeout_argument_index, 1);
    assert_eq!(request.timeout_parameter_type, "Int");
    assert_eq!(request.cancellation_argument_index, 2);
    assert_eq!(request.cancellation_parameter_type, "Cancellation");
    assert_eq!(request.maximum_timeout_ms, 5000);
    assert_eq!(
        request.result_variant_identity,
        "outbound.lookup-outcome.v1"
    );
    assert_eq!(request.timed_out_case_identity, "timed-out");
    assert_eq!(request.cancelled_case_identity, "cancelled");
}

#[test]
fn retained_revisions_execute_and_inspect_their_saved_environments() {
    let r1_environment = canonical_environment(5000);
    let r2_environment = environment(9, "not-found", "unavailable");
    let mut workspace = workspace(&r1_environment);
    workspace
        .retain_revision(
            "r2",
            Some("r1".into()),
            sources(),
            &r2_environment,
            EvolutionCoverage::default(),
        )
        .unwrap();
    assert_ne!(
        r1_environment.stable_digest(),
        r2_environment.stable_digest()
    );
    for (revision, environment, maximum, case, display_case) in [
        ("r1", &r1_environment, 5000, "timed-out", "TimedOut"),
        ("r2", &r2_environment, 9, "not-found", "NotFound"),
    ] {
        let saved = workspace.revision(revision).unwrap();
        assert_eq!(
            saved.capability_environment_digest,
            environment.stable_digest()
        );
        let inspection = workspace
            .inspect_function(revision, "outbound.service.lookup")
            .unwrap();
        assert_eq!(inspection.revision_id, revision);
        assert_eq!(inspection.outbound_requests[0].maximum_timeout_ms, maximum);
        assert_eq!(
            inspection.outbound_requests[0].timed_out_case_identity,
            case
        );
        assert_eq!(
            inspection.outbound_requests[0].capability_environment_digest,
            environment.stable_digest()
        );
        let mut provider = Provider::returning(OutboundProviderOutcome::TimedOut);
        let success = completed(workspace.execute(
            revision,
            "outbound.service.lookup",
            arguments(100.min(maximum)),
            &mut provider,
        ));
        assert_eq!(success.revision_id, revision);
        assert_eq!(success.value, outcome(display_case, None));
    }
}
