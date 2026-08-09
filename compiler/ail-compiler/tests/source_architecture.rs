use std::cell::Cell;
use std::collections::BTreeMap;

use ail_compiler::{
    ArchitectureChangeResult, ArchitectureEvaluationInput, ArchitecturePolicyContext,
    ArchitectureRequest, ArchitectureSourceChangeRequest, BaselineMatch, BehaviorValidation,
    CandidateRevision, CapabilityEnvironment, CapabilityInterface, CapabilityOperation,
    CapabilityProvider, DispatchBudget, EvolutionCoverage, EvolutionSource, EvolutionWorkspace,
    ExecutionResponse, GroupDependencies, NewUnitBudget, RuntimeFault, RuntimeValue,
    SourceArchitectureConfig, SourceOperationArchitecture, SourceStateAccess,
};

const CONTRACTS: &str = r"module contracts;

record CancelRequest {
  value: Text;
}

variant StoreOutcome {
  Queued;
  Running;
  Missing;
  Completed;
  Cancelled;
}

variant CancelOutcome {
  Malformed;
  Cancelled;
  NotFound;
  NotCancellable;
}

fn preserve(request: CancelRequest) -> CancelRequest {
  request
}
";

const BASE_TRANSPORT: &str = r"module transport;
import contracts;

fn dispatch(request: contracts.CancelRequest) -> contracts.CancelOutcome {
  contracts.CancelOutcome::NotFound
}
";

const CENTRALIZED_TRANSPORT: &str = r"module transport;
import contracts;

fn dispatch(request: contracts.CancelRequest, store: capability JobsStore) -> contracts.CancelOutcome effects { store.cancel_if_active } {
  if text.is_empty(request.value) {
    contracts.CancelOutcome::Malformed
  } else {
    let stored = store.cancel_if_active(request);
    match stored {
      contracts.StoreOutcome::Queued => { contracts.CancelOutcome::Cancelled },
      contracts.StoreOutcome::Running => { contracts.CancelOutcome::Cancelled },
      contracts.StoreOutcome::Missing => { contracts.CancelOutcome::NotFound },
      contracts.StoreOutcome::Completed => { contracts.CancelOutcome::NotCancellable },
      contracts.StoreOutcome::Cancelled => { contracts.CancelOutcome::NotCancellable },
    }
  }
}
";

const HELPER_SPLIT_TRANSPORT: &str = r"module transport;
import contracts;

fn invoke_store(request: contracts.CancelRequest, store: capability JobsStore) -> contracts.StoreOutcome effects { store.cancel_if_active } {
  store.cancel_if_active(request)
}

fn classify(stored: contracts.StoreOutcome) -> contracts.CancelOutcome {
  match stored {
    contracts.StoreOutcome::Queued => { contracts.CancelOutcome::Cancelled },
    contracts.StoreOutcome::Running => { contracts.CancelOutcome::Cancelled },
    contracts.StoreOutcome::Missing => { contracts.CancelOutcome::NotFound },
    contracts.StoreOutcome::Completed => { contracts.CancelOutcome::NotCancellable },
    contracts.StoreOutcome::Cancelled => { contracts.CancelOutcome::NotCancellable },
  }
}

fn decide(request: contracts.CancelRequest, store: capability JobsStore) -> contracts.CancelOutcome effects { store.cancel_if_active } {
  if text.is_empty(request.value) {
    contracts.CancelOutcome::Malformed
  } else {
    classify(invoke_store(request))
  }
}

fn dispatch(request: contracts.CancelRequest, store: capability JobsStore) -> contracts.CancelOutcome effects { store.cancel_if_active } {
  decide(request)
}
";

const BASE_DOMAIN: &str = r"module domain;

fn marker(value: Text) -> Text {
  value
}
";

const DOMAIN_OWNED: &str = r"module domain;
import contracts;

fn marker(value: Text) -> Text {
  value
}

