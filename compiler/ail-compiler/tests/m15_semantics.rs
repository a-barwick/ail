use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ail_compiler::{
    CapabilityEnvironment, CapabilityInterface, CapabilityOperation, CheckResult, DiagnosticValue,
    TypeCheckStatus, check_source,
};
use serde_json::Value;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/fixtures")
}

fn fixture(name: &str) -> Value {
    let path = fixtures_dir().join(name);
    serde_json::from_str(&fs::read_to_string(path).expect("fixture is readable"))
        .expect("fixture is valid JSON")
}

fn source(fixture: &Value) -> &str {
    fixture["input"]["source"]
        .as_str()
        .expect("fixture source is a string")
}

fn environment(fixture: &Value) -> CapabilityEnvironment {
    let mut environment = CapabilityEnvironment::new();
    let capabilities = fixture["input"]["environment"]["capabilities"]
        .as_object()
        .expect("fixture capabilities are an object");
    for (interface_name, interface_value) in capabilities {
        let mut interface = CapabilityInterface::new();
        for (operation_name, operation_value) in interface_value
            .as_object()
            .expect("capability operations are an object")
        {
            let parameters = operation_value["parameters"]
                .as_array()
                .expect("operation parameters are an array")
                .iter()
                .map(|parameter| parameter.as_str().expect("operation parameter is a string"))
                .collect::<Vec<_>>();
            let result = operation_value["result"]
                .as_str()
                .expect("operation result is a string");
            interface
                .insert_operation(operation_name, CapabilityOperation::new(parameters, result));
        }
        environment.insert_interface(interface_name, interface);
    }
    environment
}

fn check_fixture(name: &str) -> (Value, CheckResult) {
    let fixture = fixture(name);
    let revision_id = fixture["input"]["revision"]["revision_id"]
        .as_str()
        .expect("revision ID is a string");
    let result = check_source(source(&fixture), revision_id, &environment(&fixture));
    (fixture, result)
}

fn expected_value(value: &Value) -> DiagnosticValue {
    if let Some(text) = value.as_str() {
        return DiagnosticValue::Text(text.to_owned());
    }
    DiagnosticValue::TextList(
        value
            .as_array()
            .expect("diagnostic fixture value is a string or array")
            .iter()
            .map(|item| {
                item.as_str()
                    .expect("diagnostic array item is a string")
                    .to_owned()
            })
            .collect(),
    )
}

fn expected_fields(value: &Value) -> BTreeMap<String, DiagnosticValue> {
    value
        .as_object()
        .expect("diagnostic fields are an object")
        .iter()
        .map(|(key, value)| (key.clone(), expected_value(value)))
        .collect()
}

#[test]
fn every_m11_fixture_has_the_declared_parse_or_static_status() {
    for entry in fs::read_dir(fixtures_dir()).expect("fixture directory is readable") {
        let entry = entry.expect("fixture directory entry is readable");
        if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_str().expect("fixture path is UTF-8");
        let (fixture, result) = check_fixture(name);
        let category = fixture["category"]
            .as_str()
            .expect("fixture category is a string");
        if matches!(category, "rename" | "stale-revision") {
            continue;
        }
        let expected_status = fixture["expected"]["type_result"]["status"]
            .as_str()
            .expect("type status is a string");
        let actual_status = match result.type_result.status {
            TypeCheckStatus::Ok => "ok",
            TypeCheckStatus::Error => "error",
            TypeCheckStatus::NotRun => "not-run",
        };
        assert_eq!(actual_status, expected_status, "{name}");

        if expected_status == "not-run" {
            assert!(!result.parse_diagnostics.is_empty(), "{name}");
            assert!(result.diagnostics.is_empty(), "{name}");
        } else {
            assert!(result.parse_diagnostics.is_empty(), "{name}");
            assert_eq!(
                result.canonical_source.as_deref(),
                fixture["expected"]["canonical_source"].as_str(),
                "{name}"
            );
        }
    }
}

#[test]
fn inferred_let_types_and_explicit_function_types_are_elaborated() {
    let (fixture, result) = check_fixture("positive.json");
    assert_eq!(result.type_result.status, TypeCheckStatus::Ok);
    assert!(result.diagnostics.is_empty());

    let expected = &fixture["expected"]["type_result"]["facts"];
    for expected_fact in expected.as_array().expect("facts are an array") {
        let expected_explicit = expected_fact["explicit_type"].as_str();
        let expected_inferred = expected_fact["inferred_type"].as_str();
        assert!(
            result.type_result.facts.iter().any(|fact| {
                fact.explicit_type.as_deref() == expected_explicit
                    && fact.inferred_type.as_deref() == expected_inferred
            }),
            "missing expected type fact: {expected_fact}"
        );
    }
    assert!(result.type_result.facts.iter().all(|fact| {
        fact.handle.revision_id
            == fixture["input"]["revision"]["revision_id"]
                .as_str()
                .expect("revision ID is a string")
    }));
}

