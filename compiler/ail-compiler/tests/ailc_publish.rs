use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const CONCENTRATED_TRANSPORT: &str = r#"module transport;
import contracts;
import validation;

fn dispatch(request: contracts.ReviewRequest) -> contracts.ReviewDecision {
  match validation.validate_job_id(request) {
    contracts.ValidationResult::Invalid(reason) => {
      contracts.ReviewDecision::Rejected(reason)
    },
    contracts.ValidationResult::Valid => {
      match validation.validate_task(request) {
        contracts.ValidationResult::Invalid(reason) => {
          contracts.ReviewDecision::Rejected(reason)
        },
        contracts.ValidationResult::Valid => {
          match validation.validate_payload(request) {
            contracts.ValidationResult::Invalid(reason) => {
              contracts.ReviewDecision::Rejected(reason)
            },
            contracts.ValidationResult::Valid => {
              match validation.validate_priority(request) {
                contracts.ValidationResult::Invalid(reason) => {
                  contracts.ReviewDecision::Rejected(reason)
                },
                contracts.ValidationResult::Valid => {
                  match validation.validate_reviewer(request) {
                    contracts.ValidationResult::Invalid(reason) => {
                      contracts.ReviewDecision::Rejected(reason)
                    },
                    contracts.ValidationResult::Valid => {
                      let job = contracts.ApprovedJob {
                        job_id: request.job_id,
                        task: request.task,
                        payload: request.payload,
                        priority: validation.selected_priority(request.priority),
                        reviewer: request.reviewer,
                      };
                      contracts.ReviewDecision::Approved(job)
                    },
                  }
                },
              }
            },
          }
        },
      }
    },
  }
}
"#;

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples")
}

fn ailc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ailc"))
}

fn temp_workspace(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ail-publish-{label}-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temporary workspace is writable");
    path
}

fn copy_dir(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).expect("example directory is readable") {
        let entry = entry.expect("directory entry is readable");
        let source = entry.path();
        if source.is_file() {
            fs::copy(&source, to.join(entry.file_name())).expect("example file is copyable");
        }
    }
}

fn write_file(dir: &Path, name: &str, source: &str) {
    fs::write(dir.join(name), source).expect("temporary source is writable");
}

fn passing_architecture_policy() -> &'static str {
    r#"{
  "semantic_model_version": "source-architecture-v1",
  "analysis_scope": "transport:dispatch",
  "module_groups": {
    "adapters": "persistence-adapter",
    "contracts": "contract",
    "domain": "domain",
    "tests": "verification",
    "transport": "transport"
  },
  "capability_namespaces": {},
  "endpoint_groups": {},
  "operations": {},
  "policy": {
    "revision": "policy-r1",
    "allowed_group_dependencies": {
      "contract": [],
      "transport": ["contract"],
      "domain": ["contract"],
      "persistence-adapter": [],
      "verification": ["contract", "domain", "transport"]
    },
    "transport_capabilities": [],
    "transport_state": [],
    "dispatch_no_growth": {
      "control_flow_complexity": 4,
      "minimal_context_node_count": 12
    },
    "new_unit": {
      "control_flow_complexity_max": 6,
      "minimal_context_node_count_max": 12
    },
    "new_cycles": false,
    "coverage_required": true,
    "baseline_match": {
      "baseline_revision": "baseline-r1",
      "scope": "transport:dispatch",
      "metrics": {
        "control_flow_complexity": 4,
        "minimal_context_node_count": 12
      },
      "accepted_debt": true
    }
  }
}
"#
}

fn write_passing_architecture_workspace(dir: &Path) {
    write_file(
        dir,
        "contracts.ail",
        "module contracts;\n\nfn keep(value: Text) -> Text {\n  value\n}\n",
    );
    write_file(
        dir,
        "domain.ail",
        "module domain;\n\nfn work(value: Text) -> Text {\n  value\n}\n",
    );
    write_file(
        dir,
        "transport.ail",
        "module transport;\nimport contracts;\n\nfn dispatch(value: Text) -> Text {\n  contracts.keep(value)\n}\n",
    );
    write_file(
        dir,
        "adapters.ail",
        "module adapters;\n\nfn marker(value: Text) -> Text {\n  value\n}\n",
    );
    write_file(
        dir,
        "tests.ail",
        "module tests;\n\nfn marker(value: Text) -> Text {\n  value\n}\n",
    );
    write_file(dir, "architecture.json", passing_architecture_policy());
}