fn cancel_job(request: contracts.CancelRequest, store: capability JobsStore) -> contracts.CancelOutcome effects { store.cancel_if_active } {
  if text.is_empty(request.value) {
    contracts.CancelOutcome::Malformed
  } else {
    let stored = store.cancel_if_active(request);
    match stored {
      contracts.StoreOutcome::Queued => { contracts.CancelOutcome::Cancelled },
      contracts.StoreOutcome::Running => { contracts.CancelOutcome::Cancelled },
      contracts.StoreOutcome::Missing => { contracts.CancelOutcome::NotFound },
      contracts.StoreOutcome::Completed => { contracts.CancelOutcome::NotCancellable },
      contracts.StoreOutcome::Cancelled => { contracts.CancelOutcome::NotCancellable },
    }
  }
}
";

const ADAPTERS: &str = r"module adapters;

fn marker(value: Text) -> Text {
  value
}
";

const TESTS: &str = r"module tests;

fn marker(value: Text) -> Text {
  value
}
";

fn environment() -> CapabilityEnvironment {
    let mut store = CapabilityInterface::new();
    store.insert_operation(
        "cancel_if_active",
        CapabilityOperation::new(["contracts.CancelRequest"], "contracts.StoreOutcome"),
    );
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("JobsStore", store);
    environment
}

fn sources(transport: &str, domain: &str) -> Vec<EvolutionSource> {
    vec![
        EvolutionSource::new("adapters.ail", ADAPTERS),
        EvolutionSource::new("contracts.ail", CONTRACTS),
        EvolutionSource::new("domain.ail", domain),
        EvolutionSource::new("tests.ail", TESTS),
        EvolutionSource::new("transport.ail", transport),
    ]
}

fn workspace() -> EvolutionWorkspace {
    EvolutionWorkspace::new_with_architecture(
        "source-cancel-job",
        "r1",
        sources(BASE_TRANSPORT, BASE_DOMAIN),
        &environment(),
        EvolutionCoverage {
            declared_complete: true,
            ..EvolutionCoverage::default()
        },
        architecture_config(),
    )
    .expect("base source compiles")
}

fn architecture_config() -> SourceArchitectureConfig {
    let dispatch = DispatchBudget {
        control_flow_complexity: 1,
        minimal_context_node_count: 3,
    };
    SourceArchitectureConfig {
        module_groups: BTreeMap::from([
            ("adapters".into(), "persistence-adapter".into()),
            ("contracts".into(), "contract".into()),
            ("domain".into(), "domain".into()),
            ("tests".into(), "verification".into()),
            ("transport".into(), "transport".into()),
        ]),
        capability_namespaces: BTreeMap::from([("JobsStore".into(), "jobs_store".into())]),
        endpoint_groups: BTreeMap::from([
            ("capability:jobs_store".into(), "persistence-adapter".into()),
            ("state:jobs".into(), "persistence-adapter".into()),
            ("text:is_empty".into(), "contract".into()),
        ]),
        operations: BTreeMap::from([(
            "JobsStore.cancel_if_active".into(),
            SourceOperationArchitecture::State {
                domain: "jobs".into(),
                access: SourceStateAccess::ReadWrite,
            },
        )]),
        policy: ArchitecturePolicyContext {
            revision: "policy-r1".into(),
            allowed_group_dependencies: GroupDependencies {
                contract: vec![],
                transport: vec!["contract".into(), "domain".into()],
                domain: vec!["contract".into(), "persistence-adapter".into()],
                persistence_adapter: vec![],
                verification: vec!["contract".into(), "domain".into(), "transport".into()],
            },
            transport_capabilities: vec![],
            transport_state: vec![],
            dispatch_no_growth: dispatch.clone(),
            new_unit: NewUnitBudget {
                control_flow_complexity_max: 6,
                minimal_context_node_count_max: 12,
            },
            new_cycles: false,
            coverage_required: true,
            baseline_match: BaselineMatch {
                baseline_revision: "baseline-r1".into(),
                scope: "transport:dispatch".into(),
                metrics: dispatch,
                accepted_debt: true,
            },
        },
        semantic_model_version: "source-architecture-v1".into(),
    }
}

