use std::fs;
use std::path::{Path, PathBuf};

use ail_compiler::{
    CandidateChangeRequest, CapabilityEnvironment, CapabilityInterface, CapabilityOperation,
    CapabilityProvider, ChangeResponse, EvolutionCoverage, EvolutionSource, EvolutionWorkspace,
    ExecutionResponse, ImpactRequest, ProposedSchemaChange, PublicBehaviorFailure, RuntimeFault,
    RuntimeValue, SourceArtifact, Span, UncheckedBoundary,
};
use serde_json::Value;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/evolution-fixtures")
}

fn fixture(name: &str) -> Value {
    serde_json::from_str(
        &fs::read_to_string(fixture_root().join(name)).expect("fixture is readable"),
    )
    .expect("fixture is JSON")
}

fn sources(revision: &str) -> Vec<EvolutionSource> {
    let root = fixture_root().join(revision);
    let mut paths = fs::read_dir(&root)
        .expect("fixture directory is readable")
        .map(|entry| entry.expect("directory entry").path())
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            EvolutionSource::new(name, fs::read_to_string(&path).expect("source is readable"))
        })
        .collect()
}

fn environment() -> CapabilityEnvironment {
    let mut jobs = CapabilityInterface::new();
    jobs.insert_operation(
        "insert_if_absent",
        CapabilityOperation::new(["Job"], "InsertOutcome"),
    );
    let mut clock = CapabilityInterface::new();
    clock.insert_operation(
        "now",
        CapabilityOperation::new(Vec::<String>::new(), "Text"),
    );
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("JobsStore", jobs);
    environment.insert_interface("Clock", clock);
    environment
}

fn coverage() -> EvolutionCoverage {
    EvolutionCoverage {
        declared_complete: true,
        unchecked: vec![UncheckedBoundary {
            identity: "external:job-api-clients".to_owned(),
            reason: "client source unavailable".to_owned(),
        }],
        artifacts: vec![SourceArtifact {
            path: "evolution-fixtures/transaction.json".to_owned(),
            role: "completion-evidence".to_owned(),
        }],
    }
}

fn workspace() -> EvolutionWorkspace {
    EvolutionWorkspace::new(
        "m19-job-service",
        "schema-r1",
        sources("r1"),
        &environment(),
        coverage(),
    )
    .expect("R1 source set is valid")
}

fn impact_request() -> ImpactRequest {
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

fn required_impact_ids() -> Vec<String> {
    fixture("transaction.json")["request"]["required_impact_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect()
}

fn request(candidate_sources: Vec<EvolutionSource>) -> CandidateChangeRequest {
    CandidateChangeRequest {
        base_revision_id: "schema-r1".to_owned(),
        candidate_sources,
        required_impact_ids: required_impact_ids(),
    }
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
            Span::empty(0),
            std::iter::empty::<(&str, &str)>(),
            std::iter::empty::<(&str, &str)>(),
        ))
    }
}

struct InsertedStore;

impl CapabilityProvider for InsertedStore {
    fn supports(&self, receiver: &str, interface: &str) -> bool {
        receiver == "jobs" && interface == "JobsStore"
    }

    fn call(
        &mut self,
        _receiver: &str,
        _interface: &str,
        operation: &str,
        arguments: &[RuntimeValue],
    ) -> Result<RuntimeValue, RuntimeFault> {
        assert_eq!(operation, "insert_if_absent");
        assert_eq!(arguments.len(), 1);
        assert_eq!(
            arguments[0].field("priority"),
            Some(&RuntimeValue::variant("Priority", "High", None))
        );
        Ok(RuntimeValue::variant("InsertOutcome", "Inserted", None))
    }
}

fn accepted_behavior(
    candidate: &ail_compiler::CandidateRevision<'_>,
) -> Result<String, PublicBehaviorFailure> {
    if candidate.revision().source_set_digest
        != "sha256:3f9b4ee6e2d6fd91a58fa8eeb5c0bd593f67212e3b89411fa22ba22f8eac5431"
    {
        return Err(PublicBehaviorFailure {
            failing_case: "candidate-source-set".to_owned(),
            location: "workspace:candidate".to_owned(),
        });
    }
    let request = match candidate.execute(
        "fixture_request_v2",
        vec![RuntimeValue::Bytes(vec![1, 2, 3])],
        &mut NoCapabilities,
    ) {
        ExecutionResponse::Completed(success) => success.value,
        ExecutionResponse::Failed(failure) => panic!("fixture execution failed: {failure:?}"),
    };
    let response = candidate.execute("create_job", vec![request], &mut InsertedStore);
    let ExecutionResponse::Completed(success) = response else {
        panic!("candidate service did not execute: {response:?}")
    };
    assert_eq!(success.calls.len(), 1);
    assert!(matches!(
        success.value,
        RuntimeValue::Variant { ref type_name, ref case, .. }
            if type_name == "CreateJobResult" && case == "Created"
    ));
    Ok("37-public-cases-pass".to_owned())
}