fn run_publish(path: &Path) -> std::process::Output {
    ailc()
        .arg("publish")
        .arg(path)
        .output()
        .expect("ailc publish runs")
}

fn run_check(path: &Path) -> std::process::Output {
    ailc()
        .arg("check")
        .arg(path)
        .output()
        .expect("ailc check runs")
}

fn source_bytes(path: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(path)
        .expect("workspace is readable")
        .map(|entry| entry.expect("workspace entry is readable"))
        .filter(|entry| entry.path().extension().is_some_and(|value| value == "ail"))
        .map(|entry| {
            let name = entry
                .file_name()
                .into_string()
                .expect("source name is UTF-8");
            let bytes = fs::read(entry.path()).expect("source is readable");
            (name, bytes)
        })
        .collect()
}

fn revision_store(path: &Path) -> PathBuf {
    path.join(".ail")
}

fn published_revision(path: &Path) -> PathBuf {
    revision_store(path).join("revisions/published/revision.json")
}

#[test]
fn publish_of_a_failing_type_candidate_writes_no_revision() {
    let path = temp_workspace("type-fail");
    write_file(
        &path,
        "broken.ail",
        "record Job {\n  job_id: Text;\n}\n\nfn make_job() -> Job {\n  let job = Job { job_id: 1 };\n  job\n}\n",
    );

    let output = run_publish(&path);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("AIL.TYPE.FIELD_MISMATCH"), "{stderr}");
    assert!(!revision_store(&path).exists(), "{stderr}");

    fs::remove_dir_all(path).expect("temporary workspace is removable");
}

#[test]
fn publish_of_a_failing_architecture_candidate_writes_no_revision() {
    let path = temp_workspace("arch-fail");
    copy_dir(&examples_dir().join("architecture-denied"), &path);

    let output = run_publish(&path);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("AIL.ARCH.BOUNDARY"), "{stderr}");
    assert!(!revision_store(&path).exists(), "{stderr}");

    fs::remove_dir_all(path).expect("temporary workspace is removable");
}

#[test]
fn publish_of_a_passing_candidate_writes_a_revision() {
    let path = temp_workspace("pass");
    copy_dir(&examples_dir().join("composed-service"), &path);

    let output = run_publish(&path);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stdout.contains("published\n"), "{stdout}");
    assert!(stdout.contains("revision_id=published"), "{stdout}");
    assert!(
        published_revision(&path).is_file(),
        "passing publish must write {path:?}"
    );
    let document = fs::read_to_string(published_revision(&path)).expect("revision is readable");
    assert!(
        document.contains("\"revision_id\":\"published\""),
        "{document}"
    );
    assert!(document.contains("source_set_digest"), "{document}");
    assert!(
        path.join(".ail/revisions/published/sources/service.ail")
            .is_file()
    );

    fs::remove_dir_all(path).expect("temporary workspace is removable");
}

#[test]
fn published_sources_do_not_follow_later_live_file_edits() {
    let path = temp_workspace("frozen-source");
    let checked_source = "module service;\n\nfn identity(value: Text) -> Text {\n  value\n}\n";
    write_file(&path, "service.ail", checked_source);

    let output = run_publish(&path);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    write_file(
        &path,
        "service.ail",
        "module service;\n\nfn changed(value: Text) -> Text {\n  value\n}\n",
    );
    let frozen_source = fs::read(path.join(".ail/revisions/published/sources/service.ail"))
        .expect("published source is readable");
    assert_eq!(frozen_source, checked_source.as_bytes());

    fs::remove_dir_all(path).expect("temporary workspace is removable");
}

