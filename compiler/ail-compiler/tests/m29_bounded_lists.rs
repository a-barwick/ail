use std::cell::Cell;
use std::fs;
use std::path::Path;

use ail_compiler::{
    CandidateChangeRequest, CapabilityEnvironment, CapabilityInterface, CapabilityOperation,
    CapabilityProvider, ChangeResponse, DiagnosticValue, EvolutionCoverage, EvolutionSource,
    EvolutionWorkspace, ExecutionResponse, ImpactRequest, InspectionRequest, ProposedSchemaChange,
    RenameRequest, RenameResponse, RuntimeFault, RuntimeValue, TypeCheckStatus, Workspace,
    check_source, format_source,
};

const DOMAIN: &str = include_str!("../../examples/batch-cancellation/domain.ail");
const SINGLE: &str = include_str!("../../examples/batch-cancellation/single.ail");
const SERVICE: &str = include_str!("../../examples/batch-cancellation/service.ail");

fn environment() -> CapabilityEnvironment {
    let mut store = CapabilityInterface::new();
    store.insert_operation(
        "cancel",
        CapabilityOperation::new(
            ["cancellation.domain.JobId"],
            "cancellation.domain.CancelOutcome",
        ),
    );
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("JobsStore", store);
    environment
}

fn sources() -> Vec<EvolutionSource> {
    vec![
        EvolutionSource::new("domain.ail", DOMAIN),
        EvolutionSource::new("service.ail", SERVICE),
        EvolutionSource::new("single.ail", SINGLE),
    ]
}

fn coverage() -> EvolutionCoverage {
    EvolutionCoverage {
        declared_complete: true,
        ..EvolutionCoverage::default()
    }
}

fn workspace() -> EvolutionWorkspace {
    EvolutionWorkspace::new(
        "bounded-cancellation",
        "r1",
        sources(),
        &environment(),
        coverage(),
    )
    .expect("the batch cancellation service compiles")
}

fn transaction_environment() -> CapabilityEnvironment {
    let mut store = CapabilityInterface::new();
    store.insert_operation(
        "insert_if_absent",
        CapabilityOperation::new(["Job"], "InsertOutcome"),
    );
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("JobsStore", store);
    environment
}

fn transaction_sources(revision: &str, map_function: &str) -> Vec<EvolutionSource> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/evolution-fixtures")
        .join(revision);
    let mut paths = fs::read_dir(root)
        .expect("evolution fixture directory is readable")
        .map(|entry| entry.expect("evolution fixture entry is readable").path())
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let mut source = fs::read_to_string(&path).expect("evolution fixture is readable");
            if name == "adapters.ail" {
                source.push('\n');
                source.push_str(map_function);
            }
            EvolutionSource::new(name, source)
        })
        .collect()
}

fn transaction_impact_request() -> ImpactRequest {
    ImpactRequest {
        base_revision_id: "schema-r1".to_owned(),
        change: ProposedSchemaChange {
            kind: "add-required-field-with-version-successor".to_owned(),
            subject_identity: "job.create-request.v1".to_owned(),
            successor_identity: "job.create-request.v2".to_owned(),
            member_display_name: "priority".to_owned(),
            member_identity: "priority".to_owned(),
            member_type: "Priority".to_owned(),
        },
    }
}

fn transaction_impact_id(role: &str) -> String {
    match role {
        "request-schema" => "request-contract",
        "stored-schema" => "stored-contract",
        "closed-priority-schema" => "priority-contract",
        "v1-request-adapter" => "request-adapter",
        "v1-stored-adapter" => "stored-adapter",
        "handler" => "handler-construction",
        "store-capability" => "store-contract",
        "persisted-encoder" => "stored-encoder",
        "v1-response-projection" => "v1-projection",
        "v2-response-projection" => "v2-projection",
        "v2-request-fixture" => "v2-fixture",
        "completion-evidence" => "completion-artifact",
        role => role,
    }
    .to_owned()
}

