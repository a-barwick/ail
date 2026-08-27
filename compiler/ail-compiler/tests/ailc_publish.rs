use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

fn run_publish(path: &Path) -> std::process::Output {
    ailc()
        .arg("publish")
        .arg(path)
        .output()
        .expect("ailc publish runs")
}

fn revision_store(path: &Path) -> PathBuf {
    path.join(".ail")
}

fn published_revision(path: &Path) -> PathBuf {
    revision_store(path).join("revisions/published/revision.json")
}

#[test]
fn publish_of_a_failing_type_candidate_writes_no_revision() {
    let path = examples_dir().join("job-review-type-refused");

    let output = run_publish(&path);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("AIL.TYPE.FIELD_MISMATCH"), "{stderr}");
    assert!(!revision_store(&path).exists(), "{stderr}");
}

#[test]
fn publish_of_a_failing_architecture_candidate_writes_no_revision() {
    let path = examples_dir().join("architecture-denied");

    let output = run_publish(&path);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("AIL.ARCH.BOUNDARY"), "{stderr}");
    assert!(!revision_store(&path).exists(), "{stderr}");
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
    copy_dir(&examples_dir().join("composed-service"), &path);
    let checked_source =
        fs::read(path.join("service.ail")).expect("checked example source is readable");

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
    assert_eq!(frozen_source, checked_source);

    fs::remove_dir_all(path).expect("temporary workspace is removable");
}

#[test]
fn publish_of_a_passing_architecture_candidate_writes_a_revision() {
    let path = temp_workspace("arch-pass");
    copy_dir(&examples_dir().join("job-review"), &path);

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
    let check_stdout = String::from_utf8_lossy(&check.stdout);
    assert_eq!(check_stdout, "ok\nbehavior: not-run 0/6\n");
    assert!(!check_stdout.contains("passed"), "{check_stdout}");
    assert!(!revision_store(&path).exists());

    let output = run_publish(&path);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stdout.contains("behavior: not-run 0/6\n"), "{stdout}");
    assert!(!stdout.contains("passed"), "{stdout}");
    assert!(published_revision(&path).is_file());
    let document = fs::read_to_string(published_revision(&path)).expect("revision is readable");
    assert!(
        document.contains("architecture_settings_digest"),
        "{document}"
    );

    fs::remove_dir_all(path).expect("temporary workspace is removable");
}

#[test]
fn publish_of_declared_capabilities_records_the_environment_digest() {
    let path = temp_workspace("cap-pass");
    copy_dir(&examples_dir().join("capability-declared"), &path);

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
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stdout.contains("published\n"), "{stdout}");
    let document = fs::read_to_string(published_revision(&path)).expect("revision is readable");
    assert!(
        document.contains("capability_environment_digest"),
        "{document}"
    );
    assert!(
        !document.contains("\"capability_environment_digest\":\"sha256:4af4a6b7ff095044d5395564812c008d5e22607cc05eee393201afa984e7353b\""),
        "publish must record the loaded environment, not the empty one: {document}"
    );

    let json = ailc()
        .args(["publish", "--json"])
        .arg(&path)
        .output()
        .expect("ailc publish --json runs");
    assert!(
        json.status.success(),
        "{}",
        String::from_utf8_lossy(&json.stderr)
    );
    let published: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("publish --json is JSON");
    assert_eq!(published["status"], "published");
    assert_eq!(
        published["capability_environment"]["path"],
        "capabilities.json"
    );
    let digest = published["capability_environment"]["digest"]
        .as_str()
        .expect("loaded digest is a string");
    assert!(digest.starts_with("sha256:"), "{digest}");
    assert!(
        document.contains(&format!("\"capability_environment_digest\":\"{digest}\"")),
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
