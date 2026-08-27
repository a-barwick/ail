//! `ail-run` executes the frozen bytes of a published revision.
//!
//! The target program is the already-published `compiler/examples/job-review`.
//! These checks prove three things: the runner uses the frozen bytes, an
//! unpublished live edit does not change what runs, and `ailc check` on the same
//! folder still does not run the program.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use ail_compiler::{CapabilityEnvironment, ExecutionResponse, RunRefusal, load_published_program};

const PUBLISHED_JOB_ID: &str = "fixture-job";
const LIVE_JOB_ID: &str = "live-edit-job";
const PUBLISHED_SOURCE_SET_DIGEST: &str =
    "sha256:d04ad0c8928eab29b6d8e5e069d86ea702ebe928031100ab5e500ab3b92cfb88";
const ENTRY: &str = "scenarios.review_fixture";
const PAYLOAD_HEX: &str = "6a6f622d7061796c6f6164";

fn job_review() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/job-review")
}

fn ail_run(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ail-run"))
        .arg(directory)
        .args(arguments)
        .output()
        .expect("ail-run runs")
}

fn ailc(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ailc"))
        .args(arguments)
        .output()
        .expect("ailc runs")
}

fn temp_directory(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("ail-run-{label}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&path).expect("temporary directory is writable");
    path
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("destination directory is writable");
    for entry in fs::read_dir(from).expect("source directory is readable") {
        let entry = entry.expect("directory entry is readable");
        let source = entry.path();
        let destination = to.join(entry.file_name());
        if source.is_dir() {
            copy_tree(&source, &destination);
        } else {
            fs::copy(&source, &destination).expect("file is copyable");
        }
    }
}

/// A copy of the published example, live source files and frozen store alike.
fn published_copy(label: &str) -> PathBuf {
    let path = temp_directory(label);
    copy_tree(&job_review(), &path);
    path
}

fn live_sources_only(label: &str) -> PathBuf {
    let path = temp_directory(label);
    copy_tree(&job_review(), &path);
    fs::remove_dir_all(path.join(".ail")).expect("frozen store is removable");
    path
}

fn frozen_source(path: &Path, name: &str) -> PathBuf {
    path.join(".ail/revisions/published/sources").join(name)
}

fn rewrite(path: &Path, from: &str, to: &str) {
    let text = fs::read_to_string(path).expect("source is readable");
    assert!(
        text.contains(from),
        "{} must contain {from}",
        path.display()
    );
    fs::write(path, text.replace(from, to)).expect("source is writable");
}

#[test]
fn the_published_job_review_runs_from_the_frozen_bytes() {
    let output = ail_run(&job_review(), &[ENTRY, "--bytes", PAYLOAD_HEX]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stdout.contains("completed\n"), "{stdout}");
    assert!(stdout.contains("revision_id=published\n"), "{stdout}");
    assert!(
        stdout.contains(&format!(
            "source_set_digest={PUBLISHED_SOURCE_SET_DIGEST}\n"
        )),
        "{stdout}"
    );
    assert!(
        stdout.contains(&format!("job_id: \"{PUBLISHED_JOB_ID}\"")),
        "{stdout}"
    );
    assert!(
        stdout.contains("contracts.ReviewDecision::Approved"),
        "{stdout}"
    );
    assert!(stdout.contains("calls=0\n"), "{stdout}");
}

#[test]
fn a_store_without_any_live_source_file_still_runs() {
    // Nothing but the frozen store is present, so a completed run can only have
    // come from the bytes under `.ail/revisions/published/sources`.
    let path = temp_directory("store-only");
    copy_tree(&job_review().join(".ail"), &path.join(".ail"));
    assert!(
        fs::read_dir(&path)
            .expect("temporary directory is readable")
            .filter_map(Result::ok)
            .all(|entry| entry.file_name() == ".ail"),
        "the runner must not need a live source file"
    );

    let output = ail_run(&path, &[ENTRY, "--bytes", PAYLOAD_HEX]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains(&format!("job_id: \"{PUBLISHED_JOB_ID}\"")),
        "{stdout}"
    );

    fs::remove_dir_all(path).expect("temporary directory is removable");
}

#[test]
fn an_unpublished_live_edit_does_not_change_what_runs() {
    let path = published_copy("live-edit");
    rewrite(&path.join("scenarios.ail"), PUBLISHED_JOB_ID, LIVE_JOB_ID);
    assert!(
        fs::read_to_string(path.join("scenarios.ail"))
            .expect("live source is readable")
            .contains(LIVE_JOB_ID)
    );
    assert!(
        fs::read_to_string(frozen_source(&path, "scenarios.ail"))
            .expect("frozen source is readable")
            .contains(PUBLISHED_JOB_ID)
    );

    let output = ail_run(&path, &[ENTRY, "--bytes", PAYLOAD_HEX]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains(&format!("job_id: \"{PUBLISHED_JOB_ID}\"")),
        "{stdout}"
    );
    assert!(!stdout.contains(LIVE_JOB_ID), "{stdout}");
    assert!(
        stdout.contains(&format!(
            "source_set_digest={PUBLISHED_SOURCE_SET_DIGEST}\n"
        )),
        "{stdout}"
    );

    fs::remove_dir_all(path).expect("temporary directory is removable");
}

#[test]
fn ailc_check_does_not_run_the_program_after_a_live_edit() {
    let path = published_copy("check-does-not-run");
    rewrite(&path.join("scenarios.ail"), PUBLISHED_JOB_ID, LIVE_JOB_ID);

    let check = ailc(&["check", path.to_str().expect("path is UTF-8")]);
    let stdout = String::from_utf8_lossy(&check.stdout);
    let stderr = String::from_utf8_lossy(&check.stderr);
    assert!(check.status.success(), "{stderr}");
    assert_eq!(stdout, "ok\nbehavior: not-run 0/6\n");
    // The live edit changes only the returned job id. Neither the live nor the
    // published id appears, because check evaluates no program value.
    assert!(!stdout.contains(LIVE_JOB_ID), "{stdout}");
    assert!(!stdout.contains(PUBLISHED_JOB_ID), "{stdout}");
    assert!(!stdout.contains("Approved"), "{stdout}");

    let run = ailc(&["run", path.to_str().expect("path is UTF-8")]);
    assert!(!run.status.success());
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("unknown command"),
        "ailc must expose no command that executes a program"
    );

    fs::remove_dir_all(path).expect("temporary directory is removable");
}