fn evaluation_input(scope: &str) -> ArchitectureEvaluationInput {
    ArchitectureEvaluationInput {
        request: ArchitectureRequest {
            base_revision_id: "r1".into(),
            candidate_revision_id: "r2".into(),
            analysis_scope: scope.into(),
            policy_revision: "policy-r1".into(),
            baseline_revision: "baseline-r1".into(),
            review_boundary: "review-r1".into(),
            requested_governance_changes: vec![],
            authorization_id: None,
        },
        governance_authorizations: vec![],
        active_exceptions: vec![],
        active_policy_revision: "policy-r1".into(),
        active_baseline_revision: "baseline-r1".into(),
    }
}

fn request(transport: &str, domain: &str) -> ArchitectureSourceChangeRequest {
    ArchitectureSourceChangeRequest {
        base_revision_id: "r1".into(),
        candidate_sources: sources(transport, domain),
    }
}

fn cancel_request(value: &str) -> RuntimeValue {
    RuntimeValue::record(
        "contracts.CancelRequest",
        [("value", RuntimeValue::Text(value.into()))],
    )
}

fn outcome(case: &str) -> RuntimeValue {
    RuntimeValue::variant("contracts.CancelOutcome", case, None)
}

#[derive(Default)]
struct JobsStore {
    calls: usize,
}

impl CapabilityProvider for JobsStore {
    fn supports(&self, receiver: &str, interface: &str) -> bool {
        receiver == "store" && interface == "JobsStore"
    }

    fn call(
        &mut self,
        _receiver: &str,
        _interface: &str,
        operation: &str,
        arguments: &[RuntimeValue],
    ) -> Result<RuntimeValue, RuntimeFault> {
        assert_eq!(operation, "cancel_if_active");
        self.calls += 1;
        let value = arguments[0]
            .field("value")
            .and_then(|value| match value {
                RuntimeValue::Text(value) => Some(value.as_str()),
                _ => None,
            })
            .expect("checked request value");
        let case = match value {
            "queued" => "Queued",
            "running" => "Running",
            "missing" => "Missing",
            "completed" => "Completed",
            "cancelled" => "Cancelled",
            other => panic!("unexpected store input {other}"),
        };
        Ok(RuntimeValue::variant("contracts.StoreOutcome", case, None))
    }
}

fn behavior(
    entry: &'static str,
) -> impl FnOnce(
    &CandidateRevision<'_>,
) -> Result<BehaviorValidation, ail_compiler::ArchitectureRequestError> {
    move |candidate| {
        let cases = [
            ("", "Malformed", 0),
            ("queued", "Cancelled", 1),
            ("running", "Cancelled", 1),
            ("missing", "NotFound", 1),
            ("completed", "NotCancellable", 1),
            ("cancelled", "NotCancellable", 1),
        ];
        for (input, expected, expected_calls) in cases {
            let mut store = JobsStore::default();
            let ExecutionResponse::Completed(result) =
                candidate.execute(entry, vec![cancel_request(input)], &mut store)
            else {
                panic!("{entry} must pass behavior case {input}");
            };
            assert_eq!(result.value, outcome(expected));
            assert_eq!(store.calls, expected_calls);
            assert_eq!(result.calls.len(), expected_calls);
        }
        Ok(BehaviorValidation {
            status: "passed".into(),
            cases_passed: 6,
            cases_total: 6,
        })
    }
}