#[test]
fn publish_of_a_passing_architecture_candidate_writes_a_revision() {
    let path = temp_workspace("arch-pass");
    write_passing_architecture_workspace(&path);

    let check = ailc()
        .arg("check")
        .arg(&path)
        .output()
        .expect("ailc check runs");
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    assert_eq!(check.stdout, b"ok\n");
    assert!(!revision_store(&path).exists());

    let output = run_publish(&path);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(published_revision(&path).is_file());
    let document = fs::read_to_string(published_revision(&path)).expect("revision is readable");
    assert!(
        document.contains("architecture_settings_digest"),
        "{document}"
    );

    fs::remove_dir_all(path).expect("temporary workspace is removable");
}

#[test]
fn publish_does_not_replace_an_existing_revision_when_the_candidate_fails() {
    let path = temp_workspace("keep-existing");
    copy_dir(&examples_dir().join("composed-service"), &path);
    let first = run_publish(&path);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let before = fs::read_to_string(published_revision(&path)).expect("first revision is readable");

    write_file(
        &path,
        "service.ail",
        "module service;\nimport domain as model;\n\nfn handle(request: model.Request) -> model.Response {\n  missing(request)\n}\n",
    );
    let second = run_publish(&path);
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(!second.status.success(), "{stderr}");
    assert!(stderr.contains("AIL.NAME.UNKNOWN_FUNCTION"), "{stderr}");
    let after = fs::read_to_string(published_revision(&path)).expect("kept revision is readable");
    assert_eq!(before, after);

    fs::remove_dir_all(path).expect("temporary workspace is removable");
}

#[test]
fn job_review_refuses_concentration_then_freezes_the_checked_sources() {
    let example = examples_dir().join("job-review");
    let path = temp_workspace("job-review-audit");
    copy_dir(&example, &path);
    write_file(&path, "transport.ail", CONCENTRATED_TRANSPORT);

    let architecture =
        fs::read(path.join("architecture.json")).expect("architecture policy is readable");
    fs::remove_file(path.join("architecture.json")).expect("policy can be withheld for type check");
    let language_check = run_check(&path);
    assert!(
        language_check.status.success(),
        "{}",
        String::from_utf8_lossy(&language_check.stderr)
    );
    assert_eq!(language_check.stdout, b"ok\n");
    fs::write(path.join("architecture.json"), architecture)
        .expect("architecture policy can be restored");

    let denied_check = run_check(&path);
    let denied_stderr = String::from_utf8_lossy(&denied_check.stderr);
    assert!(!denied_check.status.success(), "{denied_stderr}");
    assert!(
        denied_stderr.contains("AIL.ARCH.HOTSPOT_GROWTH"),
        "{denied_stderr}"
    );
    assert!(denied_stderr.contains("candidate_cfc=6"), "{denied_stderr}");
    assert!(
        denied_stderr.contains("candidate_context="),
        "{denied_stderr}"
    );
    assert!(!revision_store(&path).exists(), "{denied_stderr}");

    let denied_publish = run_publish(&path);
    assert!(
        !denied_publish.status.success(),
        "{}",
        String::from_utf8_lossy(&denied_publish.stderr)
    );
    assert!(!revision_store(&path).exists());

    fs::copy(example.join("transport.ail"), path.join("transport.ail"))
        .expect("passing transport is copyable");
    let passing_check = run_check(&path);
    assert!(
        passing_check.status.success(),
        "{}",
        String::from_utf8_lossy(&passing_check.stderr)
    );
    assert_eq!(passing_check.stdout, b"ok\n");
    assert!(!revision_store(&path).exists());
    let checked_sources = source_bytes(&path);

    let published = run_publish(&path);
    assert!(
        published.status.success(),
        "{}",
        String::from_utf8_lossy(&published.stderr)
    );
    for (name, checked_bytes) in checked_sources {
        let frozen = fs::read(path.join(".ail/revisions/published/sources").join(&name))
            .expect("published source is readable");
        assert_eq!(
            frozen, checked_bytes,
            "{name} changed between check and publish"
        );
    }

    fs::remove_dir_all(path).expect("temporary workspace is removable");
}
