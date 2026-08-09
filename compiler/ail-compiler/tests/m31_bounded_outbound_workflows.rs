use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ail_compiler::{
    CancellationToken, CapabilityEnvironment, CapabilityInterface, CapabilityOperation,
    CapabilityProvider, EvolutionCoverage, EvolutionSource, EvolutionWorkspace, ExecutionResponse,
    InspectionRequest, OutboundBatchCheck, OutboundCapabilityMetadata, OutboundCapabilityRequest,
    OutboundProviderOutcome, OutboundRequestHandle, RuntimeFault, RuntimeValue, Span,
    TypeCheckStatus, Workspace, check_source, format_source,
};

const TYPES: &str = include_str!("../../examples/batch-lookup/types.ail");
const SERVICE: &str = include_str!("../../examples/batch-lookup/service.ail");

fn environment(maximum: u128) -> CapabilityEnvironment {
    let mut dependency = CapabilityInterface::new();
    dependency.insert_operation(
        "fetch",
        CapabilityOperation::outbound(
            ["batch_lookup.types.LookupRequest", "Int", "Cancellation"],
            "batch_lookup.types.LookupOutcome",
            OutboundCapabilityMetadata {
                timeout_argument_index: 1,
                cancellation_argument_index: 2,
                maximum_timeout_ms: maximum,
                timed_out_case_identity: "timed-out".into(),
                cancelled_case_identity: "cancelled".into(),
            },
        ),
    );
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("DependencyClient", dependency);
    environment
}

fn workspace(maximum: u128) -> EvolutionWorkspace {
    EvolutionWorkspace::new(
        "batch-lookup",
        "r1",
        vec![
            EvolutionSource::new("types.ail", TYPES),
            EvolutionSource::new("service.ail", SERVICE),
        ],
        &environment(maximum),
        EvolutionCoverage::default(),
    )
    .unwrap()
}

fn request(index: usize) -> RuntimeValue {
    RuntimeValue::record(
        "batch_lookup.types.LookupRequest",
        [("key", RuntimeValue::Text(index.to_string()))],
    )
}

fn outcome(case: &str, payload: Option<RuntimeValue>) -> RuntimeValue {
    RuntimeValue::variant("batch_lookup.types.LookupOutcome", case, payload)
}

fn arguments(count: usize, timeout: u128) -> Vec<RuntimeValue> {
    vec![
        RuntimeValue::list((0..count).map(request)),
        RuntimeValue::Int(timeout),
        RuntimeValue::Cancellation(CancellationToken::new("batch-cancel")),
    ]
}

fn completed(response: ExecutionResponse) -> ail_compiler::ExecutionSuccess {
    let ExecutionResponse::Completed(success) = response else {
        panic!("expected completion: {response:#?}");
    };
    success
}

fn failed(response: ExecutionResponse) -> ail_compiler::ExecutionFailure {
    let ExecutionResponse::Failed(failure) = response else {
        panic!("expected failure: {response:#?}");
    };
    failure
}

#[derive(Default)]
#[allow(clippy::struct_excessive_bools)]
struct BatchProvider {
    active: BTreeSet<OutboundRequestHandle>,
    outcomes: BTreeMap<OutboundRequestHandle, OutboundProviderOutcome>,
    completion_order: VecDeque<usize>,
    started: Vec<usize>,
    completed: Vec<usize>,
    cancelled: Vec<usize>,
    max_active: usize,
    cancel_batch_on_check: bool,
    fail_check: bool,
    duplicate_completion: bool,
    malformed_result: bool,
    fail_start_at: Option<usize>,
}

impl BatchProvider {
    fn ordered(order: impl IntoIterator<Item = usize>) -> Self {
        Self {
            completion_order: order.into_iter().collect(),
            ..Self::default()
        }
    }