#[test]
fn source_architecture_accepts_domain_ownership_and_rolls_back_transport_regressions() {
    let config = architecture_config();

    let mut valid = workspace();
    let valid_result = valid
        .validate_source_architecture_change(
            request(BASE_TRANSPORT, DOMAIN_OWNED),
            &config,
            &evaluation_input("domain:cancel_job"),
            behavior("domain.cancel_job"),
        )
        .expect("source-derived architecture evaluates");
    let ArchitectureChangeResult::Success(success) = valid_result else {
        panic!("domain-owned candidate must publish: {valid_result:#?}");
    };
    let domain_scope = success
        .snapshot
        .scopes
        .iter()
        .find(|scope| scope.identity == "domain:cancel_job")
        .expect("accepted snapshot contains the source-derived domain handler");
    assert!(
        domain_scope
            .direct_dependency_set
            .contains(&"capability:jobs_store.cancel_if_active".into())
    );
    assert_eq!(valid.current_revision_id(), "r2");
    assert!(valid.revision("r2").is_some());
    assert!(
        valid
            .sources("r2")
            .unwrap()
            .iter()
            .any(|source| source.source.contains("fn cancel_job("))
    );

    for (transport, scope, entry) in [
        (
            CENTRALIZED_TRANSPORT,
            "transport:dispatch",
            "transport.dispatch",
        ),
        (
            HELPER_SPLIT_TRANSPORT,
            "transport:decide",
            "transport.dispatch",
        ),
        (
            HELPER_SPLIT_TRANSPORT,
            "transport:dispatch",
            "transport.dispatch",
        ),
    ] {
        let mut rejected = workspace();
        let result = rejected
            .validate_source_architecture_change(
                request(transport, BASE_DOMAIN),
                &config,
                &evaluation_input(scope),
                behavior(entry),
            )
            .expect("behaviorally valid source reaches architecture policy");
        let ArchitectureChangeResult::Failure(failure) = result else {
            panic!("transport-owned candidate must be denied: {result:#?}");
        };
        if transport == HELPER_SPLIT_TRANSPORT {
            let reported = failure
                .snapshot
                .scopes
                .iter()
                .find(|item| item.identity == scope)
                .unwrap();
            assert_eq!(reported.state_read_set, ["jobs"]);
            assert_eq!(reported.state_write_set, ["jobs"]);
            assert!(
                reported
                    .direct_dependency_set
                    .contains(&"capability:jobs_store.cancel_if_active".into())
            );
        }
        let diagnostics = serde_json::to_string(&failure.diagnostics).unwrap();
        assert!(diagnostics.contains("AIL.ARCH.AUTHORITY"));
        assert!(diagnostics.contains("AIL.ARCH.STATE"));
        assert!(diagnostics.contains("group:transport"));
        assert_eq!(failure.published_child_revision_id, None);
        assert_eq!(rejected.current_revision_id(), "r1");
        assert!(rejected.revision("r2").is_none());
    }
}

#[test]
fn missing_capability_architecture_configuration_fails_closed_without_publication() {
    let mut configurations = Vec::new();
    let mut missing_interface = architecture_config();
    missing_interface.capability_namespaces.clear();
    configurations.push(missing_interface);
    let mut missing_operation = architecture_config();
    missing_operation.operations.clear();
    configurations.push(missing_operation);

    for config in configurations {
        let mut workspace = workspace();
        let error = workspace
            .validate_source_architecture_change(
                request(BASE_TRANSPORT, DOMAIN_OWNED),
                &config,
                &evaluation_input("domain:cancel_job"),
                behavior("domain.cancel_job"),
            )
            .expect_err("missing source interpretation cannot produce complete analysis");
        assert_eq!(
            error.kind,
            ail_compiler::ArchitectureRequestErrorKind::InvalidRevision
        );
        assert_eq!(workspace.current_revision_id(), "r1");
        assert!(workspace.revision("r2").is_none());
    }
}

#[test]
fn saved_architecture_settings_are_revision_bound_before_behavior_or_publication() {
    let original = architecture_config();
    let mut changes = Vec::new();
    let mut ownership = original.clone();
    ownership
        .module_groups
        .insert("domain".into(), "transport".into());
    changes.push(ownership);
    let mut state = original.clone();
    state.operations.insert(
        "JobsStore.cancel_if_active".into(),
        SourceOperationArchitecture::Stateless,
    );
    changes.push(state);
    let mut policy = original.clone();
    policy.policy.coverage_required = false;
    changes.push(policy);

    for changed in changes {
        assert_ne!(original.stable_digest(), changed.stable_digest());
        let behavior_called = Cell::new(false);
        let mut workspace = workspace();
        let error = workspace
            .validate_source_architecture_change(
                request(BASE_TRANSPORT, DOMAIN_OWNED),
                &changed,
                &evaluation_input("domain:cancel_job"),
                |_| {
                    behavior_called.set(true);
                    Ok(BehaviorValidation {
                        status: "passed".into(),
                        cases_passed: 6,
                        cases_total: 6,
                    })
                },
            )
            .expect_err("ordinary source changes cannot replace saved settings");
        assert_eq!(
            error.kind,
            ail_compiler::ArchitectureRequestErrorKind::InvalidRevision
        );
        assert!(!behavior_called.get());
        assert_eq!(workspace.current_revision_id(), "r1");
        assert!(workspace.revision("r2").is_none());
    }
}

