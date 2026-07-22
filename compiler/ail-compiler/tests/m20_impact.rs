use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use ail_compiler::{
    CapabilityEnvironment, CapabilityInterface, CapabilityOperation, EvolutionCoverage,
    EvolutionSource, EvolutionWorkspace, ImpactEntry, ImpactRequest, ProposedSchemaChange,
    SourceArtifact, UncheckedBoundary, relationship_kinds,
};
use serde_json::Value;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/evolution-fixtures")
}

fn fixture() -> Value {
    serde_json::from_str(
        &fs::read_to_string(fixture_root().join("workspace.json")).expect("fixture is readable"),
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

fn environment(job_type: &str) -> CapabilityEnvironment {
    let mut interface = CapabilityInterface::new();
    interface.insert_operation(
        "insert_if_absent",
        CapabilityOperation::new([job_type], "InsertOutcome"),
    );
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("JobsStore", interface);
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
        &environment("Job"),
        coverage(),
    )
    .expect("R1 source set is valid")
}

fn request(revision_id: &str) -> ImpactRequest {
    ImpactRequest {
        base_revision_id: revision_id.to_owned(),
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

#[test]
fn ordered_source_set_revision_matches_the_m19_digest() {
    let fixture = fixture();
    let workspace = workspace();
    let revision = workspace.revision("schema-r1").unwrap();
    assert_eq!(
        revision.source_set_digest,
        fixture["revisions"]["r1"]["source_set_digest"]
            .as_str()
            .unwrap()
    );
    let paths = revision
        .sources
        .iter()
        .map(|source| source.path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        [
            "adapters.ail",
            "contracts.ail",
            "persistence.ail",
            "projections.ail",
            "service.ail"
        ]
    );
    assert!(
        workspace
            .sources("schema-r1")
            .unwrap()
            .iter()
            .all(|source| source.source.ends_with('\n'))
    );
}

#[test]
fn stable_identities_are_distinct_from_revision_handles() {
    let workspace = workspace();
    let identities = workspace.identities("schema-r1").unwrap();
    let request = identities
        .iter()
        .find(|identity| identity.identity == "job.create-request.v1")
        .unwrap();
    assert_eq!(request.display_name, "CreateJobRequest");
    assert_eq!(request.handle.revision_id, "schema-r1");
    assert_ne!(request.identity, request.handle.local_id);
    assert!(identities.iter().any(|identity| {
        identity.identity == "job.create-request.v1/job-id"
            && identity.parent_identity.as_deref() == Some("job.create-request.v1")
    }));
}

#[test]
fn semantic_graph_exercises_every_accepted_relationship_in_order() {
    let workspace = workspace();
    let graph = workspace.graph("schema-r1").unwrap();
    let exercised = graph.iter().map(|edge| edge.kind).collect::<BTreeSet<_>>();
    assert_eq!(
        exercised,
        relationship_kinds()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
    );
    assert!(graph.windows(2).all(|window| {
        let left = &window[0];
        let right = &window[1];
        (
            left.site.path.as_bytes(),
            left.site.span.start,
            relationship_kinds()
                .iter()
                .position(|kind| *kind == left.kind)
                .unwrap(),
            left.target.as_str(),
            left.site.handle.local_id.as_str(),
        ) <= (
            right.site.path.as_bytes(),
            right.site.span.start,
            relationship_kinds()
                .iter()
                .position(|kind| *kind == right.kind)
                .unwrap(),
            right.target.as_str(),
            right.site.handle.local_id.as_str(),
        )
    }));
}

fn expected_entries(value: &Value) -> Vec<ImpactEntry> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| ImpactEntry {
            location: entry["location"].as_str().unwrap().to_owned(),
            role: entry["role"].as_str().unwrap().to_owned(),
            reason: entry["reason"].as_str().unwrap().to_owned(),
            path: entry["path"]
                .as_array()
                .unwrap()
                .iter()
                .map(|part| part.as_str().unwrap().to_owned())
                .collect(),
        })
        .collect()
}

#[test]
fn impact_query_matches_the_exact_m19_categories() {
    let fixture = fixture();
    let expected = &fixture["impact"]["expected"];
    let report = workspace().impact(request("schema-r1")).unwrap();
    assert_eq!(
        report.must_change,
        expected_entries(&expected["must_change"])
    );
    assert_eq!(report.review, expected_entries(&expected["review"]));
    assert_eq!(report.unchecked.len(), 1);
    assert_eq!(
        report.analyzed_paths,
        expected["analyzed_paths"]
            .as_array()
            .unwrap()
            .iter()
            .map(|path| path.as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(report.effect_summary.capabilities, "unchanged");
    assert_eq!(report.effect_summary.effects, "unchanged");
    assert_eq!(report.effect_summary.ordering, "unchanged");
}

#[test]
fn requests_are_revision_bound_repeatable_and_parent_safe() {
    let mut workspace = workspace();
    let first = workspace.impact(request("schema-r1")).unwrap();
    let second = workspace.impact(request("schema-r1")).unwrap();
    assert_eq!(first, second);

    workspace
        .retain_revision(
            "schema-r2",
            Some("schema-r1".to_owned()),
            sources("r2"),
            &environment("Job"),
            coverage(),
        )
        .expect("R2 can be retained without changing the current revision");
    assert_eq!(workspace.current_revision_id(), "schema-r1");
    assert_eq!(first, workspace.impact(request("schema-r1")).unwrap());

    let stale = workspace.impact(request("unknown-r1")).unwrap_err();
    assert_eq!(stale.code, "AIL.PROTOCOL.STALE_REVISION");
}

#[test]
fn incomplete_coverage_cannot_return_a_clean_report() {
    let workspace = EvolutionWorkspace::new(
        "m19-job-service",
        "schema-r1",
        sources("r1"),
        &environment("Job"),
        EvolutionCoverage::default(),
    )
    .expect("source itself remains valid");
    let failure = workspace.impact(request("schema-r1")).unwrap_err();
    assert_eq!(failure.code, "AIL.IMPACT.INCOMPLETE_COVERAGE");
}
