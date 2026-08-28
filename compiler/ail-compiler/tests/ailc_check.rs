use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples")
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root exists")
}

fn ailc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ailc"))
}

fn write_temp_source(label: &str, source: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ail-check-{label}-{}-{unique}.ail",
        std::process::id()
    ));
    fs::write(&path, source).expect("temporary source is writable");
    path
}

fn run_check(path: &Path) -> std::process::Output {
    ailc()
        .arg("check")
        .arg(path)
        .output()
        .expect("ailc check runs")
}

fn assert_reports_source(stderr: &str, source_name: &str) {
    assert!(
        stderr.contains(&format!("at {source_name}:")),
        "diagnostic must name {source_name}: {stderr}"
    );
    assert!(
        !stderr.contains("<source-set>"),
        "diagnostic must not use the generic source-set path: {stderr}"
    );
}

fn revision_store(path: &Path) -> PathBuf {
    path.join(".ail")
}

#[test]
fn check_accepts_the_composed_service_workspace() {
    let path = examples_dir().join("composed-service");
    let output = run_check(&path);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"ok\n");
    assert!(output.stderr.is_empty());
    assert!(
        !revision_store(&path).exists(),
        "ailc check must remain read-only"
    );
}

#[test]
fn check_accepts_one_file_as_the_whole_program() {
    let path = write_temp_source(
        "one-file",
        "fn identity(value: Text) -> Text {\n  value\n}\n",
    );
    let output = run_check(&path);
    fs::remove_file(path).expect("temporary source is removable");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"ok\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn check_rejects_the_repository_root_instead_of_collecting_examples() {
    let path = repository_root();
    let output = run_check(&path);

    assert!(!output.status.success());
    assert_ne!(output.stdout, b"ok\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("{}: no valid .ail source files", path.display())),
        "{stderr}"
    );
    assert!(!stderr.contains("source set is empty"), "{stderr}");
}