#[test]
fn complete_r1_to_r2_candidate_commits_with_frozen_evidence() {
    let expected = &fixture("transaction.json")["expected"];
    let mut workspace = workspace();
    let impact = workspace.impact(impact_request()).unwrap();
    let response = workspace.validate_change(request(sources("r2")), &impact, accepted_behavior);
    let ChangeResponse::Committed(success) = response else {
        panic!("candidate was rejected: {response:?}")
    };

    assert_eq!(success.status, "committed");
    assert_eq!(workspace.current_revision_id(), "schema-r2");
    assert_eq!(success.revision.revision_id, "schema-r2");
    assert_eq!(
        success.revision.parent_revision_id.as_deref(),
        Some("schema-r1")
    );
    assert_eq!(
        success.revision.source_set_digest,
        expected["source_set_digest"].as_str().unwrap()
    );
    assert_eq!(success.completion.edits.len(), 5);
    for (edit, expected_edit) in success
        .completion
        .edits
        .iter()
        .zip(expected["edits"].as_array().unwrap())
    {
        assert_eq!(edit.path, expected_edit["path"].as_str().unwrap());
        assert_eq!(
            edit.span.start,
            usize::try_from(expected_edit["start_utf8_byte"].as_u64().unwrap()).unwrap()
        );
        assert_eq!(
            edit.span.end,
            usize::try_from(expected_edit["end_utf8_byte"].as_u64().unwrap()).unwrap()
        );
        let replacement = fs::read_to_string(
            fixture_root().join(expected_edit["replacement_source"].as_str().unwrap()),
        )
        .unwrap();
        assert_eq!(edit.replacement, replacement);
    }
    let persistent = &success.completion.persistent_identities;
    for (actual, key) in [
        (&persistent.preserved, "preserved"),
        (&persistent.added, "added"),
        (&persistent.retired, "retired"),
    ] {
        assert_eq!(
            actual,
            &expected["persistent_identities"][key]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        );
    }
    let diff = &success.completion.semantic_diff;
    assert_eq!(diff.changes.len(), 7, "{diff:?}");
    for (actual, expected) in diff
        .changes
        .iter()
        .zip(expected["semantic_diff"]["changes"].as_array().unwrap())
    {
        assert_eq!(actual.kind, expected["kind"].as_str().unwrap());
        assert_eq!(actual.identity, expected["identity"].as_str().unwrap());
        assert_eq!(actual.before.as_deref(), expected["before"].as_str());
        assert_eq!(actual.after.as_deref(), expected["after"].as_str());
    }
    assert_eq!(diff.effect_summary.before, ["jobs.insert_if_absent"]);
    assert_eq!(diff.effect_summary.after, ["jobs.insert_if_absent"]);
    assert_eq!(diff.effect_summary.ordering, "unchanged");
    assert_eq!(diff.capability_summary.before, ["JobsStore"]);
    assert_eq!(diff.capability_summary.after, ["JobsStore"]);
    assert_eq!(diff.capability_summary.authority, "unchanged");
    assert_eq!(
        success.completion.validation.public_behavior,
        "37-public-cases-pass"
    );
    assert_eq!(success.completion.impact_report, impact);
    assert_eq!(success.completion.unchecked.len(), 1);
    assert!(!success.completion.identity_map.entries.is_empty());
    assert!(success.completion.identity_map.entries.iter().all(|entry| {
        entry.old_handle.revision_id == "schema-r1"
            && entry
                .new_handle
                .as_ref()
                .is_none_or(|handle| handle.revision_id == "schema-r2")
    }));
}

fn assert_rejection(
    response: ChangeResponse,
    phase: &str,
    code: &str,
    location: &str,
    workspace: &EvolutionWorkspace,
) {
    let ChangeResponse::Rejected(failure) = response else {
        panic!("candidate unexpectedly committed: {response:?}")
    };
    assert_eq!(failure.status, "rejected");
    assert_eq!(failure.phase, phase);
    assert_eq!(failure.diagnostic.code, code);
    assert_eq!(failure.diagnostic.primary_handle.local_id, location);
    assert!(failure.edits.is_empty());
    assert_eq!(workspace.current_revision_id(), failure.current_revision_id);
}