    fn index(request: &OutboundCapabilityRequest) -> usize {
        request.arguments[0]
            .field("key")
            .and_then(|value| match value {
                RuntimeValue::Text(value) => value.parse().ok(),
                _ => None,
            })
            .unwrap()
    }

    fn handle(index: usize) -> OutboundRequestHandle {
        OutboundRequestHandle(format!("request-{index}"))
    }

    fn handle_index(handle: &OutboundRequestHandle) -> usize {
        handle.0.strip_prefix("request-").unwrap().parse().unwrap()
    }
}

impl CapabilityProvider for BatchProvider {
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
        panic!("bounded workflow must not use the ordinary provider path")
    }

    fn supports_outbound_batch(&self, _: &str, _: &str, _: &str) -> bool {
        true
    }

    fn start_outbound(
        &mut self,
        request: &OutboundCapabilityRequest,
    ) -> Result<OutboundRequestHandle, RuntimeFault> {
        let index = Self::index(request);
        if self.fail_start_at == Some(index) {
            return Err(RuntimeFault::new(
                "TEST.HOST.START_FAILURE",
                Span::empty(0),
                [("start", "successful")],
                [("start", "failed")],
            ));
        }
        let handle = Self::handle(index);
        self.started.push(index);
        self.active.insert(handle.clone());
        self.max_active = self.max_active.max(self.active.len());
        let value = match index {
            2 => OutboundProviderOutcome::TimedOut,
            5 => OutboundProviderOutcome::Cancelled,
            6 => OutboundProviderOutcome::Returned(outcome("Unavailable", None)),
            _ => OutboundProviderOutcome::Returned(outcome(
                "Found",
                Some(RuntimeValue::Text(format!("value-{index}"))),
            )),
        };
        self.outcomes.insert(handle.clone(), value);
        Ok(handle)
    }

    fn check_outbound(
        &mut self,
        _: &[OutboundRequestHandle],
    ) -> Result<OutboundBatchCheck, RuntimeFault> {
        if self.fail_check {
            return Err(RuntimeFault::new(
                "TEST.HOST.FAILURE",
                Span::empty(0),
                [("host", "available")],
                [("host", "failed")],
            ));
        }
        if self.cancel_batch_on_check {
            return Ok(OutboundBatchCheck {
                completed: vec![],
                cancelled: true,
            });
        }
        let position = self
            .completion_order
            .iter()
            .position(|index| self.active.contains(&Self::handle(*index)))
            .expect("test completion schedule always contains an active request");
        let index = self.completion_order.remove(position).unwrap();
        if self.duplicate_completion {
            return Ok(OutboundBatchCheck {
                completed: vec![Self::handle(index), Self::handle(index)],
                cancelled: false,
            });
        }
        Ok(OutboundBatchCheck {
            completed: vec![Self::handle(index)],
            cancelled: false,
        })
    }

    fn cancel_outbound(&mut self, handle: &OutboundRequestHandle) -> Result<(), RuntimeFault> {
        self.cancelled.push(Self::handle_index(handle));
        self.active.remove(handle);
        Ok(())
    }

    fn collect_outbound(
        &mut self,
        handle: &OutboundRequestHandle,
    ) -> Result<OutboundProviderOutcome, RuntimeFault> {
        let index = Self::handle_index(handle);
        self.completed.push(index);
        self.active.remove(handle);
        if self.malformed_result {
            return Ok(OutboundProviderOutcome::Returned(RuntimeValue::Text(
                "malformed".into(),
            )));
        }
        Ok(self.outcomes.remove(handle).unwrap())
    }
}