fn job_id(value: &str) -> RuntimeValue {
    RuntimeValue::record(
        "cancellation.domain.JobId",
        [("value", RuntimeValue::Text(value.to_owned()))],
    )
}

fn outcome(case: &str) -> RuntimeValue {
    RuntimeValue::variant("cancellation.domain.CancelOutcome", case, None)
}

#[derive(Default)]
struct JobsStore {
    supports_checks: Cell<usize>,
    calls: Vec<String>,
}

impl CapabilityProvider for JobsStore {
    fn supports(&self, receiver: &str, interface: &str) -> bool {
        self.supports_checks.set(self.supports_checks.get() + 1);
        receiver == "store" && interface == "JobsStore"
    }

    fn call(
        &mut self,
        _receiver: &str,
        _interface: &str,
        operation: &str,
        arguments: &[RuntimeValue],
    ) -> Result<RuntimeValue, RuntimeFault> {
        assert_eq!(operation, "cancel");
        let value = arguments[0]
            .field("value")
            .and_then(|value| match value {
                RuntimeValue::Text(value) => Some(value.clone()),
                _ => None,
            })
            .expect("the static contract supplies a JobId");
        self.calls.push(value.clone());
        match value.as_str() {
            "fault" => Err(RuntimeFault::new(
                "TEST.JOBS_STORE.UNAVAILABLE",
                ail_compiler::Span::empty(0),
                [("operation", "cancel")],
                [("job_id", value)],
            )),
            "missing" => Ok(outcome("NotFound")),
            "done" => Ok(outcome("AlreadyFinished")),
            "unavailable" => Ok(outcome("Unavailable")),
            _ => Ok(outcome("Cancelled")),
        }
    }
}

fn completed(response: ExecutionResponse) -> ail_compiler::ExecutionSuccess {
    let ExecutionResponse::Completed(success) = response else {
        panic!("execution must complete: {response:#?}");
    };
    success
}

fn failed(response: ExecutionResponse) -> ail_compiler::ExecutionFailure {
    let ExecutionResponse::Failed(failure) = response else {
        panic!("execution must fail: {response:#?}");
    };
    failure
}

#[test]
fn list_and_map_syntax_is_canonical_typed_and_contextual() {
    for source in [DOMAIN, SINGLE, SERVICE] {
        assert_eq!(format_source(source).unwrap(), source);
    }

    let contextual = "fn map(in: Text) -> Text { in }";
    assert_eq!(
        format_source(contextual).unwrap(),
        "fn map(in: Text) -> Text {\n  in\n}\n"
    );

    let source = concat!(
        "record Item { value: Text; }\n\n",
        "fn values(items: List<Item, 8>) -> List<Text, 8> {\n",
        "  map item in items {\n",
        "    item.value\n",
        "  }\n",
        "}\n",
    );
    let checked = check_source(source, "lists", &CapabilityEnvironment::new());
    assert_eq!(checked.type_result.status, TypeCheckStatus::Ok);
    assert!(checked.diagnostics.is_empty());
}