#[test]
fn every_frozen_rejection_is_atomic_and_structured() {
    let rejections = fixture("rejections.json");

    let mut committed = workspace();
    let impact = committed.impact(impact_request()).unwrap();
    let success = committed.validate_change(request(sources("r2")), &impact, accepted_behavior);
    assert!(matches!(success, ChangeResponse::Committed(_)));
    let stale = committed.validate_change(request(sources("r2")), &impact, accepted_behavior);
    assert_rejection(
        stale,
        "revision",
        rejections["cases"][0]["code"].as_str().unwrap(),
        rejections["cases"][0]["location"].as_str().unwrap(),
        &committed,
    );

    let mut missed = workspace();
    let impact = missed.impact(impact_request()).unwrap();
    let mut missed_request = request(sources("r2"));
    missed_request
        .required_impact_ids
        .retain(|id| id != "handler-construction");
    let response = missed.validate_change(missed_request, &impact, accepted_behavior);
    assert_rejection(
        response,
        "impact",
        rejections["cases"][1]["code"].as_str().unwrap(),
        rejections["cases"][1]["location"].as_str().unwrap(),
        &missed,
    );

    let mut incompatible = workspace();
    let impact = incompatible.impact(impact_request()).unwrap();
    let mut incompatible_sources = sources("r2");
    let contracts = incompatible_sources
        .iter_mut()
        .find(|source| source.path == "contracts.ail")
        .unwrap();
    contracts.source = contracts
        .source
        .replacen(
            "record CreateJobRequestV1 identity \"job.create-request.v1\"",
            "record CreateJobRequestV1",
            1,
        )
        .replacen(
            "record CreateJobRequest identity \"job.create-request.v2\"",
            "record CreateJobRequest identity \"job.create-request.v1\"",
            1,
        );
    let response =
        incompatible.validate_change(request(incompatible_sources), &impact, accepted_behavior);
    assert_rejection(
        response,
        "schema",
        rejections["cases"][2]["code"].as_str().unwrap(),
        rejections["cases"][2]["location"].as_str().unwrap(),
        &incompatible,
    );

    let mut effect = workspace();
    let impact = effect.impact(impact_request()).unwrap();
    let mut effect_sources = sources("r2");
    let service = effect_sources
        .iter_mut()
        .find(|source| source.path == "service.ail")
        .unwrap();
    service.source = service.source.replacen(
        "fn create_job(request: CreateJobRequest, jobs: capability JobsStore) -> CreateJobResult effects { jobs.insert_if_absent }",
        "fn create_job(request: CreateJobRequest, jobs: capability JobsStore, clock: capability Clock) -> CreateJobResult effects { jobs.insert_if_absent, clock.now }",
        1,
    );
    let response = effect.validate_change(request(effect_sources), &impact, accepted_behavior);
    assert_rejection(
        response,
        "capabilities",
        rejections["cases"][3]["code"].as_str().unwrap(),
        rejections["cases"][3]["location"].as_str().unwrap(),
        &effect,
    );

    let mut behavior = workspace();
    let impact = behavior.impact(impact_request()).unwrap();
    let response = behavior.validate_change(request(sources("r2")), &impact, |_| {
        Err(PublicBehaviorFailure {
            failing_case: "uc003-v1-request-created".to_owned(),
            location: "projections.ail#project_created_v1".to_owned(),
        })
    });
    assert_rejection(
        response,
        "public-behavior",
        rejections["cases"][4]["code"].as_str().unwrap(),
        rejections["cases"][4]["location"].as_str().unwrap(),
        &behavior,
    );
}

#[test]
fn incomplete_or_invalid_candidates_publish_nothing() {
    let mut incomplete = workspace();
    let impact = incomplete.impact(impact_request()).unwrap();
    let mut incomplete_sources = sources("r2");
    incomplete_sources.retain(|source| source.path != "projections.ail");
    let response =
        incomplete.validate_change(request(incomplete_sources), &impact, accepted_behavior);
    assert_rejection(
        response,
        "impact",
        "AIL.IMPACT.MISSED_CONSUMER",
        "workspace:sources",
        &incomplete,
    );

    let mut invalid = workspace();
    let impact = invalid.impact(impact_request()).unwrap();
    let mut invalid_sources = sources("r2");
    invalid_sources[0].source.push_str("fn broken(");
    let response = invalid.validate_change(request(invalid_sources), &impact, accepted_behavior);
    assert_rejection(
        response,
        "static",
        "AIL.PROTOCOL.VALIDATION_FAILED",
        "workspace:candidate",
        &invalid,
    );
}

#[test]
fn identical_transactions_return_identical_results() {
    let mut first = workspace();
    let mut second = workspace();
    let first_impact = first.impact(impact_request()).unwrap();
    let second_impact = second.impact(impact_request()).unwrap();
    let first = first.validate_change(request(sources("r2")), &first_impact, accepted_behavior);
    let second = second.validate_change(request(sources("r2")), &second_impact, accepted_behavior);
    assert_eq!(first, second);
}