#[test]
fn eight_requests_never_exceed_three_and_results_remain_in_input_order() {
    assert_eq!(format_source(TYPES).unwrap(), TYPES);
    assert_eq!(format_source(SERVICE).unwrap(), SERVICE);
    let workspace = workspace(1000);
    let order = [2, 0, 1, 4, 3, 7, 5, 6];
    let mut provider = BatchProvider::ordered(order);
    let success = completed(workspace.execute(
        "r1",
        "batch_lookup.service.lookup_batch",
        arguments(8, 100),
        &mut provider,
    ));
    assert_eq!(provider.max_active, 3);
    assert_eq!(provider.started, (0..8).collect::<Vec<_>>());
    assert_eq!(provider.completed, order);
    let RuntimeValue::List(values) = success.value else {
        panic!("list result")
    };
    assert!(matches!(&values[2], RuntimeValue::Variant { case, .. } if case == "TimedOut"));
    assert!(matches!(&values[5], RuntimeValue::Variant { case, .. } if case == "Cancelled"));
    assert!(matches!(&values[6], RuntimeValue::Variant { case, .. } if case == "Unavailable"));
    assert!(
        matches!(&values[7], RuntimeValue::Variant { case, payload: Some(value), .. } if case == "Found" && **value == RuntimeValue::Text("value-7".into()))
    );
    assert_eq!(
        success
            .calls
            .iter()
            .map(|call| call.outbound.as_ref().unwrap().batch_index.unwrap())
            .collect::<Vec<_>>(),
        (0..8).collect::<Vec<_>>()
    );
    let mut observed_completion = success
        .calls
        .iter()
        .map(|call| {
            let outbound = call.outbound.as_ref().unwrap();
            (
                outbound.completion_order.unwrap(),
                outbound.batch_index.unwrap(),
            )
        })
        .collect::<Vec<_>>();
    observed_completion.sort_unstable();
    assert_eq!(
        observed_completion
            .into_iter()
            .map(|(_, index)| index)
            .collect::<Vec<_>>(),
        order
    );
}

#[test]
fn invalid_inputs_start_zero_requests_and_empty_input_is_empty() {
    let workspace = workspace(1000);
    for invalid in [arguments(9, 100), arguments(8, 0), arguments(8, 1001)] {
        let mut provider = BatchProvider::ordered(0..8);
        let failure = failed(workspace.execute(
            "r1",
            "batch_lookup.service.lookup_batch",
            invalid,
            &mut provider,
        ));
        assert!(matches!(
            failure.fault.code,
            "AIL.RUNTIME.LIST_CARDINALITY" | "AIL.RUNTIME.OUTBOUND_TIMEOUT_ARGUMENT"
        ));
        assert!(provider.started.is_empty());
        assert!(failure.calls.is_empty());
    }
    let mut malformed_cancellation = arguments(8, 100);
    malformed_cancellation[2] = RuntimeValue::Text("not-a-token".into());
    let mut provider = BatchProvider::ordered(0..8);
    let failure = failed(workspace.execute(
        "r1",
        "batch_lookup.service.lookup_batch",
        malformed_cancellation,
        &mut provider,
    ));
    assert_eq!(failure.fault.code, "AIL.RUNTIME.ARGUMENT_TYPE");
    assert!(provider.started.is_empty());
    assert!(failure.calls.is_empty());
    let mut provider = BatchProvider::default();
    let success = completed(workspace.execute(
        "r1",
        "batch_lookup.service.lookup_batch",
        arguments(0, 100),
        &mut provider,
    ));
    assert_eq!(success.value, RuntimeValue::list([]));
    assert!(provider.started.is_empty());

    let invalid_limit = SERVICE.replace("limit 3", "limit 9");
    let checked = check_source(&invalid_limit, "invalid-limit", &environment(1000));
    assert_eq!(checked.type_result.status, TypeCheckStatus::Error);
    assert!(
        checked
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "AIL.TYPE.PARALLEL_MAP_LIMIT")
    );
    let shadowing = SERVICE.replace("map item in requests", "map timeout in requests");
    let checked = check_source(&shadowing, "shadowing", &environment(1000));
    assert!(
        checked
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "AIL.NAME.DUPLICATE_DECLARATION")
    );
}

