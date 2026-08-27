use std::env;
use std::fs;
use std::process::ExitCode;

use ail_compiler::{
    BehaviorValidation, CliCheckError, CliPublishError, EvolutionBuildFailure, SourceFinding,
    check_cli_path_with_evidence, findings_document, format_source, parse, publish_cli_path,
    reconstruct,
};

const CHECK_SUMMARY: &str = "source contains check diagnostics";
const ARCHITECTURE_SUMMARY: &str = "source contains architecture diagnostics";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

struct Invocation {
    command: String,
    path: String,
    json: bool,
}

fn parse_arguments() -> Result<Invocation, String> {
    let mut command = None;
    let mut path = None;
    let mut json = false;
    for argument in env::args().skip(1) {
        if argument == "--json" {
            json = true;
        } else if command.is_none() {
            command = Some(argument);
        } else if path.is_none() {
            path = Some(argument);
        } else {
            return Err(usage("too many arguments"));
        }
    }
    Ok(Invocation {
        command: command.ok_or_else(|| usage("missing command"))?,
        path: path.ok_or_else(|| usage("missing source path"))?,
        json,
    })
}

fn run() -> Result<(), String> {
    let invocation = parse_arguments()?;
    match invocation.command.as_str() {
        "check" => check(&invocation.path, invocation.json),
        "publish" => publish(&invocation.path, invocation.json),
        "format" => {
            let source = read_source(&invocation.path)?;
            let formatted = format_source(&source).map_err(|diagnostics| {
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.code)
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;
            print!("{formatted}");
            Ok(())
        }
        "reconstruct" => {
            let source = read_source(&invocation.path)?;
            let parsed = parse(&source);
            print!("{}", reconstruct(&parsed.tokens));
            Ok(())
        }
        _ => Err(usage("unknown command")),
    }
}

fn check(path: &str, json: bool) -> Result<(), String> {
    match check_cli_path_with_evidence(path) {
        Ok(success) => {
            if json {
                if let Some(behavior) = success.behavior_validation.as_ref() {
                    let document = serde_json::json!({
                        "status": "ok",
                        "summary": "",
                        "behavior_validation": behavior_json(behavior),
                        "findings": [],
                    });
                    println!("{document}");
                } else {
                    print!("{}", findings_document("ok", "", &[]));
                }
            } else {
                println!("ok");
                if let Some(behavior) = success.behavior_validation.as_ref() {
                    print_behavior(behavior);
                }
            }
            Ok(())
        }
        Err(error) => report_check_error(error, json),
    }
}

fn publish(path: &str, json: bool) -> Result<(), String> {
    match publish_cli_path(path) {
        Ok(revision) => {
            if json {
                let mut document = serde_json::json!({
                    "status": "published",
                    "revision_id": revision.revision_id,
                    "source_set_digest": revision.source_set_digest,
                    "findings": [],
                });
                if let Some(behavior) = revision.behavior_validation.as_ref() {
                    document
                        .as_object_mut()
                        .expect("publish document is an object")
                        .insert("behavior_validation".into(), behavior_json(behavior));
                }
                println!("{document}");
            } else {
                println!("published");
                println!("revision_id={}", revision.revision_id);
                println!("source_set_digest={}", revision.source_set_digest);
                if let Some(behavior) = revision.behavior_validation.as_ref() {
                    print_behavior(behavior);
                }
            }
            Ok(())
        }
        Err(CliPublishError::Check(error)) => report_check_error(error, json),
        Err(CliPublishError::Write(message)) => Err(message),
    }
}

fn behavior_json(behavior: &BehaviorValidation) -> serde_json::Value {
    serde_json::json!({
        "status": behavior.status,
        "cases_passed": behavior.cases_passed,
        "cases_total": behavior.cases_total,
    })
}

fn print_behavior(behavior: &BehaviorValidation) {
    println!(
        "behavior: {} {}/{}",
        behavior.status, behavior.cases_passed, behavior.cases_total
    );
}

fn report_check_error(error: CliCheckError, json: bool) -> Result<(), String> {
    match error {
        CliCheckError::Io(message) => {
            if json {
                print!("{}", findings_document("failed", &message, &[]));
            }
            Err(message)
        }
        CliCheckError::Build(failure) => {
            if json {
                print!(
                    "{}",
                    findings_document("failed", CHECK_SUMMARY, &failure.findings)
                );
            } else {
                report_build_failure(&failure);
            }
            Err(CHECK_SUMMARY.to_owned())
        }
        CliCheckError::Architecture(failure) => {
            if json {
                print!(
                    "{}",
                    findings_document("failed", ARCHITECTURE_SUMMARY, &failure.findings)
                );
            } else {
                report_findings(&failure.findings);
            }
            Err(ARCHITECTURE_SUMMARY.to_owned())
        }
    }
}

fn report_build_failure(failure: &EvolutionBuildFailure) {
    report_findings(&failure.findings);
    for cause in &failure.causes {
        if !failure
            .findings
            .iter()
            .any(|finding| finding.code == *cause)
        {
            eprintln!("{cause}");
        }
    }
}

fn report_findings(findings: &[SourceFinding]) {
    for finding in findings {
        eprintln!("{}", finding.render());
    }
}

fn read_source(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))
}

fn usage(reason: &str) -> String {
    format!("{reason}\nusage: ailc <check|publish|format|reconstruct> [--json] <source>")
}
