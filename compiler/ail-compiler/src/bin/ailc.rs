use std::env;
use std::fs;
use std::process::ExitCode;

use ail_compiler::{format_source, parse, reconstruct};

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
    let source = fs::read_to_string(&path).map_err(|error| format!("{path}: {error}"))?;

    match command.as_str() {
        "check" => {
            let parsed = parse(&source);
            if parsed.diagnostics.is_empty() {
                println!("ok");
                Ok(())
            } else {
                for diagnostic in parsed.diagnostics {
                    eprintln!(
                        "{}:{}:{}: expected {}, found {}",
                        diagnostic.code,
                        diagnostic.span.start,
                        diagnostic.span.end,
                        diagnostic.expected,
                        diagnostic.actual
                    );
                }
                Err("source contains parse diagnostics".to_owned())
            }
        }
        "format" => {
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
            let parsed = parse(&source);
            print!("{}", reconstruct(&parsed.tokens));
            Ok(())
        }
        _ => Err(usage("unknown command")),
    }
}

fn usage(reason: &str) -> String {
    format!("{reason}\nusage: ailc <check|format|reconstruct> <source.ail>")
}