#[test]
fn a_directory_with_no_published_revision_is_refused() {
    let path = live_sources_only("unpublished");

    let output = ail_run(&path, &[ENTRY, "--bytes", PAYLOAD_HEX]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stdout}");
    assert!(stderr.contains("refused\n"), "{stderr}");
    assert!(stderr.contains("AIL.RUN.NO_PUBLISHED_REVISION"), "{stderr}");
    assert!(
        stderr.contains("reason=revision store directory is absent"),
        "{stderr}"
    );
    assert!(!stdout.contains("completed"), "{stdout}");
    assert!(!stdout.contains(PUBLISHED_JOB_ID), "{stdout}");

    fs::remove_dir_all(path).expect("temporary directory is removable");
}

#[test]
fn a_current_pointer_without_stored_sources_is_refused() {
    let path = published_copy("dangling-pointer");
    fs::write(path.join(".ail/current"), "candidate\n").expect("pointer is writable");

    let output = ail_run(&path, &[ENTRY, "--bytes", PAYLOAD_HEX]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("AIL.RUN.NO_PUBLISHED_REVISION"), "{stderr}");
    assert!(
        stderr.contains("reason=revision candidate has no stored sources"),
        "{stderr}"
    );

    fs::remove_dir_all(path).expect("temporary directory is removable");
}

#[test]
fn edited_frozen_bytes_are_refused() {
    let path = published_copy("tampered");
    rewrite(
        &frozen_source(&path, "scenarios.ail"),
        PUBLISHED_JOB_ID,
        "tampered-job",
    );

    let output = ail_run(&path, &[ENTRY, "--bytes", PAYLOAD_HEX]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stdout}");
    assert!(stderr.contains("AIL.RUN.FROZEN_SOURCE_DIGEST"), "{stderr}");
    assert!(stderr.contains("path=scenarios.ail"), "{stderr}");
    assert!(!stdout.contains("tampered-job"), "{stdout}");

    fs::remove_dir_all(path).expect("temporary directory is removable");
}