#[test]
fn invalid_bounds_elements_and_map_sources_have_stable_diagnostics() {
    let maximum = check_source(
        "fn valid(items: List<Text, 4294967295>) -> List<Text, 4294967295> { items }",
        "maximum-list-bound",
        &CapabilityEnvironment::new(),
    );
    assert_eq!(maximum.type_result.status, TypeCheckStatus::Ok);
    assert!(maximum.diagnostics.is_empty());

    let oversized_spelling = "99999999999999999999999999999999999999999999999999";
    let cases = [
        (
            "fn bad(items: List<Text, 0>) -> List<Text, 0> { items }".to_owned(),
            "AIL.TYPE.LIST_BOUND",
        ),
        (
            "fn bad(items: List<Text, 4294967296>) -> List<Text, 4294967296> { items }"
                .to_owned(),
            "AIL.TYPE.LIST_BOUND",
        ),
        (
            format!(
                "fn bad(items: List<Text, {oversized_spelling}>) -> List<Text, {oversized_spelling}> {{ items }}"
            ),
            "AIL.TYPE.LIST_BOUND",
        ),
        (
            "fn bad(items: List<List<Text, 2>, 2>) -> List<Text, 2> { map item in items { \"x\" } }"
                .to_owned(),
            "AIL.TYPE.LIST_ELEMENT",
        ),
        (
            "fn bad(value: Text) -> List<Text, 2> { map item in value { item } }".to_owned(),
            "AIL.TYPE.MAP_SOURCE",
        ),
    ];
    for (source, code) in cases {
        let checked = check_source(&source, "invalid-list", &CapabilityEnvironment::new());
        assert_eq!(checked.type_result.status, TypeCheckStatus::Error);
        assert!(
            checked
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code),
            "missing {code} in {:#?}",
            checked.diagnostics
        );
    }

    let oversized = check_source(
        &format!(
            "fn bad(items: List<Text, {oversized_spelling}>) -> List<Text, {oversized_spelling}> {{ items }}"
        ),
        "oversized-list-bound-spelling",
        &CapabilityEnvironment::new(),
    );
    assert!(oversized.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "AIL.TYPE.LIST_BOUND"
            && diagnostic.actual.get("bound")
                == Some(&DiagnosticValue::Text(oversized_spelling.to_owned()))
    }));
}

#[test]
fn batch_cancellation_is_empty_ordered_aligned_and_duplicate_preserving() {
    let workspace = workspace();
    let mut store = JobsStore::default();
    let empty = completed(workspace.execute(
        "r1",
        "cancellation.service.cancel_batch",
        vec![RuntimeValue::list([])],
        &mut store,
    ));
    assert_eq!(empty.value, RuntimeValue::list([]));
    assert!(empty.calls.is_empty());
    assert!(store.calls.is_empty());

    let values = ["first", "missing", "done", "first", "unavailable"];
    let result = completed(workspace.execute(
        "r1",
        "cancellation.service.cancel_batch",
        vec![RuntimeValue::list(values.map(job_id))],
        &mut store,
    ));
    assert_eq!(
        result.value,
        RuntimeValue::list([
            outcome("Cancelled"),
            outcome("NotFound"),
            outcome("AlreadyFinished"),
            outcome("Cancelled"),
            outcome("Unavailable"),
        ])
    );
    assert_eq!(store.calls, values);
    assert_eq!(result.calls.len(), values.len());
    assert!(
        result.calls.iter().zip(values).all(|(call, expected)| {
            call.arguments == [job_id(expected)] && call.result.is_some()
        })
    );
}

#[test]
fn external_lists_are_completely_validated_before_capability_checks_or_calls() {
    let workspace = workspace();
    let mut oversize_store = JobsStore::default();
    let oversize = failed(workspace.execute(
        "r1",
        "cancellation.service.cancel_batch",
        vec![RuntimeValue::list(
            (0..33).map(|index| job_id(&index.to_string())),
        )],
        &mut oversize_store,
    ));
    assert_eq!(oversize.fault.code, "AIL.RUNTIME.LIST_CARDINALITY");
    assert_eq!(oversize.fault.expected["maximum"], "32");
    assert_eq!(oversize.fault.actual["count"], "33");
    assert!(oversize.calls.is_empty());
    assert!(oversize_store.calls.is_empty());
    assert_eq!(oversize_store.supports_checks.get(), 0);

    let mut malformed_store = JobsStore::default();
    let malformed = failed(workspace.execute(
        "r1",
        "cancellation.service.cancel_batch",
        vec![RuntimeValue::list([
            job_id("valid"),
            RuntimeValue::Text("not-a-job-id".to_owned()),
        ])],
        &mut malformed_store,
    ));
    assert_eq!(malformed.fault.code, "AIL.RUNTIME.LIST_ELEMENT");
    assert_eq!(malformed.fault.actual["index"], "1");
    assert!(malformed.calls.is_empty());
    assert!(malformed_store.calls.is_empty());
    assert_eq!(malformed_store.supports_checks.get(), 0);

    let mut nested_store = JobsStore::default();
    let malformed_field = failed(workspace.execute(
        "r1",
        "cancellation.service.cancel_batch",
        vec![RuntimeValue::list([RuntimeValue::record(
            "cancellation.domain.JobId",
            [("value", RuntimeValue::Int(7))],
        )])],
        &mut nested_store,
    ));
    assert_eq!(malformed_field.fault.code, "AIL.RUNTIME.LIST_ELEMENT");
    assert_eq!(malformed_field.fault.actual["index"], "0");
    assert_eq!(malformed_field.fault.actual["actual_type"], "Int");
    assert_eq!(
        malformed_field.fault.actual["value_path"],
        "argument[0][0].value"
    );
    assert!(malformed_field.calls.is_empty());
    assert!(nested_store.calls.is_empty());
    assert_eq!(nested_store.supports_checks.get(), 0);
}