#[test]
fn check_rejects_a_type_correct_architecture_policy_violation() {
    let path = examples_dir().join("architecture-denied");
    let output = run_check(&path);
    assert!(!output.status.success());
    assert_ne!(output.stdout, b"ok\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("AIL.ARCH.BOUNDARY"), "{stderr}");
    assert!(
        stderr.contains("source contains architecture diagnostics"),
        "{stderr}"
    );
    assert!(
        !revision_store(&path).exists(),
        "ailc check must not write a revision"
    );
}

#[test]
fn job_review_keeps_a_live_refusal_separate_from_the_publishable_program() {
    let refused_path = examples_dir().join("job-review-refused");
    let refused = run_check(&refused_path);
    assert!(!refused.status.success());
    assert!(refused.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&refused.stderr);
    for fact in [
        "AIL.ARCH.HOTSPOT_GROWTH architecture error",
        "facts.base_cfc=1",
        "facts.base_context=6",
        "facts.candidate_cfc=6",
        "facts.candidate_context=9",
        "rule=M23-POL-DISPATCH-NO-GROWTH",
        "scope=transport:dispatch",
    ] {
        assert!(stderr.contains(fact), "missing {fact}: {stderr}");
    }
    assert!(
        !revision_store(&refused_path).exists(),
        "the refusing candidate must not contain a published revision"
    );

    let published_path = examples_dir().join("job-review");
    let published = run_check(&published_path);
    assert!(
        published.status.success(),
        "{}",
        String::from_utf8_lossy(&published.stderr)
    );
    let stdout = String::from_utf8_lossy(&published.stdout);
    assert_eq!(stdout, "ok\nbehavior: not-run 0/6\n");
    assert!(!stdout.contains("passed"), "{stdout}");
    assert!(published.stderr.is_empty());
}

#[test]
fn check_reports_the_architecture_facts_the_checker_already_computed() {
    let path = examples_dir().join("architecture-denied");
    let output = run_check(&path);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // ADR 0016 replaced the `code:scope:rule:` line with a structured finding.
    // The rule and scope are still reported, now as their own facts.
    assert!(
        stderr.contains("AIL.ARCH.BOUNDARY architecture error"),
        "{stderr}"
    );
    assert!(stderr.contains("rule=M23-POL-GROUP-DEPENDENCY"), "{stderr}");
    assert!(stderr.contains("scope=group:transport"), "{stderr}");
    assert!(
        stderr.contains("forbidden_group_edges.0.source_group=transport"),
        "{stderr}"
    );
    assert!(
        stderr.contains("forbidden_group_edges.0.target_group=domain"),
        "{stderr}"
    );
    assert!(
        stderr.contains("forbidden_group_edges.0.source=transport:dispatch"),
        "{stderr}"
    );
    assert!(
        stderr.contains("forbidden_group_edges.0.target=domain:work"),
        "{stderr}"
    );
}

#[test]
fn check_rejects_an_imported_file_as_a_one_file_workspace() {
    let path = examples_dir().join("composed-service/service.ail");
    let output = run_check(&path);
    assert!(!output.status.success());
    assert_ne!(output.stdout, b"ok\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("AIL.MODULE.MISSING_IMPORT"), "{stderr}");
    assert!(
        stderr.contains("source contains check diagnostics"),
        "{stderr}"
    );
}

#[test]
fn check_rejects_a_type_error_with_the_workspace_diagnostic() {
    let path = write_temp_source(
        "type",
        "record Job {\n  job_id: Text;\n}\n\nfn make_job() -> Job {\n  let job = Job { job_id: 1 };\n  job\n}\n",
    );
    let output = run_check(&path);
    let name = path.file_name().and_then(|name| name.to_str()).unwrap();
    fs::remove_file(&path).expect("temporary source is removable");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("AIL.TYPE.FIELD_MISMATCH"), "{stderr}");
    assert!(stderr.contains(&format!("at {name}:6:27-6:28")), "{stderr}");
    assert!(stderr.contains("expected.type=Text"), "{stderr}");
    assert!(stderr.contains("actual.type=Int"), "{stderr}");
}

#[test]
fn check_rejects_a_name_error_with_the_workspace_diagnostic() {
    let path = write_temp_source("name", "fn run(value: Text) -> Text { missing(value) }\n");
    let output = run_check(&path);
    let name = path.file_name().and_then(|name| name.to_str()).unwrap();
    fs::remove_file(&path).expect("temporary source is removable");

    assert!(!output.status.success());
    assert_ne!(output.stdout, b"ok\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("AIL.NAME.UNKNOWN_FUNCTION"), "{stderr}");
    assert_reports_source(&stderr, name);
}

#[test]
fn check_rejects_an_effect_error_with_the_workspace_diagnostic() {
    let path = write_temp_source(
        "effect",
        "fn run(value: Text) -> Text effects { store.mark } {\n  value\n}\n",
    );
    let output = run_check(&path);
    let name = path.file_name().and_then(|name| name.to_str()).unwrap();
    fs::remove_file(&path).expect("temporary source is removable");

    assert!(!output.status.success());
    assert_ne!(output.stdout, b"ok\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("AIL.CAPABILITY.INVALID_EFFECT"), "{stderr}");
    assert_reports_source(&stderr, name);
}

#[test]
fn check_rejects_recursive_calls() {
    let path = write_temp_source(
        "recursion",
        "fn first(value: Text) -> Text { second(value) }\n\nfn second(value: Text) -> Text { first(value) }\n",
    );
    let output = run_check(&path);
    fs::remove_file(path).expect("temporary source is removable");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("AIL.CALL.RECURSIVE_CYCLE"), "{stderr}");
}

#[test]
fn check_rejects_unknown_capability_interfaces() {
    let path = examples_dir().join("batch-lookup");
    let output = run_check(&path);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("AIL.CAPABILITY.UNKNOWN_INTERFACE"),
        "{stderr}"
    );
}

#[test]
fn check_accepts_a_source_set_with_declared_capabilities() {
    let path = examples_dir().join("capability-declared");
    let output = run_check(&path);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"ok\n");
    assert!(output.stderr.is_empty());
    assert!(
        !revision_store(&path).exists(),
        "ailc check must remain read-only"
    );

    let json = ailc()
        .args(["check", "--json"])
        .arg(&path)
        .output()
        .expect("ailc check --json runs");
    assert!(
        json.status.success(),
        "{}",
        String::from_utf8_lossy(&json.stderr)
    );
    let document: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("check --json is JSON");
    assert_eq!(document["status"], "ok");
    assert_eq!(
        document["capability_environment"]["path"],
        "capabilities.json"
    );
    let digest = document["capability_environment"]["digest"]
        .as_str()
        .expect("loaded digest is a string");
    assert!(digest.starts_with("sha256:"), "{digest}");
}

