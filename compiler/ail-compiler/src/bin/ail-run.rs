//! `ail-run` executes the frozen bytes of a published revision.
//!
//! `ailc` never executes a program. This binary runs only what
//! `ailc publish` froze under `<dir>/.ail/revisions/<current>/sources/`. It
//! reads no live `.ail` file from the workspace directory, so an unpublished
//! edit cannot change what runs.

use std::env;
use std::fmt::Write;
use std::process::ExitCode;

use ail_compiler::{ExecutionResponse, NoCapabilities, RuntimeValue, load_published_program};

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
    directory: String,
    function: String,
    arguments: Vec<RuntimeValue>,
}

fn run() -> Result<(), String> {
    let invocation = parse_arguments()?;
    let program = load_published_program(&invocation.directory)
        .map_err(|refusal| format!("refused\n{}", refusal.render()))?;
    let mut capabilities = NoCapabilities;
    let response = program.run(
        &invocation.function,
        invocation.arguments,
        &mut capabilities,
    );
    match response {
        ExecutionResponse::Completed(success) => {
            println!("completed");
            println!("revision_id={}", program.revision_id());
            println!("source_set_digest={}", program.source_set_digest());
            println!("function={}", invocation.function);
            println!("value={}", render_value(&success.value));
            println!("calls={}", success.calls.len());
            Ok(())
        }
        ExecutionResponse::Failed(failure) => {
            println!("failed");
            println!("revision_id={}", program.revision_id());
            println!("source_set_digest={}", program.source_set_digest());
            println!("function={}", invocation.function);
            println!("calls={}", failure.calls.len());
            let mut message = failure.fault.code.to_owned();
            for (key, value) in &failure.fault.expected {
                write!(message, " expected.{key}={value}").expect("writing to String cannot fail");
            }
            for (key, value) in &failure.fault.actual {
                write!(message, " actual.{key}={value}").expect("writing to String cannot fail");
            }
            Err(message)
        }
    }
}

fn parse_arguments() -> Result<Invocation, String> {
    let mut positional = Vec::new();
    let mut arguments = Vec::new();
    let mut raw = env::args().skip(1);
    while let Some(argument) = raw.next() {
        match argument.as_str() {
            "--text" => arguments.push(RuntimeValue::Text(value_for("--text", &mut raw)?)),
            "--int" => {
                let spelling = value_for("--int", &mut raw)?;
                let value = spelling.parse::<u128>().map_err(|_| {
                    usage(&format!(
                        "--int expects a non-negative integer, got {spelling}"
                    ))
                })?;
                arguments.push(RuntimeValue::Int(value));
            }
            "--bytes" => {
                let spelling = value_for("--bytes", &mut raw)?;
                arguments.push(RuntimeValue::Bytes(parse_hex(&spelling)?));
            }
            other if other.starts_with("--") => {
                return Err(usage(&format!("unknown option {other}")));
            }
            _ if positional.len() < 2 => positional.push(argument),
            _ => return Err(usage("too many arguments")),
        }
    }
    let mut positional = positional.into_iter();
    Ok(Invocation {
        directory: positional
            .next()
            .ok_or_else(|| usage("missing workspace directory"))?,
        function: positional
            .next()
            .ok_or_else(|| usage("missing entry function"))?,
        arguments,
    })
}

fn value_for(option: &str, raw: &mut impl Iterator<Item = String>) -> Result<String, String> {
    raw.next()
        .ok_or_else(|| usage(&format!("{option} expects a value")))
}

fn parse_hex(spelling: &str) -> Result<Vec<u8>, String> {
    if spelling.len() % 2 != 0 {
        return Err(usage("--bytes expects an even number of hex digits"));
    }
    let digits = spelling.as_bytes();
    let mut bytes = Vec::with_capacity(digits.len() / 2);
    for pair in digits.chunks_exact(2) {
        let text = std::str::from_utf8(pair).map_err(|_| usage("--bytes expects hex digits"))?;
        bytes.push(u8::from_str_radix(text, 16).map_err(|_| usage("--bytes expects hex digits"))?);
    }
    Ok(bytes)
}

/// Render one runtime value deterministically.
///
/// Record fields and list elements print in the interpreter's own order, so two
/// runs of the same frozen revision print the same line.
fn render_value(value: &RuntimeValue) -> String {
    match value {
        RuntimeValue::Unit => "unit".to_owned(),
        RuntimeValue::Text(text) => {
            serde_json::to_string(text).expect("a Rust string always encodes as JSON")
        }
        RuntimeValue::Int(number) => number.to_string(),
        RuntimeValue::Bool(flag) => flag.to_string(),
        RuntimeValue::Bytes(bytes) => {
            let mut rendered = "bytes:".to_owned();
            for byte in bytes {
                write!(rendered, "{byte:02x}").expect("writing to String cannot fail");
            }
            rendered
        }
        RuntimeValue::List(values) => format!(
            "[{}]",
            values
                .iter()
                .map(render_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        RuntimeValue::Cancellation(token) => format!("cancellation:{}", token.id),
        RuntimeValue::Record { type_name, fields } => {
            if fields.is_empty() {
                return format!("{type_name} {{}}");
            }
            format!(
                "{type_name} {{ {} }}",
                fields
                    .iter()
                    .map(|(name, value)| format!("{name}: {}", render_value(value)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        RuntimeValue::Variant {
            type_name,
            case,
            payload,
        } => match payload {
            Some(payload) => format!("{type_name}::{case}({})", render_value(payload)),
            None => format!("{type_name}::{case}"),
        },
    }
}

fn usage(reason: &str) -> String {
    format!(
        "{reason}\nusage: ail-run <workspace-directory> <function> \
         [--text VALUE | --int VALUE | --bytes HEX]...\n\
         ail-run executes the published revision under <workspace-directory>/.ail, \
         never the live source files.\n\
         Record, variant, and list arguments cannot be written on this command line."
    )
}

#[cfg(test)]
mod tests {
    use super::render_value;
    use ail_compiler::RuntimeValue;

    #[test]
    fn records_and_variants_render_with_their_named_types() {
        let value = RuntimeValue::variant(
            "contracts.ReviewDecision",
            "Approved",
            Some(RuntimeValue::record(
                "contracts.ApprovedJob",
                [
                    ("job_id", RuntimeValue::Text("fixture-job".to_owned())),
                    ("payload", RuntimeValue::Bytes(vec![0x01, 0xff])),
                ],
            )),
        );
        assert_eq!(
            render_value(&value),
            "contracts.ReviewDecision::Approved(contracts.ApprovedJob \
             { job_id: \"fixture-job\", payload: bytes:01ff })"
        );
    }
}
