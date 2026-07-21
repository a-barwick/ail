use std::fs;
use std::path::{Path, PathBuf};

use ail_compiler::{
    CapabilityEnvironment, CapabilityInterface, CapabilityOperation, HandleKind,
    IdentityClassification, InspectionRequest, RenameRequest, RenameResponse, Workspace,
    source_digest,
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
                .map(|parameter| parameter.as_str().expect("operation parameter is a string"));
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

fn workspace(fixture: &Value) -> Workspace {
    Workspace::new(
        fixture["input"]["revision"]["workspace_id"]
            .as_str()
            .expect("workspace ID is a string"),
        fixture["input"]["revision"]["revision_id"]
            .as_str()
            .expect("revision ID is a string"),
        fixture["input"]["path"].as_str().expect("path is a string"),
        fixture["input"]["source"]
            .as_str()
            .expect("source is a string"),
        environment(fixture),
    )
    .expect("fixture source creates a revision")
}

fn handle_for_kind(
    workspace: &Workspace,
    revision_id: &str,
    kind: &str,
) -> ail_compiler::SemanticHandle {
    workspace
        .handles(revision_id)
        .expect("revision handles exist")
        .into_iter()
        .find(|handle| {
            workspace
                .inspect(InspectionRequest {
                    revision_id: revision_id.to_owned(),
                    handle: handle.clone(),
                })
                .expect("current handle inspects")
                .semantic_kind
                == kind
        })
        .expect("requested handle kind exists")
}

#[test]
fn revision_storage_uses_canonical_source_and_the_required_sha256_digest() {
    let fixture = fixture("rename.json");
    let workspace = workspace(&fixture);
    let source = fixture["input"]["source"].as_str().unwrap();

    assert_eq!(workspace.source("rename-r1"), Some(source));
    assert_eq!(
        workspace.current_revision().source_digest,
        fixture["input"]["revision"]["source_digest"]
            .as_str()
            .unwrap()
    );
    assert_eq!(
        source_digest(source),
        "sha256:73455c0444e33980fc0ecc3ba3ba81ed0bd6220201a0f1477fa3251b631e67de"
    );
}

#[test]
fn inspection_exposes_elaborated_function_and_local_facts() {
    let fixture = fixture("positive.json");
    let workspace = workspace(&fixture);
    let revision_id = fixture["input"]["revision"]["revision_id"]
        .as_str()
        .unwrap();

    let function = handle_for_kind(&workspace, revision_id, "function");
    let function = workspace
        .inspect(InspectionRequest {
            revision_id: revision_id.to_owned(),
            handle: function,
        })
        .expect("function inspection succeeds");
    assert_eq!(
        function.explicit_type.as_deref(),
        Some("fn(Text, capability JobsStore) -> CreateJobResult effects { jobs.insert }")
    );
    assert_eq!(function.effects, ["jobs.insert"]);
    assert_eq!(function.capabilities, ["jobs:JobsStore"]);
    assert_eq!(
        function.dependencies,
        [
            "CreateJobResult",
            "Job",
            "JobsStore",
            "StoreOutcome",
            "Text"
        ]
    );

    let binding = handle_for_kind(&workspace, revision_id, "let-binding");
    let binding = workspace
        .inspect(InspectionRequest {
            revision_id: revision_id.to_owned(),
            handle: binding,
        })
        .expect("let binding inspection succeeds");
    assert_eq!(binding.explicit_type, None);
    assert_eq!(binding.inferred_type.as_deref(), Some("Job"));
    assert_eq!(binding.dependencies, ["Job", "Text"]);
}

#[test]
fn record_rename_commits_canonical_edits_and_a_complete_identity_map() {
    let fixture = fixture("rename.json");
    let mut workspace = workspace(&fixture);
    let record = handle_for_kind(&workspace, "rename-r1", "record");
    let response = workspace.rename(RenameRequest {
        base_revision_id: "rename-r1".to_owned(),
        handle: record.clone(),
        new_name: "StoredJob".to_owned(),
    });
    let RenameResponse::Committed(success) = response else {
        panic!("rename must commit");
    };

    assert_eq!(success.status, "committed");
    assert_eq!(success.revision.revision_id, "rename-r2");
    assert_eq!(
        success.revision.parent_revision_id.as_deref(),
        Some("rename-r1")
    );
    assert_eq!(
        success.revision.source_digest,
        fixture["expected"]["protocol_result"]["revision"]["source_digest"]
            .as_str()
            .unwrap()
    );
    assert_eq!(success.edits.len(), 4);
    assert!(
        success
            .edits
            .windows(2)
            .all(|pair| pair[0].span.start < pair[1].span.start)
    );
    assert!(success.edits.iter().all(|edit| edit.path == "main.ail"));
    assert!(
        success
            .edits
            .iter()
            .all(|edit| edit.replacement == "StoredJob")
    );
    let mut replayed = fixture["input"]["source"].as_str().unwrap().to_owned();
    for edit in success.edits.iter().rev() {
        replayed.replace_range(edit.span.start..edit.span.end, &edit.replacement);
    }
    assert_eq!(
        replayed,
        fixture["expected"]["canonical_source"].as_str().unwrap()
    );
    assert_eq!(
        workspace.source("rename-r1"),
        fixture["input"]["source"].as_str()
    );
    assert_eq!(
        workspace.source("rename-r2"),
        fixture["expected"]["canonical_source"].as_str()
    );
    assert_eq!(workspace.current_revision_id(), "rename-r2");
    assert!(
        success
            .identity_map
            .entries
            .iter()
            .any(|entry| entry.old_handle == record
                && entry.classification == IdentityClassification::Surviving)
    );
    assert!(
        success
            .identity_map
            .entries
            .iter()
            .any(|entry| entry.old_handle.kind == HandleKind::Syntax
                && entry.classification == IdentityClassification::Replaced)
    );
    assert!(success.identity_map.entries.iter().all(|entry| {
        !matches!(
            entry.classification,
            IdentityClassification::Removed | IdentityClassification::Unmapped
        )
    }));
    assert!(
        workspace
            .inspect(InspectionRequest {
                revision_id: "rename-r2".to_owned(),
                handle: record,
            })
            .is_err()
    );
}

#[test]
fn invalid_handle_name_and_collision_reject_without_publishing_a_revision() {
    let source = "record Job {\n  job_id: Text;\n}\n\nrecord StoredJob {\n  job_id: Text;\n}\n";
    let mut workspace = Workspace::new(
        "rename-rejections",
        "reject-r1",
        "main.ail",
        source,
        CapabilityEnvironment::new(),
    )
    .expect("source creates a revision");
    let record = handle_for_kind(&workspace, "reject-r1", "record");
    let syntax = workspace
        .handles("reject-r1")
        .unwrap()
        .into_iter()
        .find(|handle| handle.kind == HandleKind::Syntax)
        .unwrap();

    let invalid_handle = workspace.rename(RenameRequest {
        base_revision_id: "reject-r1".to_owned(),
        handle: syntax,
        new_name: "Renamed".to_owned(),
    });
    let RenameResponse::Rejected(invalid_handle) = invalid_handle else {
        panic!("syntax handle must reject");
    };
    assert_eq!(
        invalid_handle.diagnostic.code,
        "AIL.PROTOCOL.INVALID_HANDLE"
    );

    let invalid_name = workspace.rename(RenameRequest {
        base_revision_id: "reject-r1".to_owned(),
        handle: record.clone(),
        new_name: "fn".to_owned(),
    });
    let RenameResponse::Rejected(invalid_name) = invalid_name else {
        panic!("keyword must reject");
    };
    assert_eq!(invalid_name.diagnostic.code, "AIL.PROTOCOL.INVALID_NAME");

    let collision = workspace.rename(RenameRequest {
        base_revision_id: "reject-r1".to_owned(),
        handle: record,
        new_name: "StoredJob".to_owned(),
    });
    let RenameResponse::Rejected(collision) = collision else {
        panic!("top-level collision must reject");
    };
    assert_eq!(collision.diagnostic.code, "AIL.PROTOCOL.NAME_COLLISION");
    assert_eq!(workspace.current_revision_id(), "reject-r1");
    assert_eq!(workspace.source("reject-r2"), None);
}

#[test]
fn stale_base_revision_rejects_after_a_previous_rename_without_retrying() {
    let fixture = fixture("stale-revision.json");
    let mut workspace = workspace(&fixture);
    let record = handle_for_kind(&workspace, "stale-r1", "record");
    let first = workspace.rename(RenameRequest {
        base_revision_id: "stale-r1".to_owned(),
        handle: record.clone(),
        new_name: "StoredJob".to_owned(),
    });
    assert!(matches!(first, RenameResponse::Committed(_)));

    let stale = workspace.rename(RenameRequest {
        base_revision_id: "stale-r1".to_owned(),
        handle: record,
        new_name: "LaterJob".to_owned(),
    });
    let RenameResponse::Rejected(stale) = stale else {
        panic!("stale revision must reject");
    };
    assert_eq!(stale.diagnostic.code, "AIL.PROTOCOL.STALE_REVISION");
    assert_eq!(
        stale.diagnostic.expected.get("current_revision_id"),
        Some(&ail_compiler::DiagnosticValue::Text("stale-r2".to_owned()))
    );
    assert_eq!(stale.edits, []);
    assert_eq!(workspace.current_revision_id(), "stale-r2");
}

#[test]
fn repeated_protocol_requests_produce_the_same_handles_edits_and_identity_map() {
    let fixture = fixture("rename.json");
    let mut left = workspace(&fixture);
    let mut right = workspace(&fixture);
    let left_handle = handle_for_kind(&left, "rename-r1", "record");
    let right_handle = handle_for_kind(&right, "rename-r1", "record");

    assert_eq!(left.handles("rename-r1"), right.handles("rename-r1"));
    let left = left.rename(RenameRequest {
        base_revision_id: "rename-r1".to_owned(),
        handle: left_handle,
        new_name: "StoredJob".to_owned(),
    });
    let right = right.rename(RenameRequest {
        base_revision_id: "rename-r1".to_owned(),
        handle: right_handle,
        new_name: "StoredJob".to_owned(),
    });
    assert_eq!(left, right);
}
