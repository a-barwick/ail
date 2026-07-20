//! Persistent M8 performance adapter for the frozen Rust V2 fixture boundary.

use std::{
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use ail_job_service_v2::codec::{run_case_value, SingleCaseResult};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
struct Manifest {
    fixtures: Vec<ManifestEntry>,
}

#[derive(Deserialize)]
struct ManifestEntry {
    path: String,
}

#[derive(Deserialize)]
struct Command {
    #[serde(rename = "command")]
    kind: String,
    iterations: Option<usize>,
    duration_ns: Option<u64>,
    sample_stride: Option<usize>,
}

fn emit(value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer(&mut output, value)?;
    writeln!(output)?;
    output.flush()?;
    Ok(())
}

fn load_cases(path: &Path) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let manifest: Manifest = serde_json::from_slice(&fs::read(path)?)?;
    let root = path
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or("manifest does not have a repository root")?;
    manifest
        .fixtures
        .iter()
        .map(|entry| {
            let content = fs::read(root.join(&entry.path))?;
            Ok(serde_json::from_slice(&content)?)
        })
        .collect()
}

fn run(cases: &[Value]) -> Result<Vec<SingleCaseResult>, Box<dyn std::error::Error>> {
    cases
        .iter()
        .cloned()
        .map(|value| Ok(run_case_value(value)?))
        .collect()
}

fn argument() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    while let Some(value) = arguments.next() {
        if value == "--manifest" {
            return arguments
                .next()
                .map(PathBuf::from)
                .ok_or_else(|| "missing --manifest value".into());
        }
    }
    Err("missing --manifest".into())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cases = load_cases(&argument()?)?;
    emit(&json!({"type": "ready", "case_count": cases.len()}))?;
    for line in io::stdin().lock().lines() {
        let command: Command = serde_json::from_str(&line?)?;
        match command.kind.as_str() {
            "verify" => {
                emit(&json!({"type": "verified", "results": run(&cases)?}))?;
            }
            "warmup" => {
                let iterations = command.iterations.ok_or("missing iterations")?;
                let mut checksum = 0;
                for _ in 0..iterations {
                    for result in run(&cases)? {
                        checksum ^= result.case_id.len();
                    }
                }
                emit(&json!({
                    "type": "warmed",
                    "iterations": iterations,
                    "request_count": iterations * cases.len(),
                    "checksum": checksum,
                }))?;
            }
            "measure" => {
                let duration =
                    Duration::from_nanos(command.duration_ns.ok_or("missing duration_ns")?);
                let stride = command.sample_stride.ok_or("missing sample_stride")?;
                let started = Instant::now();
                let mut samples = Vec::new();
                let mut request_count = 0;
                let mut checksum = 0;
                while request_count == 0 || started.elapsed() < duration {
                    for value in &cases {
                        let before = Instant::now();
                        let result = run_case_value(value.clone())?;
                        let elapsed = u64::try_from(before.elapsed().as_nanos())?;
                        if request_count % stride == 0 {
                            samples.push(elapsed);
                        }
                        request_count += 1;
                        checksum ^= result.case_id.len();
                    }
                }
                emit(&json!({
                    "type": "measured",
                    "clock": "std::time::Instant",
                    "elapsed_ns": u64::try_from(started.elapsed().as_nanos())?,
                    "request_count": request_count,
                    "sample_stride": stride,
                    "samples_ns": samples,
                    "checksum": checksum,
                }))?;
            }
            "shutdown" => {
                emit(&json!({"type": "stopped"}))?;
                return Ok(());
            }
            other => return Err(format!("unsupported command {other:?}").into()),
        }
    }
    Ok(())
}
