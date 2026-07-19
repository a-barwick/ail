use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use ail_job_service_v2::codec::{run_case_value, RunnerError, SingleCaseResult};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
struct FixtureManifest {
    fixtures: Vec<FixtureEntry>,
}

#[derive(Debug, Deserialize)]
struct FixtureEntry {
    path: String,
}

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match run(&arguments) {
        Ok(value) => match serde_json::to_string(&value) {
            Ok(encoded) => {
                println!("{encoded}");
                ExitCode::SUCCESS
            }
            Err(error) => fail(&format!("could not encode runner result: {error}")),
        },
        Err(error) => fail(&error.to_string()),
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("rust baseline runner: {message}");
    ExitCode::FAILURE
}

fn run(arguments: &[String]) -> Result<Value, RunnerError> {
    match arguments {
        [flag, path] if flag == "--case" => {
            let result = run_case_file(Path::new(path))?;
            serde_json::to_value(result)
                .map_err(|error| runner_error(format!("could not encode case result: {error}")))
        }
        [flag, path] if flag == "--corpus" => run_corpus(Path::new(path)),
        _ => Err(runner_error(
            "expected exactly --case <fixture> or --corpus <manifest>",
        )),
    }
}

fn run_case_file(path: &Path) -> Result<SingleCaseResult, RunnerError> {
    let bytes = fs::read(path)
        .map_err(|error| runner_error(format!("could not read {}: {error}", path.display())))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| runner_error(format!("could not parse {}: {error}", path.display())))?;
    run_case_value(value)
}

fn run_corpus(path: &Path) -> Result<Value, RunnerError> {
    let bytes = fs::read(path)
        .map_err(|error| runner_error(format!("could not read {}: {error}", path.display())))?;
    let manifest: FixtureManifest = serde_json::from_slice(&bytes)
        .map_err(|error| runner_error(format!("could not parse {}: {error}", path.display())))?;
    let root = repository_root(path)?;
    let results = manifest
        .fixtures
        .iter()
        .map(|entry| run_case_file(&root.join(&entry.path)))
        .collect::<Result<Vec<_>, _>>()?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    Ok(serde_json::json!({
        "result_format": 1,
        "fixture_manifest_sha256": digest,
        "results": results,
    }))
}

fn repository_root(manifest_path: &Path) -> Result<PathBuf, RunnerError> {
    let canonical = manifest_path.canonicalize().map_err(|error| {
        runner_error(format!(
            "could not resolve manifest {}: {error}",
            manifest_path.display()
        ))
    })?;
    canonical
        .ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .map(Path::to_path_buf)
        .ok_or_else(|| runner_error("fixture manifest is not inside the repository"))
}

fn runner_error(message: impl Into<String>) -> RunnerError {
    RunnerError::new(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .and_then(Path::parent)
            .expect("v2 crate is nested under benchmarks/baselines/rust")
            .to_path_buf()
    }

    #[test]
    fn argument_contract_rejects_missing_or_extra_arguments() {
        assert!(run(&[]).is_err());
        assert!(run(&["--case".into()]).is_err());
        assert!(run(&["--case".into(), "x".into(), "y".into()]).is_err());
    }

    #[test]
    fn one_case_command_returns_the_normalized_result() {
        let fixture = repo_root()
            .join("benchmarks/fixtures/public/create_job/uc001-v1-created-empty-payload.json");
        let result =
            run(&["--case".into(), fixture.to_string_lossy().into_owned()]).expect("run one case");

        assert_eq!(result["result_format"], 1);
        assert_eq!(result["case_id"], "uc001-v1-created-empty-payload");
        assert_eq!(result["actual"]["response"]["result"]["kind"], "created");
    }

    #[test]
    fn corpus_command_preserves_manifest_digest_count_and_order() {
        let manifest = repo_root().join("benchmarks/fixtures/manifest.json");
        let result =
            run(&["--corpus".into(), manifest.to_string_lossy().into_owned()]).expect("run corpus");
        let results = result["results"].as_array().expect("corpus results");

        assert_eq!(results.len(), 37);
        assert_eq!(
            result["fixture_manifest_sha256"],
            "33b8369ca3680367b5371811ce52ef639a878696698f70765def0f6c9e8c1eb5"
        );
        assert_eq!(results[0]["case_id"], "uc001-v1-created-empty-payload");
        assert_eq!(results[36]["case_id"], "uc003-v1-stored-job-adapted");
    }

    #[test]
    fn missing_fixture_is_reported_as_a_runner_error() {
        let missing = repo_root().join("benchmarks/fixtures/public/missing.json");
        let error = run(&["--case".into(), missing.to_string_lossy().into_owned()])
            .expect_err("missing fixture should fail");
        assert!(error.to_string().contains("could not read"));
    }
}