#[test]
fn whole_batch_cancellation_marks_every_uncompleted_position_without_new_starts() {
    let workspace = workspace(1000);
    let mut provider = BatchProvider {
        cancel_batch_on_check: true,
        ..BatchProvider::default()
    };
    let success = completed(workspace.execute(
        "r1",
        "batch_lookup.service.lookup_batch",
        arguments(8, 100),
        &mut provider,
    ));
    assert_eq!(provider.started, [0, 1, 2]);
    provider.cancelled.sort_unstable();
    assert_eq!(provider.cancelled, [0, 1, 2]);
    let RuntimeValue::List(values) = success.value else {
        panic!("list result")
    };
    assert_eq!(values.len(), 8);
    assert!(
        values.iter().all(
            |value| matches!(value, RuntimeValue::Variant { case, .. } if case == "Cancelled")
        )
    );
    assert_eq!(success.calls.len(), 3);
    assert!(success.calls.iter().all(|call| {
        call.result.as_ref().is_some_and(
            |value| matches!(value, RuntimeValue::Variant { case, .. } if case == "Cancelled"),
        ) && call.outbound.as_ref().is_some_and(|outbound| {
            outbound.outcome == Some(OutboundProviderOutcome::Cancelled)
                && outbound.completion_order.is_none()
        })
    }));
}

#[test]
fn failed_third_start_records_and_cancels_only_successfully_started_requests() {
    let workspace = workspace(1000);
    let mut provider = BatchProvider {
        fail_start_at: Some(2),
        ..BatchProvider::default()
    };
    let failure = failed(workspace.execute(
        "r1",
        "batch_lookup.service.lookup_batch",
        arguments(8, 100),
        &mut provider,
    ));
    assert_eq!(failure.fault.code, "TEST.HOST.START_FAILURE");
    assert_eq!(provider.started, [0, 1]);
    provider.cancelled.sort_unstable();
    assert_eq!(provider.cancelled, [0, 1]);
    assert_eq!(failure.calls.len(), 2);
    assert!(failure.calls.iter().all(|call| {
        call.result.is_some()
            && call.outbound.as_ref().is_some_and(|outbound| {
                outbound.outcome == Some(OutboundProviderOutcome::Cancelled)
                    && outbound.completion_order.is_none()
            })
    }));
}

#[test]
fn parallel_map_arguments_reject_outside_operations_but_allow_effect_free_helpers() {
    let direct = SERVICE.replace(
        "dependency.fetch(item, timeout, cancellation)",
        "dependency.fetch(dependency.fetch(item, timeout, cancellation), timeout, cancellation)",
    );
    let checked = check_source(&direct, "direct-hidden-effect", &environment(1000));
    assert!(
        checked
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "AIL.CAPABILITY.PARALLEL_MAP_ARGUMENT_EFFECT" })
    );

    let helper = SERVICE.replace(
        "fn lookup_batch",
        "fn hidden(item: types.LookupRequest, timeout: Int, cancellation: Cancellation, dependency: capability DependencyClient) -> types.LookupOutcome effects { dependency.fetch } {\n  dependency.fetch(item, timeout, cancellation)\n}\n\nfn lookup_batch",
    );
    let helper = helper.replace(
        "dependency.fetch(item, timeout, cancellation)\n  }\n}",
        "dependency.fetch(hidden(item, timeout, cancellation), timeout, cancellation)\n  }\n}",
    );
    let checked = check_source(&helper, "helper-hidden-effect", &environment(1000));
    assert!(
        checked
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "AIL.CAPABILITY.PARALLEL_MAP_ARGUMENT_EFFECT" })
    );

    let pure = SERVICE.replace(
        "fn lookup_batch",
        "fn identity(item: types.LookupRequest) -> types.LookupRequest {\n  item\n}\n\nfn lookup_batch",
    );
    let pure = pure.replace(
        "dependency.fetch(item, timeout, cancellation)",
        "dependency.fetch(identity(item), timeout, cancellation)",
    );
    EvolutionWorkspace::new(
        "effect-free-helper",
        "r1",
        vec![
            EvolutionSource::new("types.ail", TYPES),
            EvolutionSource::new("service.ail", pure),
        ],
        &environment(1000),
        EvolutionCoverage::default(),
    )
    .expect("effect-free helper arguments remain accepted");
}