#[test]
fn record_field_mismatch_matches_the_required_structured_diagnostic() {
    let (fixture, result) = check_fixture("type-error.json");
    assert_eq!(result.type_result.status, TypeCheckStatus::Error);
    assert!(result.type_result.facts.is_empty());
    let diagnostic = result.diagnostics.first().expect("type diagnostic exists");
    let expected = &fixture["expected"]["primary_diagnostic"];
    assert_eq!(diagnostic.code, expected["code"].as_str().unwrap());
    assert_eq!(diagnostic.category, expected["category"].as_str().unwrap());
    assert_eq!(
        diagnostic.revision_id,
        fixture["input"]["revision"]["revision_id"]
    );
    assert_eq!(diagnostic.expected, expected_fields(&expected["expected"]));
    assert_eq!(diagnostic.actual, expected_fields(&expected["actual"]));
    assert_eq!(diagnostic.related_handles.len(), 2);
    assert_eq!(diagnostic.causal_chain.len(), 1);
    assert_eq!(diagnostic.causal_chain[0].step, "check-record-initializer");
    assert_eq!(
        diagnostic.causal_chain[0].handle.kind,
        diagnostic.primary_handle.kind
    );
    assert_ne!(
        diagnostic.causal_chain[0].handle.local_id,
        diagnostic.primary_handle.local_id
    );
}

#[test]
fn undeclared_capability_effect_preserves_type_facts_and_reports_causality() {
    let (fixture, result) = check_fixture("capability-error.json");
    assert_eq!(result.type_result.status, TypeCheckStatus::Ok);
    assert_eq!(result.type_result.facts.len(), 2);
    let diagnostic = result
        .diagnostics
        .first()
        .expect("capability diagnostic exists");
    let expected = &fixture["expected"]["primary_diagnostic"];
    assert_eq!(diagnostic.code, expected["code"].as_str().unwrap());
    assert_eq!(diagnostic.category, expected["category"].as_str().unwrap());
    assert_eq!(diagnostic.expected, expected_fields(&expected["expected"]));
    assert_eq!(diagnostic.actual, expected_fields(&expected["actual"]));
    assert_eq!(diagnostic.related_handles.len(), 2);
    assert_eq!(
        diagnostic
            .causal_chain
            .iter()
            .map(|step| step.step.as_str())
            .collect::<Vec<_>>(),
        ["resolve-capability-operation", "compare-declared-effects"]
    );
}

#[test]
fn local_names_are_forward_only_and_capability_arguments_require_exact_types() {
    let source = "record Job {\n  job_id: Text;\n}\n\nfn broken(jobs: capability JobsStore) -> Unit effects { jobs.insert } {\n  let job = Job { job_id: later };\n  let later = 1;\n  jobs.insert(later)\n}\n";
    let mut interface = CapabilityInterface::new();
    interface.insert_operation("insert", CapabilityOperation::new(["Job"], "Unit"));
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("JobsStore", interface);

    let result = check_source(source, "synthetic-r1", &environment);
    assert_eq!(result.type_result.status, TypeCheckStatus::Error);
    assert_eq!(result.diagnostics[0].code, "AIL.NAME.UNRESOLVED");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "AIL.TYPE.CAPABILITY_ARGUMENT")
    );
}

#[test]
fn closed_records_and_variants_require_their_declared_shape() {
    let source = "record Job {\n  job_id: Text;\n}\n\nvariant Outcome {\n  Created(Job);\n  Empty;\n}\n\nfn make() -> Outcome {\n  let incomplete = Job { };\n  let wrong_payload = Outcome::Created(1);\n  Outcome::Empty(unknown)\n}\n";
    let result = check_source(source, "shape-r1", &CapabilityEnvironment::new());

    assert_eq!(result.type_result.status, TypeCheckStatus::Error);
    let codes = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    assert!(codes.contains(&"AIL.NAME.UNRESOLVED"));
    assert!(codes.contains(&"AIL.TYPE.RECORD_FIELD_SET"));
    assert!(codes.contains(&"AIL.TYPE.VARIANT_PAYLOAD_MISMATCH"));
    assert_eq!(codes[0], "AIL.NAME.UNRESOLVED");
}

#[test]
fn duplicate_declarations_are_reported_before_type_or_capability_problems() {
    let source = "record Job {\n  job_id: Text;\n  job_id: Text;\n}\n\nfn make(job: Job, job: Job) -> Job {\n  let job = Job { job_id: 1 };\n  job\n}\n";
    let result = check_source(source, "duplicate-r1", &CapabilityEnvironment::new());

    assert_eq!(result.type_result.status, TypeCheckStatus::Error);
    assert_eq!(result.diagnostics[0].code, "AIL.NAME.DUPLICATE_DECLARATION");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "AIL.TYPE.FIELD_MISMATCH")
    );
}