#[test]
fn check_does_not_search_outside_the_source_set_root_for_capabilities() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-check-cap-search-{}-{unique}",
        std::process::id()
    ));
    let source_set = root.join("source-set");
    fs::create_dir_all(source_set.join(".ail")).expect("decoy directory is writable");
    for name in ["types.ail", "service.ail"] {
        fs::copy(
            examples_dir().join("batch-lookup").join(name),
            source_set.join(name),
        )
        .expect("batch-lookup source is copyable");
    }
    let declared = fs::read_to_string(examples_dir().join("capability-declared/capabilities.json"))
        .expect("declared capabilities are readable");
    fs::write(root.join("capabilities.json"), &declared).expect("parent decoy is writable");
    fs::write(source_set.join(".ail/capabilities.json"), &declared)
        .expect(".ail decoy is writable");
    fs::write(source_set.join("capability.json"), &declared).expect("wrong-name decoy is writable");

    let output = run_check(&source_set);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains("AIL.CAPABILITY.UNKNOWN_INTERFACE"),
        "{stderr}"
    );
    assert!(stderr.contains("this check supplies none"), "{stderr}");
    assert!(
        !stderr.contains("capabilities.json"),
        "absent source-set file must not mention a loaded path: {stderr}"
    );

    fs::remove_dir_all(root).expect("temporary workspace is removable");
}

#[test]
fn check_rejects_malformed_capabilities_json() {
    let path = examples_dir().join("capability-malformed");
    let output = run_check(&path);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert_ne!(output.stdout, b"ok\n");
    assert!(stderr.contains("capabilities.json"), "{stderr}");
    assert!(
        stderr.contains("expected") || stderr.contains("EOF") || stderr.contains("invalid"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("AIL.CAPABILITY.UNKNOWN_INTERFACE"),
        "malformed file must fail as a load error: {stderr}"
    );
}

#[test]
fn check_rejects_duplicate_capability_names() {
    let path = examples_dir().join("capability-duplicate");
    let output = run_check(&path);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("capabilities.json"), "{stderr}");
    assert!(
        stderr.contains("duplicate capability name JobsStore"),
        "{stderr}"
    );
}

#[test]
fn check_rejects_undeclared_capability_authority() {
    let path = examples_dir().join("capability-undeclared");
    let output = run_check(&path);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains("AIL.CAPABILITY.UNKNOWN_INTERFACE"),
        "{stderr}"
    );
    assert!(stderr.contains("expected.capability=Clock"), "{stderr}");
    assert!(
        stderr.contains("capability_environment.path=capabilities.json"),
        "{stderr}"
    );
    assert!(
        stderr.contains("capability_environment.digest=sha256:"),
        "{stderr}"
    );
    assert!(
        stderr.contains("capability_environment.interfaces=JobsStore"),
        "{stderr}"
    );
    assert!(!stderr.contains("this check supplies none"), "{stderr}");
}

#[test]
fn check_reports_workspace_parse_causes() {
    let path = write_temp_source("parse", "record Job {\n  job_id\n}\n");
    let output = run_check(&path);
    fs::remove_file(&path).expect("temporary source is removable");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let name = path.file_name().and_then(|name| name.to_str()).unwrap();
    assert!(
        stderr.contains(&format!("{name} has parse diagnostics")),
        "{stderr}"
    );
}