#[test]
fn unexpected_host_failure_stops_starts_cancels_active_and_preserves_original_fault() {
    let workspace = workspace(1000);
    let mut provider = BatchProvider {
        fail_check: true,
        ..BatchProvider::default()
    };
    let failure = failed(workspace.execute(
        "r1",
        "batch_lookup.service.lookup_batch",
        arguments(8, 100),
        &mut provider,
    ));
    assert_eq!(failure.fault.code, "TEST.HOST.FAILURE");
    assert_eq!(provider.started, [0, 1, 2]);
    provider.cancelled.sort_unstable();
    assert_eq!(provider.cancelled, [0, 1, 2]);
    assert_eq!(failure.calls.len(), 3);
}

#[test]
fn malformed_host_completions_fail_before_collection_and_trace_collected_faults() {
    let workspace = workspace(1000);
    let mut duplicate = BatchProvider {
        completion_order: [0].into(),
        duplicate_completion: true,
        ..BatchProvider::default()
    };
    let failure = failed(workspace.execute(
        "r1",
        "batch_lookup.service.lookup_batch",
        arguments(8, 100),
        &mut duplicate,
    ));
    assert_eq!(failure.fault.code, "AIL.RUNTIME.OUTBOUND_HOST_CONTRACT");
    assert!(duplicate.completed.is_empty());
    duplicate.cancelled.sort_unstable();
    assert_eq!(duplicate.cancelled, [0, 1, 2]);

    let mut malformed = BatchProvider {
        completion_order: [0].into(),
        malformed_result: true,
        ..BatchProvider::default()
    };
    let failure = failed(workspace.execute(
        "r1",
        "batch_lookup.service.lookup_batch",
        arguments(8, 100),
        &mut malformed,
    ));
    assert_eq!(failure.fault.code, "AIL.RUNTIME.CAPABILITY_RESULT");
    malformed.cancelled.sort_unstable();
    assert_eq!(malformed.cancelled, [1, 2]);
    let outbound = failure.calls[0].outbound.as_ref().unwrap();
    assert_eq!(outbound.completion_order, Some(0));
    assert!(outbound.outcome.is_some());
    assert!(failure.calls[0].result.is_none());
}

#[test]
fn repeated_workflows_update_their_own_observed_calls() {
    let service = format!(
        "{SERVICE}\nfn lookup_twice(requests: List<types.LookupRequest, 8>, timeout: Int, cancellation: Cancellation, dependency: capability DependencyClient) -> List<types.LookupOutcome, 8> effects {{ dependency.fetch }} {{\n  let first = lookup_batch(requests, timeout, cancellation);\n  lookup_batch(requests, timeout, cancellation)\n}}\n"
    );
    let workspace = EvolutionWorkspace::new(
        "repeated-batch",
        "r1",
        vec![
            EvolutionSource::new("types.ail", TYPES),
            EvolutionSource::new("service.ail", service),
        ],
        &environment(1000),
        EvolutionCoverage::default(),
    )
    .unwrap();
    let order = [2, 0, 1, 4, 3, 7, 5, 6];
    let mut provider = BatchProvider::ordered(order.into_iter().chain(order));
    let success = completed(workspace.execute(
        "r1",
        "batch_lookup.service.lookup_twice",
        arguments(8, 100),
        &mut provider,
    ));
    assert_eq!(success.calls.len(), 16);
    assert!(success.calls.iter().all(|call| {
        let outbound = call.outbound.as_ref().unwrap();
        call.result.is_some() && outbound.outcome.is_some() && outbound.completion_order.is_some()
    }));
    assert_eq!(
        success.calls[8..]
            .iter()
            .map(|call| call.outbound.as_ref().unwrap().batch_index.unwrap())
            .collect::<Vec<_>>(),
        (0..8).collect::<Vec<_>>()
    );
}

