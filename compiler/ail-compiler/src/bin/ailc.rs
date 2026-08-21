use std::env;
use std::fs;
use std::process::ExitCode;

use ail_compiler::{
    CliCheckError, CliPublishError, EvolutionBuildFailure, SourceSetDiagnostic, check_cli_path,
    format_source, parse, publish_cli_path, reconstruct,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().ok_or_else(|| usage("missing command"))?;
    let path = arguments
        .next()
        .ok_or_else(|| usage("missing source path"))?;
    if arguments.next().is_some() {
        return Err(usage("too many arguments"));
    }

    match command.as_str() {
        "check" => check(&path),
        "publish" => publish(&path),
        "format" => {
            let source = read_source(&path)?;
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
            let source = read_source(&path)?;
            let parsed = parse(&source);
            print!("{}", reconstruct(&parsed.tokens));
            Ok(())
        }
        _ => Err(usage("unknown command")),
    }
}

fn check(path: &str) -> Result<(), String> {
    match check_cli_path(path) {
        Ok(()) => {
            println!("ok");
            Ok(())
        }
        Err(error) => report_check_error(error),
    }
}

fn publish(path: &str) -> Result<(), String> {
    match publish_cli_path(path) {
        Ok(revision) => {
            println!("published");
            println!("revision_id={}", revision.revision_id);
            println!("source_set_digest={}", revision.source_set_digest);
            Ok(())
        }
        Err(CliPublishError::Check(error)) => report_check_error(error),
        Err(CliPublishError::Write(message)) => Err(message),
    }
}

fn report_check_error(error: CliCheckError) -> Result<(), String> {
    match error {
        CliCheckError::Io(message) => Err(message),
        CliCheckError::Build(failure) => report_build_failure(&failure),
        CliCheckError::Architecture(failure) => {
            for diagnostic in failure.diagnostics {
                eprintln!("{diagnostic}");
            }
            Err("source contains architecture diagnostics".to_owned())
        }
    }
}

fn report_build_failure(failure: &EvolutionBuildFailure) -> Result<(), String> {
    for diagnostic in &failure.diagnostics {
        eprintln!("{}", format_source_set_diagnostic(diagnostic));
    }
    for cause in &failure.causes {
        if !failure
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == cause)
        {
            eprintln!("{cause}");
        }
    }
    Err("source contains check diagnostics".to_owned())
}

fn format_source_set_diagnostic(diagnostic: &SourceSetDiagnostic) -> String {
    let mut line = format!(
        "{}:{}:{}:{}:",
        diagnostic.code, diagnostic.path, diagnostic.span.start, diagnostic.span.end
    );
    for (key, value) in &diagnostic.details {
        line.push(' ');
        line.push_str(key);
        line.push('=');
        line.push_str(value);
    }
    line
}

fn read_source(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))
}

fn usage(reason: &str) -> String {
    format!("{reason}\nusage: ailc <check|publish|format|reconstruct> <source>")
}