#[test]
fn a_frozen_source_the_revision_does_not_list_is_refused() {
    let path = published_copy("unlisted-source");
    fs::write(
        frozen_source(&path, "extra.ail"),
        "module extra;\n\nfn keep(value: Text) -> Text {\n  value\n}\n",
    )
    .expect("frozen source is writable");

    let output = ail_run(&path, &[ENTRY, "--bytes", PAYLOAD_HEX]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("AIL.RUN.UNREADABLE_STORE"), "{stderr}");
    assert!(
        stderr.contains("extra.ail: frozen source is not listed in revision.json"),
        "{stderr}"
    );

    fs::remove_dir_all(path).expect("temporary directory is removable");
}

#[test]
fn a_revision_checked_under_another_capability_environment_is_refused() {
    let path = published_copy("other-capabilities");
    let document = path.join(".ail/revisions/published/revision.json");
    let text = fs::read_to_string(&document).expect("revision document is readable");
    let recorded = CapabilityEnvironment::new().stable_digest();
    assert!(text.contains(&recorded), "{text}");
    fs::write(
        &document,
        text.replace(
            &recorded,
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        ),
    )
    .expect("revision document is writable");

    let refusal = load_published_program(&path).expect_err("the runner must refuse");
    assert!(matches!(
        refusal,
        RunRefusal::CapabilityEnvironmentDigest { ref actual, .. } if *actual == recorded
    ));
    assert!(
        refusal
            .render()
            .contains("AIL.RUN.CAPABILITY_ENVIRONMENT_DIGEST"),
        "{}",
        refusal.render()
    );

    fs::remove_dir_all(path).expect("temporary directory is removable");
}

#[test]
fn the_loaded_program_reports_the_frozen_source_set_it_will_run() {
    let program = load_published_program(job_review()).expect("job-review is published");
    assert_eq!(program.revision_id(), "published");
    assert_eq!(program.source_set_digest(), PUBLISHED_SOURCE_SET_DIGEST);
    assert_eq!(
        program.capability_environment_digest(),
        CapabilityEnvironment::new().stable_digest()
    );

    let frozen = program.frozen_sources();
    let paths = frozen
        .iter()
        .map(|source| source.path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec![
            "contracts.ail",
            "domain.ail",
            "persistence.ail",
            "scenarios.ail",
            "transport.ail",
            "validation.ail",
        ]
    );
    for source in frozen {
        let stored = fs::read_to_string(frozen_source(&job_review(), &source.path))
            .expect("frozen source is readable");
        assert_eq!(source.source, stored, "{}", source.path);
    }
}

#[test]
fn the_runner_supplies_no_capabilities() {
    let program = load_published_program(job_review()).expect("job-review is published");
    let mut capabilities = ail_compiler::NoCapabilities;
    let response = program.run(
        "contracts.preserve_request",
        vec![ail_compiler::RuntimeValue::record(
            "contracts.ReviewRequest",
            [
                (
                    "job_id",
                    ail_compiler::RuntimeValue::Text(PUBLISHED_JOB_ID.to_owned()),
                ),
                (
                    "task",
                    ail_compiler::RuntimeValue::Text("compile".to_owned()),
                ),
                ("payload", ail_compiler::RuntimeValue::Bytes(vec![0x00])),
                (
                    "priority",
                    ail_compiler::RuntimeValue::variant("contracts.PriorityOption", "None", None),
                ),
                (
                    "requested_by",
                    ail_compiler::RuntimeValue::Text("queue-agent".to_owned()),
                ),
                (
                    "reviewer",
                    ail_compiler::RuntimeValue::Text("release-bot".to_owned()),
                ),
            ],
        )],
        &mut capabilities,
    );
    assert!(matches!(response, ExecutionResponse::Completed(_)));
    assert!(!ail_compiler::CapabilityProvider::supports(
        &capabilities,
        "store",
        "Store"
    ));
}