#[test]
fn inspection_reports_the_complete_revision_bound_workflow() {
    let r1_environment = environment(1000);
    let workspace = workspace(1000);
    let inspection = workspace
        .inspect_function("r1", "batch_lookup.service.lookup_batch")
        .unwrap();
    assert_eq!(inspection.bounded_parallel_maps.len(), 1);
    let map = &inspection.bounded_parallel_maps[0];
    assert_eq!(
        map.capability_environment_digest,
        r1_environment.stable_digest()
    );
    assert_eq!(map.effect, "dependency.fetch");
    assert_eq!(map.input_list_bound, 8);
    assert_eq!(map.concurrency_limit, 3);
    assert_eq!(map.timeout_input, "timeout");
    assert_eq!(map.cancellation_input, "cancellation");
    assert_eq!(map.result_ordering, "input-order");
    assert_eq!(
        map.completion_results,
        ["Found", "NotFound", "Unavailable", "TimedOut", "Cancelled"]
    );

    let mut retained = workspace;
    let r2_environment = environment(2000);
    retained
        .retain_revision(
            "r2",
            Some("r1".into()),
            vec![
                EvolutionSource::new("types.ail", TYPES),
                EvolutionSource::new("service.ail", SERVICE),
            ],
            &r2_environment,
            EvolutionCoverage::default(),
        )
        .unwrap();
    assert_eq!(
        retained
            .inspect_function("r1", "batch_lookup.service.lookup_batch")
            .unwrap()
            .bounded_parallel_maps[0]
            .capability_environment_digest,
        r1_environment.stable_digest()
    );
    assert_eq!(
        retained
            .inspect_function("r2", "batch_lookup.service.lookup_batch")
            .unwrap()
            .bounded_parallel_maps[0]
            .capability_environment_digest,
        r2_environment.stable_digest()
    );
}

#[test]
fn source_inspection_indexes_the_parallel_map_binding() {
    let source = "record Request identity \"request\" { key: Text; }\n\nvariant Outcome identity \"outcome\" {\n  Found identity \"found\";\n  TimedOut identity \"timed-out\";\n  Cancelled identity \"cancelled\";\n}\n\nfn batch(requests: List<Request, 8>, timeout: Int, cancellation: Cancellation, dependency: capability DependencyClient) -> List<Outcome, 8> effects { dependency.fetch } {\n  parallel map item in requests limit 3 {\n    dependency.fetch(item, timeout, cancellation)\n  }\n}\n";
    let mut dependency = CapabilityInterface::new();
    dependency.insert_operation(
        "fetch",
        CapabilityOperation::outbound(
            ["Request", "Int", "Cancellation"],
            "Outcome",
            OutboundCapabilityMetadata {
                timeout_argument_index: 1,
                cancellation_argument_index: 2,
                maximum_timeout_ms: 1000,
                timed_out_case_identity: "timed-out".into(),
                cancelled_case_identity: "cancelled".into(),
            },
        ),
    );
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("DependencyClient", dependency);
    let workspace = Workspace::new("binding", "r1", "binding.ail", source, environment).unwrap();
    let binding = workspace
        .handles("r1")
        .unwrap()
        .into_iter()
        .find_map(|handle| {
            let inspection = workspace
                .inspect(InspectionRequest {
                    revision_id: "r1".into(),
                    handle,
                })
                .ok()?;
            (inspection.semantic_kind == "parallel-map-binding").then_some(inspection)
        })
        .expect("parallel map binding is indexed");
    assert_eq!(binding.inferred_type.as_deref(), Some("Request"));
    assert!(binding.dependencies.contains(&"Request".into()));
}