#[test]
fn exact_bound_completes_and_provider_faults_stop_at_the_failing_index() {
    let workspace = workspace();
    let mut full_store = JobsStore::default();
    let full = completed(workspace.execute(
        "r1",
        "cancellation.service.cancel_batch",
        vec![RuntimeValue::list(
            (0..32).map(|index| job_id(&index.to_string())),
        )],
        &mut full_store,
    ));
    let RuntimeValue::List(values) = full.value else {
        panic!("map result must be a list");
    };
    assert_eq!(values.len(), 32);
    assert_eq!(full.calls.len(), 32);

    let mut faulting_store = JobsStore::default();
    let failure = failed(workspace.execute(
        "r1",
        "cancellation.service.cancel_batch",
        vec![RuntimeValue::list([
            job_id("before"),
            job_id("fault"),
            job_id("after"),
        ])],
        &mut faulting_store,
    ));
    assert_eq!(failure.fault.code, "TEST.JOBS_STORE.UNAVAILABLE");
    assert_eq!(failure.fault.actual["map_index"], "1");
    assert_eq!(faulting_store.calls, ["before", "fault"]);
    assert_eq!(failure.calls.len(), 2);
    assert!(failure.calls[0].result.is_some());
    assert!(failure.calls[1].result.is_none());
}

#[test]
fn map_bodies_preserve_imported_transitive_effects() {
    let missing_effect = sources()
        .into_iter()
        .map(|source| {
            if source.path == "service.ail" {
                EvolutionSource::new(
                    source.path,
                    source.source.replace(" effects { store.cancel }", ""),
                )
            } else {
                source
            }
        })
        .collect();
    let missing_effect_failure = EvolutionWorkspace::new(
        "bounded-cancellation",
        "invalid-effect",
        missing_effect,
        &environment(),
        coverage(),
    )
    .expect_err("map bodies preserve imported transitive effects");
    assert!(
        missing_effect_failure
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "AIL.CAPABILITY.UNDECLARED_TRANSITIVE_EFFECT" })
    );
}

