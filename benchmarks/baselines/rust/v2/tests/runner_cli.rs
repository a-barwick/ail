use std::{path::PathBuf, process::Command};

use serde_json::Value;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .expect("v2 crate is nested under benchmarks/baselines/rust")
        .to_path_buf()
}

#[test]
fn binary_writes_exactly_one_json_value_for_one_case() {
    let output = Command::new(env!("CARGO_BIN_EXE_ail-job-service-v2"))
        .current_dir(repo_root())
        .args([
            "--case",
            "benchmarks/fixtures/public/create_job/uc003-v2-created-priority-high.json",
        ])
        .output()
        .expect("run Rust baseline binary");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).expect("single JSON stdout");
    assert_eq!(value["case_id"], "uc003-v2-created-priority-high");
    assert_eq!(
        value["actual"]["response"]["result"]["job"]["priority"],
        "high"
    );
}

#[test]
fn binary_rejects_an_invalid_command_without_stdout_data() {
    let output = Command::new(env!("CARGO_BIN_EXE_ail-job-service-v2"))
        .arg("--unknown")
        .output()
        .expect("run Rust baseline binary");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected exactly"));
}