#[test]
fn architecture_settings_digest_is_stable_and_inherited() {
    let first = architecture_config();
    let mut second = architecture_config();
    second.module_groups = second.module_groups.into_iter().rev().collect();
    second.endpoint_groups = second.endpoint_groups.into_iter().rev().collect();
    assert_eq!(first.stable_digest(), second.stable_digest());

    let mut workspace = workspace();
    let base_digest = workspace
        .revision("r1")
        .unwrap()
        .architecture_settings_digest
        .clone();
    assert_eq!(base_digest.as_deref(), Some(first.stable_digest().as_str()));
    workspace
        .retain_revision(
            "r1-edit",
            Some("r1".into()),
            sources(BASE_TRANSPORT, BASE_DOMAIN),
            &environment(),
            EvolutionCoverage {
                declared_complete: true,
                ..EvolutionCoverage::default()
            },
        )
        .unwrap();
    assert_eq!(
        workspace
            .revision("r1-edit")
            .unwrap()
            .architecture_settings_digest,
        base_digest
    );
    let result = workspace
        .validate_source_architecture_change(
            request(BASE_TRANSPORT, DOMAIN_OWNED),
            &second,
            &evaluation_input("domain:cancel_job"),
            behavior("domain.cancel_job"),
        )
        .unwrap();
    assert!(matches!(result, ArchitectureChangeResult::Success(_)));
    assert_eq!(
        workspace
            .revision("r2")
            .unwrap()
            .architecture_settings_digest,
        base_digest
    );
}

#[test]
fn unused_endpoint_settings_and_declared_coverage_cannot_forge_completeness() {
    for declared_complete in [true, false] {
        let mut config = architecture_config();
        config.module_groups = BTreeMap::from([
            ("contracts".into(), "contract".into()),
            ("transport".into(), "transport".into()),
        ]);
        config.capability_namespaces.clear();
        config.operations.clear();
        config.endpoint_groups = BTreeMap::from([
            ("unused-contract".into(), "contract".into()),
            ("unused-transport".into(), "transport".into()),
            ("unused-domain".into(), "domain".into()),
            ("unused-adapter".into(), "persistence-adapter".into()),
            ("unused-tests".into(), "verification".into()),
        ]);
        let incomplete_sources = vec![
            EvolutionSource::new("contracts.ail", CONTRACTS),
            EvolutionSource::new("transport.ail", BASE_TRANSPORT),
        ];
        let mut workspace = EvolutionWorkspace::new_with_architecture(
            "incomplete-source",
            "r1",
            incomplete_sources.clone(),
            &CapabilityEnvironment::new(),
            EvolutionCoverage {
                declared_complete,
                ..EvolutionCoverage::default()
            },
            config.clone(),
        )
        .unwrap();
        let error = workspace
            .validate_source_architecture_change(
                ArchitectureSourceChangeRequest {
                    base_revision_id: "r1".into(),
                    candidate_sources: incomplete_sources,
                },
                &config,
                &evaluation_input("transport:dispatch"),
                |_| {
                    Ok(BehaviorValidation {
                        status: "passed".into(),
                        cases_passed: 6,
                        cases_total: 6,
                    })
                },
            )
            .expect_err("an incomplete base source set must fail closed");
        assert_eq!(
            error.kind,
            ail_compiler::ArchitectureRequestErrorKind::InvalidRevision
        );
        assert_eq!(error.message, "base architecture is incomplete");
        assert_eq!(workspace.current_revision_id(), "r1");
        assert!(workspace.revision("r2").is_none());
    }
}