#[test]
fn aliases_element_identities_source_order_and_revisions_remain_deterministic() {
    let forward = workspace();
    let mut reversed_sources = sources();
    reversed_sources.reverse();
    let reverse = EvolutionWorkspace::new(
        "bounded-cancellation",
        "r1",
        reversed_sources,
        &environment(),
        coverage(),
    )
    .expect("source vector order does not affect linking");
    assert_eq!(
        forward.revision("r1").unwrap().source_set_digest,
        reverse.revision("r1").unwrap().source_set_digest
    );
    assert_eq!(forward.graph("r1"), reverse.graph("r1"));
    assert!(
        forward.graph("r1").unwrap().iter().any(|edge| {
            edge.kind == "signature-input" && edge.target == "cancellation.job-id.v1"
        })
    );
    let inspection = forward
        .inspect_function("r1", "cancellation.service.cancel_batch")
        .unwrap();
    assert_eq!(inspection.module_identity, "cancellation.service");
    assert_eq!(
        inspection.parameters[0].value_type,
        "List<cancellation.domain.JobId, 32>"
    );
    let input_list = inspection.parameters[0].bounded_list.as_ref().unwrap();
    assert_eq!(input_list.element_type, "cancellation.domain.JobId");
    assert_eq!(
        input_list.element_identity.as_deref(),
        Some("cancellation.job-id.v1")
    );
    assert_eq!(input_list.max_length, 32);
    assert_eq!(
        inspection.result_type,
        "List<cancellation.domain.CancelOutcome, 32>"
    );
    assert_eq!(inspection.effects, ["store.cancel"]);
    assert_eq!(inspection.capabilities, ["store:JobsStore"]);
    assert!(
        inspection
            .dependencies
            .contains(&"cancellation.single.cancel_one".to_owned())
    );

    let mut retained = workspace();
    let r2_sources = sources()
        .into_iter()
        .map(|source| {
            if source.path == "service.ail" {
                EvolutionSource::new(source.path, source.source.replace(", 32>", ", 2>"))
            } else {
                source
            }
        })
        .collect();
    retained
        .retain_revision(
            "r2",
            Some("r1".to_owned()),
            r2_sources,
            &environment(),
            coverage(),
        )
        .expect("the smaller exact bound is a valid retained revision");
    assert_eq!(retained.current_revision_id(), "r1");
    let input = RuntimeValue::list([job_id("one"), job_id("two"), job_id("three")]);
    assert!(matches!(
        retained.execute("r1", "cancellation.service.cancel_batch", vec![input.clone()], &mut JobsStore::default()),
        ExecutionResponse::Completed(result) if result.revision_id == "r1"
    ));
    assert!(matches!(
        retained.execute("r2", "cancellation.service.cancel_batch", vec![input], &mut JobsStore::default()),
        ExecutionResponse::Failed(result)
            if result.revision_id == "r2" && result.fault.code == "AIL.RUNTIME.LIST_CARDINALITY"
    ));
}

