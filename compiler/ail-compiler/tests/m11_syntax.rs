use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use ail_compiler::{format_source, parse, reconstruct};
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

#[test]
fn every_m11_source_is_lossless_and_byte_spans_partition_input() {
    for entry in fs::read_dir(fixtures_dir()).expect("fixture directory is readable") {
        let entry = entry.expect("directory entry is readable");
        if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let fixture: Value =
            serde_json::from_str(&fs::read_to_string(entry.path()).expect("fixture is readable"))
                .expect("fixture is valid JSON");
        let source = source(&fixture);
        let parsed = parse(source);
        assert_eq!(
            reconstruct(&parsed.tokens),
            source,
            "{} did not reconstruct",
            entry.path().display()
        );

        let mut cursor = 0;
        for token in parsed.tokens.iter().filter(|token| !token.text.is_empty()) {
            assert_eq!(token.span.start, cursor);
            assert_eq!(&source[token.span.start..token.span.end], token.text);
            cursor = token.span.end;
        }
        assert_eq!(cursor, source.len());
    }
}

#[test]
fn formatting_fixture_matches_canonical_source_and_is_idempotent() {
    let fixture = fixture("formatting.json");
    let expected = fixture["expected"]["canonical_source"]
        .as_str()
        .expect("canonical source is a string");
    let formatted = format_source(source(&fixture)).expect("formatting fixture parses");
    assert_eq!(formatted, expected);
    assert_eq!(
        format_source(&formatted).expect("canonical source parses"),
        expected
    );
}

#[test]
fn parseable_static_fixtures_match_their_canonical_source() {
    for name in ["positive.json", "capability-error.json", "type-error.json"] {
        let fixture = fixture(name);
        let expected = fixture["expected"]["canonical_source"]
            .as_str()
            .expect("canonical source is a string");
        assert_eq!(
            format_source(source(&fixture)).expect("fixture parses"),
            expected,
            "{name}"
        );
    }
}

#[test]
fn protocol_fixtures_have_canonical_input_before_protocol_operations() {
    for name in ["rename.json", "stale-revision.json"] {
        let fixture = fixture(name);
        assert_eq!(
            format_source(source(&fixture)).expect("fixture input parses"),
            source(&fixture),
            "{name}"
        );
    }
}

#[test]
fn recovery_fixture_emits_one_deterministic_missing_colon_diagnostic() {
    let fixture = fixture("recovery.json");
    let source = source(&fixture);
    let parsed = parse(source);
    assert_eq!(parsed.diagnostics.len(), 1);
    let diagnostic = &parsed.diagnostics[0];
    let expected = &fixture["expected"]["primary_diagnostic"];
    assert_eq!(
        diagnostic.code,
        expected["code"]
            .as_str()
            .expect("diagnostic code is a string")
    );
    assert_eq!(diagnostic.category, "parse");
    assert_eq!(
        diagnostic.expected,
        expected["expected"]["token"]
            .as_str()
            .expect("expected token is a string")
    );
    assert_eq!(
        diagnostic.actual,
        expected["actual"]["token"]
            .as_str()
            .expect("actual token is a string")
    );
    assert_eq!(diagnostic.span.start, diagnostic.span.end);
    assert_eq!(&source[..diagnostic.span.start], "record Job {\n  job_id");
    assert!(format_source(source).is_err());
}

#[test]
fn comments_and_noncanonical_trivia_remain_lossless() {
    let source = "/* lead */ record Job { // field\n job_id : Text ; }\n";
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty());
    assert_eq!(reconstruct(&parsed.tokens), source);
    assert_eq!(
        format_source(source).expect("source parses"),
        "record Job {\n  job_id: Text;\n}\n"
    );
}

#[test]
fn ailc_exposes_check_format_and_reconstruct_commands() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("ail-m14-{}-{unique}.ail", std::process::id()));
    let source = "record   Job{job_id:Text;}\n";
    fs::write(&path, source).expect("temporary source is writable");
    let binary = env!("CARGO_BIN_EXE_ailc");

    let reconstruct_output = Command::new(binary)
        .arg("reconstruct")
        .arg(&path)
        .output()
        .expect("ailc reconstruct runs");
    assert!(reconstruct_output.status.success());
    assert_eq!(reconstruct_output.stdout, source.as_bytes());

    let format_output = Command::new(binary)
        .arg("format")
        .arg(&path)
        .output()
        .expect("ailc format runs");
    assert!(format_output.status.success());
    assert_eq!(format_output.stdout, b"record Job {\n  job_id: Text;\n}\n");

    let check_output = Command::new(binary)
        .arg("check")
        .arg(&path)
        .output()
        .expect("ailc check runs");
    assert!(check_output.status.success());
    assert_eq!(check_output.stdout, b"ok\n");

    fs::remove_file(path).expect("temporary source is removable");
}
