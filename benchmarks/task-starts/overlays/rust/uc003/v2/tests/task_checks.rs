use std::{fs, path::PathBuf};

use ail_job_service_v2::{
    codec::run_case_value,
    domain::{
        CreateJobRequest, CreateJobResult, InsertOutcome, Job, JobStore, Priority,
        ValidationField, ValidationIssue, ValidationReason,
    },
    service::create_job,
    store::{DeterministicJobStore, RecordVersion, StoredJob},
};
use serde_json::Value;

#[derive(Debug)]
struct RecordingStore {
    outcome: InsertOutcome,
    calls: Vec<Job>,
}

impl JobStore for RecordingStore {
    fn insert_if_absent(&mut self, job: &Job) -> InsertOutcome {
        self.calls.push(job.clone());
        self.outcome
    }
}

fn request(priority: Option<Priority>) -> CreateJobRequest {
    CreateJobRequest {
        job_id: "job-1042".into(),
        task: "rebuild-search-index".into(),
        payload: br#"{"tenant":"north"}"#.to_vec(),
        priority,
    }
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .expect("v2 crate is nested under benchmarks/baselines/rust")
        .to_path_buf()
}

#[test]
fn every_priority_propagates_unchanged() {
    for priority in [Priority::Low, Priority::Normal, Priority::High] {
        let mut store = RecordingStore {
            outcome: InsertOutcome::Inserted,
            calls: Vec::new(),
        };
        let result = create_job(request(Some(priority)), &mut store);
        let CreateJobResult::Created(job) = result else {
            panic!("expected created result");
        };
        assert_eq!(job.priority, priority);
        assert_eq!(store.calls, vec![job]);
    }
}

#[test]
fn missing_priority_follows_inherited_issues_without_store_access() {
    let mut store = RecordingStore {
        outcome: InsertOutcome::Inserted,
        calls: Vec::new(),
    };
    let result = create_job(
        CreateJobRequest {
            job_id: String::new(),
            task: String::new(),
            payload: vec![0; 4097],
            priority: None,
        },
        &mut store,
    );

    assert_eq!(
        result,
        CreateJobResult::Invalid(vec![
            ValidationIssue {
                field: ValidationField::JobId,
                reason: ValidationReason::Missing,
            },
            ValidationIssue {
                field: ValidationField::Task,
                reason: ValidationReason::Missing,
            },
            ValidationIssue {
                field: ValidationField::Payload,
                reason: ValidationReason::PayloadTooLarge,
            },
            ValidationIssue {
                field: ValidationField::Priority,
                reason: ValidationReason::Missing,
            },
        ])
    );
    assert!(store.calls.is_empty());
}

#[test]
fn version_one_stored_records_adapt_explicitly_to_normal() {
    let record = StoredJob::V1 {
        job_id: "legacy".into(),
        task: "task".into(),
        payload: vec![1],
    };
    assert_eq!(record.adapt_to_v2().priority, Priority::Normal);

    let mut store =
        DeterministicJobStore::new(Vec::new(), InsertOutcome::Inserted, RecordVersion::V2);
    let job = Job {
        job_id: "job-1".into(),
        task: "task".into(),
        payload: vec![1, 2, 3],
        priority: Priority::High,
    };
    assert_eq!(store.insert_if_absent(&job), InsertOutcome::Inserted);
    assert_eq!(store.calls().len(), 1);
    assert_eq!(store.jobs()[0].adapt_to_v2(), job);
}

#[test]
fn every_public_fixture_matches_the_shared_oracle() {
    let root = repository_root();
    let manifest: Value = serde_json::from_slice(
        &fs::read(root.join("benchmarks/fixtures/manifest.json"))
            .expect("read fixture manifest"),
    )
    .expect("parse fixture manifest");
    let entries = manifest["fixtures"]
        .as_array()
        .expect("fixture entries are an array");
    assert_eq!(entries.len(), 37);

    for entry in entries {
        let path = entry["path"].as_str().expect("fixture path");
        let fixture: Value =
            serde_json::from_slice(&fs::read(root.join(path)).expect("read fixture"))
                .expect("parse fixture");
        let expected = fixture["expected"].clone();
        let result = run_case_value(fixture).expect("run fixture");
        assert_eq!(result.actual, expected, "shared oracle mismatch for {path}");
    }
}