#[test]
fn inspection_exposes_list_bounds_map_types_binders_and_dependencies() {
    let source = concat!(
        "record Item { value: Text; }\n\n",
        "fn values(items: List<Item, 8>) -> List<Text, 8> {\n",
        "  map item in items {\n",
        "    item.value\n",
        "  }\n",
        "}\n",
    );
    let workspace = Workspace::new(
        "list-inspection",
        "r1",
        "main.ail",
        source,
        CapabilityEnvironment::new(),
    )
    .unwrap();
    let inspected = workspace
        .handles("r1")
        .unwrap()
        .into_iter()
        .map(|handle| {
            workspace
                .inspect(InspectionRequest {
                    revision_id: "r1".to_owned(),
                    handle,
                })
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert!(
        inspected.iter().any(|node| {
            node.semantic_kind == "map"
                && node.inferred_type.as_deref() == Some("List<Text, 8>")
                && node.dependencies == ["Item", "Text"]
        }),
        "{inspected:#?}"
    );
    assert!(inspected.iter().any(|node| {
        node.semantic_kind == "map-binding" && node.inferred_type.as_deref() == Some("Item")
    }));
    assert_eq!(
        inspected
            .iter()
            .filter(|node| node.semantic_kind == "list-bound")
            .count(),
        2
    );
}

#[test]
fn map_binding_rename_edits_the_binder_and_references_not_the_contextual_keyword() {
    let source = concat!(
        "record Item {\n",
        "  value: Text;\n",
        "}\n\n",
        "fn values(items: List<Item, 8>) -> List<Text, 8> {\n",
        "  map item in items {\n",
        "    item.value\n",
        "  }\n",
        "}\n",
    );
    let mut workspace = Workspace::new(
        "map-binding-rename",
        "r1",
        "main.ail",
        source,
        CapabilityEnvironment::new(),
    )
    .unwrap();
    let binding = workspace
        .handles("r1")
        .unwrap()
        .into_iter()
        .find(|handle| {
            workspace
                .inspect(InspectionRequest {
                    revision_id: "r1".to_owned(),
                    handle: handle.clone(),
                })
                .is_ok_and(|node| node.semantic_kind == "map-binding")
        })
        .expect("map binding is indexed");
    let response = workspace.rename(RenameRequest {
        base_revision_id: "r1".to_owned(),
        handle: binding,
        new_name: "entry".to_owned(),
    });
    let RenameResponse::Committed(success) = response else {
        panic!("map binding rename must commit: {response:#?}");
    };
    assert_eq!(success.edits.len(), 2);
    assert!(
        success
            .edits
            .iter()
            .all(|edit| &source[edit.span.start..edit.span.end] == "item")
    );
    assert_eq!(
        workspace.source(&success.revision.revision_id),
        Some(concat!(
            "record Item {\n",
            "  value: Text;\n",
            "}\n\n",
            "fn values(items: List<Item, 8>) -> List<Text, 8> {\n",
            "  map entry in items {\n",
            "    entry.value\n",
            "  }\n",
            "}\n",
        ))
    );
}

#[test]
fn atomic_candidate_validation_accepts_map_and_rejects_invalid_map_without_publication() {
    const VALID_MAP: &str = concat!(
        "fn map_text(items: List<Text, 4>) -> List<Text, 4> {\n",
        "  map item in items {\n",
        "    item\n",
        "  }\n",
        "}\n",
    );
    const INVALID_MAP: &str = concat!(
        "fn map_text(items: List<Text, 4>) -> List<Text, 4> {\n",
        "  map item in \"not-a-list\" {\n",
        "    item\n",
        "  }\n",
        "}\n",
    );

    let base_sources = transaction_sources("r1", VALID_MAP);
    let environment = transaction_environment();
    let mut accepted = EvolutionWorkspace::new(
        "m29-candidate",
        "schema-r1",
        base_sources.clone(),
        &environment,
        coverage(),
    )
    .expect("the M29 transaction base compiles");
    let impact = accepted.impact(transaction_impact_request()).unwrap();
    let required_impact_ids = impact
        .must_change
        .iter()
        .map(|entry| transaction_impact_id(&entry.role))
        .collect::<Vec<_>>();
    let response = accepted.validate_change(
        CandidateChangeRequest {
            base_revision_id: "schema-r1".to_owned(),
            candidate_sources: transaction_sources("r2", VALID_MAP),
            required_impact_ids: required_impact_ids.clone(),
        },
        &impact,
        |candidate| {
            let result = candidate.execute(
                "map_text",
                vec![RuntimeValue::list([
                    RuntimeValue::Text("first".to_owned()),
                    RuntimeValue::Text("second".to_owned()),
                ])],
                &mut JobsStore::default(),
            );
            assert!(matches!(
                result,
                ExecutionResponse::Completed(success)
                    if success.value == RuntimeValue::list([
                        RuntimeValue::Text("first".to_owned()),
                        RuntimeValue::Text("second".to_owned()),
                    ])
            ));
            Ok("map candidate executed".to_owned())
        },
    );
    assert!(matches!(response, ChangeResponse::Committed(_)));
    assert_eq!(accepted.current_revision_id(), "schema-r2");

    let mut rejected = EvolutionWorkspace::new(
        "m29-candidate",
        "schema-r1",
        base_sources,
        &environment,
        coverage(),
    )
    .expect("the M29 transaction base compiles");
    let impact = rejected.impact(transaction_impact_request()).unwrap();
    let response = rejected.validate_change(
        CandidateChangeRequest {
            base_revision_id: "schema-r1".to_owned(),
            candidate_sources: transaction_sources("r2", INVALID_MAP),
            required_impact_ids,
        },
        &impact,
        |_| Ok("must not execute".to_owned()),
    );
    let ChangeResponse::Rejected(failure) = response else {
        panic!("invalid map candidate must be rejected");
    };
    assert_eq!(failure.phase, "static");
    assert_eq!(failure.diagnostic.code, "AIL.PROTOCOL.VALIDATION_FAILED");
    assert_eq!(rejected.current_revision_id(), "schema-r1");
    assert!(rejected.revision("schema-r2").is_none());
}
