#!/usr/bin/env python3
"""Build and verify answer-free M8 calibration workspaces."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any, NoReturn


ROOT = Path(__file__).resolve().parents[2]
CALIBRATION_ROOT = ROOT / "benchmarks" / "calibration"
TASK_START_LOCK = CALIBRATION_ROOT / "task-starts.json"
LANGUAGES = ("rust", "go", "python", "typescript")
TASKS = ("UC-001", "UC-003")
CODEX_CLI = Path(
    os.environ.get(
        "AIL_CODEX_CLI",
        "/Applications/ChatGPT.app/Contents/Resources/codex",
    )
)
MODEL = "gpt-5.6-sol"
MODEL_EFFORT = "high"
AGENT_SECONDS = 300
INPUT_TOKEN_LIMIT = 100_000
TOOL_PATH = (
    "/opt/homebrew/Cellar/python@3.13/3.13.5/Frameworks/Python.framework/"
    "Versions/3.13/bin:"
    "/opt/homebrew/Cellar/node/23.10.0_1/bin:"
    "/usr/local/go/bin:"
    "/Users/austinbarwick/.local/bin:"
    "/Users/austinbarwick/.cargo/bin:"
    "/usr/bin:/bin"
)


class CalibrationError(Exception):
    """A stable calibration verification failure."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message


def _raise(code: str, message: str) -> NoReturn:
    raise CalibrationError(code, message)


def _canonical(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _load_object(path: Path, code: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        _raise(code, f"{path}: {error}")
    if not isinstance(value, dict):
        _raise(code, f"{path}: must contain a JSON object")
    return value


def _checkpoint_files(language: str, checkpoint_id: str) -> list[str]:
    path = ROOT / "benchmarks" / "baselines" / language / "checkpoints.json"
    value = _load_object(path, "checkpoint_invalid")
    for checkpoint in value.get("checkpoints", []):
        if checkpoint.get("id") == checkpoint_id:
            files = checkpoint.get("files")
            if isinstance(files, list) and all(isinstance(item, str) for item in files):
                return files
    _raise("checkpoint_invalid", f"{language}: missing {checkpoint_id}")


def _task_source_files(language: str, task: str) -> list[str]:
    v1 = _checkpoint_files(language, f"{language}-v1")
    if task == "UC-001":
        return v1

    v2 = _checkpoint_files(language, f"{language}-v2")
    visible_tests: list[str] = []
    for path in v2:
        name = Path(path).name
        if name in {
            "checkpoints.rs",
            "checkpoints_test.go",
            "test_contract.py",
            "contract.test.ts",
        }:
            continue
        if (
            "/tests/" in path
            or name.endswith("_test.go")
            or (language in {"python", "typescript"} and "/tests/" in path)
        ):
            visible_tests.append(path)
    return sorted(set(v1 + visible_tests))


def _copy_file(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)


def _copy_shared_inputs(workspace: Path) -> None:
    shared = (
        "benchmarks/contracts/runner-contract.md",
        "benchmarks/contracts/run-classification.md",
        "benchmarks/fixtures/manifest.json",
        "benchmarks/schemas/job-service-fixture.schema.json",
        "benchmarks/schemas/runner-result.schema.json",
        "benchmarks/tasks/uc001-implement-create-job.md",
        "benchmarks/tasks/uc003-add-priority.md",
    )
    for raw_path in shared:
        _copy_file(ROOT / raw_path, workspace / raw_path)
    for source in sorted((ROOT / "benchmarks" / "fixtures" / "public").rglob("*.json")):
        _copy_file(source, workspace / source.relative_to(ROOT))


def _replace_between(
    path: Path,
    start: str,
    end: str,
    replacement: str,
) -> None:
    text = path.read_text(encoding="utf-8")
    start_index = text.find(start)
    end_index = text.find(end, start_index + len(start))
    if start_index < 0 or end_index < 0:
        _raise("task_start_transform_failed", f"{path}: transformation markers missing")
    path.write_text(
        text[:start_index] + replacement + text[end_index:],
        encoding="utf-8",
    )


def _make_uc001_incomplete(workspace: Path, language: str) -> None:
    baseline = workspace / "benchmarks" / "baselines" / language
    if language == "rust":
        cargo = baseline / "Cargo.toml"
        cargo.write_text(
            cargo.read_text(encoding="utf-8").replace(
                'members = ["v1", "v2"]',
                'members = ["v1"]',
            ),
            encoding="utf-8",
        )
        path = baseline / "v1" / "src" / "lib.rs"
        _replace_between(
            path,
            "pub fn create_job",
            "#[cfg(test)]",
            """pub fn create_job(
    _request: CreateJobRequest,
    _store: &mut impl JobStore,
) -> CreateJobResult {
    todo!("implement UC-001")
}

fn valid_job_id(_job_id: &str) -> bool {
    todo!("implement UC-001 validation")
}

""",
        )
    elif language == "go":
        path = baseline / "v1" / "jobservice.go"
        _replace_between(
            path,
            "func CreateJob",
            "func isASCIIAlphanumeric",
            """func CreateJob(request CreateJobRequest, store JobStore) CreateJobResult {
	panic("implement UC-001")
}

func validate(request CreateJobRequest) []ValidationIssue {
	panic("implement UC-001 validation")
}

func validJobID(jobID string) bool {
	panic("implement UC-001 validation")
}

""",
        )
    elif language == "python":
        project = baseline / "pyproject.toml"
        project.write_text(
            project.read_text(encoding="utf-8")
            .replace('files = ["v1", "v2", "tests"]', 'files = ["v1", "tests/test_v1.py"]')
            .replace('source = ["v1", "v2"]', 'source = ["v1"]'),
            encoding="utf-8",
        )
        path = baseline / "v1" / "job_service.py"
        _replace_between(
            path,
            "def create_job",
            "def _validate",
            """def create_job(request: CreateJobRequest, store: JobStore) -> CreateJobResult:
    \"\"\"Implement the UC-001 contract described by the supplied task.\"\"\"
    raise NotImplementedError("implement UC-001")


""",
        )
        text = path.read_text(encoding="utf-8")
        validation_index = text.find("def _validate")
        if validation_index < 0:
            _raise("task_start_transform_failed", f"{path}: validation marker missing")
        path.write_text(
            text[:validation_index]
            + """def _validate(request: CreateJobRequest) -> tuple[ValidationIssue, ...]:
    raise NotImplementedError("implement UC-001 validation")
""",
            encoding="utf-8",
        )
    else:
        path = baseline / "v1" / "job-service.ts"
        _replace_between(
            path,
            "export function createJob",
            "function validate",
            """export function createJob(
  request: CreateJobRequest,
  store: JobStore,
): CreateJobResult {
  void request;
  void store;
  throw new Error("implement UC-001");
}

""",
        )
        _replace_between(
            path,
            "function validate",
            "function assertNever",
            """function validate(
  request: CreateJobRequest,
): readonly ValidationIssue[] {
  void request;
  throw new Error("implement UC-001 validation");
}

""",
        )


def build_task_start(language: str, task: str, destination: Path) -> None:
    """Materialize one deterministic, answer-free agent workspace."""

    if language not in LANGUAGES or task not in TASKS:
        _raise("task_start_invalid", f"unsupported configuration {language}/{task}")
    if destination.exists():
        shutil.rmtree(destination)
    destination.mkdir(parents=True)
    _copy_shared_inputs(destination)
    for raw_path in _task_source_files(language, task):
        _copy_file(ROOT / raw_path, destination / raw_path)
    if task == "UC-001":
        _make_uc001_incomplete(destination, language)
    marker = {
        "task_start_format": 1,
        "language": language,
        "task": task,
        "task_path": (
            "benchmarks/tasks/uc001-implement-create-job.md"
            if task == "UC-001"
            else "benchmarks/tasks/uc003-add-priority.md"
        ),
        "answer_policy": (
            "V1 public contracts and tests with implementation holes"
            if task == "UC-001"
            else "working V1 source and V2 tests without V2 implementation source"
        ),
    }
    (destination / "task-start.json").write_text(_canonical(marker), encoding="utf-8")


def _tree_records(root: Path) -> list[dict[str, str]]:
    records: list[dict[str, str]] = []
    for path in sorted(item for item in root.rglob("*") if item.is_file()):
        records.append(
            {
                "path": path.relative_to(root).as_posix(),
                "sha256": _sha256(path),
            }
        )
    return records


def _tree_digest(records: list[dict[str, str]]) -> str:
    digest = hashlib.sha256()
    for record in records:
        digest.update(
            f"{record['sha256']}  {record['path']}\n".encode("utf-8")
        )
    return digest.hexdigest()


def _task_start_value() -> dict[str, Any]:
    configurations: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="ail-m8-task-starts-") as raw:
        temporary = Path(raw)
        for language in LANGUAGES:
            for task in TASKS:
                workspace = temporary / language / task.lower()
                build_task_start(language, task, workspace)
                records = _tree_records(workspace)
                configurations.append(
                    {
                        "language": language,
                        "task": task,
                        "source_tree_sha256": _tree_digest(records),
                        "file_count": len(records),
                    }
                )
    return {
        "task_start_lock_format": 1,
        "generator": {
            "path": "benchmarks/tools/calibration.py",
            "sha256": _sha256(Path(__file__)),
        },
        "configurations": configurations,
    }


def write_task_start_lock() -> None:
    CALIBRATION_ROOT.mkdir(parents=True, exist_ok=True)
    TASK_START_LOCK.write_text(_canonical(_task_start_value()), encoding="utf-8")
    print(f"Wrote {TASK_START_LOCK.relative_to(ROOT)}.")


def check_task_start_lock() -> None:
    actual = _load_object(TASK_START_LOCK, "task_start_lock_missing")
    if TASK_START_LOCK.read_text(encoding="utf-8") != _canonical(actual):
        _raise("task_start_lock_changed", "task-start lock is not canonical JSON")
    expected = _task_start_value()
    if actual != expected:
        _raise("task_start_lock_changed", "generated task starts differ from lock")
    expected_pairs = [(language, task) for language in LANGUAGES for task in TASKS]
    actual_pairs = [
        (item.get("language"), item.get("task"))
        for item in actual.get("configurations", [])
    ]
    if actual_pairs != expected_pairs:
        _raise("task_start_lock_changed", "configuration matrix is incomplete")
    print("M8 task starts passed: 8 answer-free configurations match the lock.")


def _run(
    command: list[str],
    *,
    cwd: Path,
    timeout: int = 300,
    environment: dict[str, str] | None = None,
) -> dict[str, Any]:
    started = time.monotonic_ns()
    merged_environment = os.environ.copy()
    merged_environment["PATH"] = TOOL_PATH
    merged_environment["LANG"] = "C"
    merged_environment["LC_ALL"] = "C"
    if environment:
        merged_environment.update(environment)
    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            env=merged_environment,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
            check=False,
        )
        timed_out = False
        return_code = completed.returncode
        stdout = completed.stdout
        stderr = completed.stderr
    except subprocess.TimeoutExpired as error:
        timed_out = True
        return_code = None
        stdout = (
            error.stdout.decode("utf-8", "replace")
            if isinstance(error.stdout, bytes)
            else error.stdout or ""
        )
        stderr = (
            error.stderr.decode("utf-8", "replace")
            if isinstance(error.stderr, bytes)
            else error.stderr or ""
        )
    return {
        "command": command,
        "elapsed_ms": (time.monotonic_ns() - started) // 1_000_000,
        "return_code": return_code,
        "timed_out": timed_out,
        "stdout": stdout,
        "stderr": stderr,
    }


def _copy_tool_environment(workspace: Path, language: str) -> None:
    """Package already-installed dependencies without adding them to source."""

    destination = workspace / "benchmarks" / "baselines" / language
    source = ROOT / "benchmarks" / "baselines" / language
    if language == "python" and (source / ".venv").is_dir():
        shutil.copytree(source / ".venv", destination / ".venv", symlinks=True)
    elif language == "typescript" and (source / "node_modules").is_dir():
        shutil.copytree(
            source / "node_modules",
            destination / "node_modules",
            symlinks=True,
        )


def _git_initialize(workspace: Path) -> None:
    for command in (
        ["git", "init"],
        ["git", "add", "."],
        [
            "git",
            "-c",
            "user.name=AIL Calibration",
            "-c",
            "user.email=calibration@invalid",
            "commit",
            "-m",
            "Frozen task start",
        ],
    ):
        result = _run(command, cwd=workspace, timeout=60)
        if result["return_code"] != 0:
            _raise("trial_setup_failed", result["stderr"])


def _agent_prompt(workspace: Path, language: str, task: str) -> str:
    task_path = (
        workspace / "benchmarks" / "tasks" / "uc001-implement-create-job.md"
        if task == "UC-001"
        else workspace / "benchmarks" / "tasks" / "uc003-add-priority.md"
    )
    baseline = workspace / "benchmarks" / "baselines" / language
    context: list[str] = []
    excluded_names = {
        "checkpoints.json",
        "runner.json",
        "seed-locations.json",
        "verification-manifest.json",
        "verification-manifest.lock.json",
    }
    for path in sorted(item for item in baseline.rglob("*") if item.is_file()):
        relative = path.relative_to(workspace).as_posix()
        if (
            path.name in excluded_names
            or ".venv" in path.parts
            or "node_modules" in path.parts
            or path.suffix not in {".rs", ".go", ".py", ".ts", ".toml", ".json", ".mjs"}
        ):
            continue
        context.append(
            f"\n--- {relative} ---\n{path.read_text(encoding='utf-8')}"
        )
    public_context = [
        workspace / "benchmarks" / "contracts" / "runner-contract.md",
        workspace / "benchmarks" / "schemas" / "job-service-fixture.schema.json",
    ]
    representative_names = (
        "uc001-v1-created-empty-payload.json",
        "uc001-v1-multiple-invalid-ordered.json",
        "uc001-v1-duplicate.json",
        "uc001-v1-unavailable.json",
        "uc003-v1-request-adapted.json",
        "uc003-v2-created-priority-high.json",
        "uc003-v2-priority-missing.json",
        "uc003-v2-priority-unknown.json",
        "uc003-v1-stored-job-adapted.json",
    )
    for name in representative_names:
        matches = list(
            (workspace / "benchmarks" / "fixtures" / "public").rglob(name)
        )
        if matches:
            public_context.append(matches[0])
    for path in public_context:
        relative = path.relative_to(workspace).as_posix()
        context.append(
            f"\n--- {relative} ---\n{path.read_text(encoding='utf-8')}"
        )
    return (
        "Act as a one-pass coding agent. Complete the task from the supplied "
        "answer-free workspace snapshot. Do not call tools: all relevant source, "
        "configuration, and visible tests are included below. Return only the "
        "structured file set required by the output schema. Each path must be "
        f"under benchmarks/baselines/{language}/. Include the complete final "
        "contents of every created or changed file, and no unchanged files. Do "
        "not change tests, benchmark tasks, fixtures, or contracts.\n\n"
        f"{task_path.read_text(encoding='utf-8')}\n"
        + "".join(context)
    )


def _write_change_schema(workspace: Path, language: str) -> Path:
    schema = {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["files"],
        "properties": {
            "files": {
                "type": "array",
                "minItems": 1,
                "items": {
                    "type": "object",
                    "required": ["path", "content"],
                    "properties": {
                        "path": {
                            "type": "string",
                            "pattern": (
                                "^benchmarks/baselines/"
                                + language
                                + "/[A-Za-z0-9_./@+-]+$"
                            ),
                        },
                        "content": {"type": "string"},
                    },
                    "additionalProperties": False,
                },
            }
        },
        "additionalProperties": False,
    }
    path = workspace / ".calibration-output-schema.json"
    path.write_text(_canonical(schema), encoding="utf-8")
    return path


def _apply_agent_changes(
    workspace: Path, language: str, stdout: str
) -> tuple[int, str | None]:
    messages: list[str] = []
    for line in stdout.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        item = event.get("item", {}) if isinstance(event, dict) else {}
        if (
            event.get("type") == "item.completed"
            and item.get("type") == "agent_message"
            and isinstance(item.get("text"), str)
        ):
            messages.append(item["text"])
    if not messages:
        return 0, "missing_structured_change"
    try:
        change = json.loads(messages[-1])
    except json.JSONDecodeError:
        return 0, "malformed_structured_change"
    files = change.get("files") if isinstance(change, dict) else None
    if not isinstance(files, list) or not files:
        return 0, "empty_structured_change"
    prefix = f"benchmarks/baselines/{language}/"
    seen: set[str] = set()
    for entry in files:
        if not isinstance(entry, dict):
            return 0, "malformed_structured_change"
        raw_path = entry.get("path")
        content = entry.get("content")
        if (
            not isinstance(raw_path, str)
            or not raw_path.startswith(prefix)
            or not isinstance(content, str)
            or raw_path in seen
        ):
            return 0, "invalid_structured_change_path"
        destination = (workspace / raw_path).resolve()
        try:
            destination.relative_to(workspace)
        except ValueError:
            return 0, "invalid_structured_change_path"
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(content, encoding="utf-8")
        seen.add(raw_path)
    return len(files), None


def _parse_agent_events(stdout: str) -> dict[str, Any]:
    events: list[dict[str, Any]] = []
    for line in stdout.splitlines():
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            events.append(value)
    completed = [
        event.get("item", {})
        for event in events
        if event.get("type") == "item.completed"
    ]
    usage = next(
        (
            event.get("usage", {})
            for event in reversed(events)
            if event.get("type") == "turn.completed"
        ),
        {},
    )
    edit_count = sum(item.get("type") == "file_change" for item in completed)
    validations = [
        item
        for item in completed
        if item.get("type") == "command_execution"
        and any(
            token in item.get("command", "").lower()
            for token in (
                " test",
                "pytest",
                "mypy",
                "ruff",
                "clippy",
                "cargo check",
                "go vet",
                "npm run build",
                "tsc ",
            )
        )
    ]
    return {
        "thread_id": next(
            (
                event.get("thread_id")
                for event in events
                if event.get("type") == "thread.started"
            ),
            None,
        ),
        "input_tokens": usage.get("input_tokens"),
        "cached_input_tokens": usage.get("cached_input_tokens"),
        "output_tokens": usage.get("output_tokens"),
        "reasoning_output_tokens": usage.get("reasoning_output_tokens"),
        "edit_count": edit_count,
        "validation_attempts": len(validations),
        "incomplete_validation_attempts": sum(
            item.get("exit_code") not in (0, None) for item in validations
        ),
        "event_count": len(events),
    }


def _verification_commands(language: str, task: str) -> list[list[str]]:
    if language == "rust":
        package = ["-p", "ail-job-service-v1"] if task == "UC-001" else ["--workspace"]
        return [
            ["cargo", "fmt", "--all", "--check"],
            ["cargo", "clippy", *package, "--all-targets", "--locked", "--offline", "--", "-D", "warnings"],
            ["cargo", "test", *package, "--locked", "--offline"],
        ]
    if language == "go":
        return [["go", "vet", "./..."], ["go", "test", "./..."]]
    if language == "python":
        return [
            [".venv/bin/ruff", "format", "--check", "."],
            [".venv/bin/ruff", "check", "."],
            [".venv/bin/mypy"],
            [".venv/bin/pytest"],
        ]
    return [
        ["npm", "run", "format:check"],
        ["npm", "run", "lint"],
        ["npm", "run", "build"],
        ["npm", "test"],
    ]


def _format_workspace(baseline: Path, language: str) -> dict[str, Any]:
    if language == "rust":
        command = ["cargo", "fmt", "--all"]
    elif language == "go":
        files = [
            path.relative_to(baseline).as_posix()
            for path in sorted(baseline.rglob("*.go"))
        ]
        command = ["gofmt", "-w", *files]
    elif language == "python":
        command = [".venv/bin/ruff", "format", "."]
    else:
        command = ["npm", "run", "format"]
    return _run(command, cwd=baseline, timeout=120)


def _protected_changes(workspace: Path, language: str) -> list[str]:
    result = _run(["git", "status", "--short"], cwd=workspace, timeout=30)
    if result["return_code"] != 0:
        return ["git_status_failed"]
    allowed = f"benchmarks/baselines/{language}/"
    protected: list[str] = []
    for line in result["stdout"].splitlines():
        path = line[3:]
        if not path.startswith(allowed):
            protected.append(path)
    return protected


def run_agent_trial(
    language: str,
    task: str,
    trial_id: str,
    evidence_root: Path,
    workspace_root: Path,
) -> Path:
    workspace = workspace_root / f"{language}-{task.lower()}-{trial_id}"
    build_task_start(language, task, workspace)
    start_records = _tree_records(workspace)
    start_digest = _tree_digest(start_records)
    _copy_tool_environment(workspace, language)
    _git_initialize(workspace)
    schema_path = _write_change_schema(workspace, language)

    command = [
        str(CODEX_CLI),
        "exec",
        "--json",
        "--ignore-user-config",
        "--ignore-rules",
        "--skip-git-repo-check",
        "--ephemeral",
        "--output-schema",
        str(schema_path),
        "-m",
        MODEL,
        "-c",
        f"model_reasoning_effort='{MODEL_EFFORT}'",
        "-c",
        "sandbox_workspace_write.network_access=false",
        "-s",
        "workspace-write",
        "-C",
        str(workspace),
        _agent_prompt(workspace, language, task),
    ]
    agent = _run(command, cwd=workspace, timeout=AGENT_SECONDS + 15)
    parsed = _parse_agent_events(agent["stdout"])
    generated_edits, change_error = _apply_agent_changes(
        workspace, language, agent["stdout"]
    )
    schema_path.unlink(missing_ok=True)
    baseline = workspace / "benchmarks" / "baselines" / language
    formatting = _format_workspace(baseline, language)
    checks = [
        _run(command, cwd=baseline, timeout=300)
        for command in _verification_commands(language, task)
    ]
    protected = _protected_changes(workspace, language)
    source_records = [
        record
        for record in _tree_records(workspace)
        if not record["path"].startswith(".git/")
        and "/.venv/" not in f"/{record['path']}/"
        and "/node_modules/" not in f"/{record['path']}/"
    ]
    classifications: list[str] = []
    if agent["timed_out"]:
        classifications.append("timeout")
    elif agent["return_code"] != 0:
        classifications.append("agent_nonzero_exit")
    if change_error:
        classifications.append(change_error)
    if formatting["return_code"] != 0:
        classifications.append("formatting_failed")
    if (parsed["input_tokens"] or 0) > INPUT_TOKEN_LIMIT:
        classifications.append("input_token_limit")
    if protected:
        classifications.append("permission_violation")
    if any(check["return_code"] != 0 for check in checks):
        classifications.append("final_checks_failed")
    success = not classifications

    raw_directory = evidence_root / "raw" / language / task.lower()
    raw_directory.mkdir(parents=True, exist_ok=True)
    raw_path = raw_directory / f"{trial_id}.jsonl.gz"
    with gzip.open(raw_path, "wt", encoding="utf-8") as stream:
        stream.write(agent["stdout"])
    record = {
        "agent_trial_format": 1,
        "trial_id": trial_id,
        "configuration": {"language": language, "task": task},
        "task_start_sha256": start_digest,
        "model": {
            "provider": "openai",
            "name": MODEL,
            "reasoning_effort": MODEL_EFFORT,
            "agent": f"codex-cli 0.145.0-alpha.18",
        },
        "environment": {
            "reference_host": platform.node(),
            "operating_system": platform.platform(),
            "architecture": platform.machine(),
            "container_image": "native-macos-not-containerized",
            "network": "denied-to-agent-tools",
        },
        "limits": {
            "run_seconds": AGENT_SECONDS,
            "input_tokens": INPUT_TOKEN_LIMIT,
        },
        "agent_result": {
            "return_code": agent["return_code"],
            "timed_out": agent["timed_out"],
            "elapsed_ms": agent["elapsed_ms"],
            **parsed,
            "generated_file_edits": generated_edits,
        },
        "changes": {
            "final_source_tree_sha256": _tree_digest(source_records),
            "protected_changes": protected,
        },
        "checks": [
            {
                "command": formatting["command"],
                "return_code": formatting["return_code"],
                "timed_out": formatting["timed_out"],
                "elapsed_ms": formatting["elapsed_ms"],
                "stdout": formatting["stdout"][-4000:],
                "stderr": formatting["stderr"][-4000:],
                "phase": "automatic_formatter",
            },
        ]
        + [
            {
                "command": check["command"],
                "return_code": check["return_code"],
                "timed_out": check["timed_out"],
                "elapsed_ms": check["elapsed_ms"],
                "stdout": check["stdout"][-4000:],
                "stderr": check["stderr"][-4000:],
                "phase": "verification",
            }
            for check in checks
        ],
        "classification": "successful" if success else classifications[0],
        "all_classifications": classifications,
        "successful": success,
        "raw_events": {
            "path": raw_path.relative_to(ROOT).as_posix()
            if raw_path.is_relative_to(ROOT)
            else str(raw_path),
            "sha256": _sha256(raw_path),
        },
        "attempted_external_access": "Could not connect" in agent["stdout"],
    }
    record_directory = evidence_root / "agent" / language / task.lower()
    record_directory.mkdir(parents=True, exist_ok=True)
    record_path = record_directory / f"{trial_id}.json"
    record_path.write_text(_canonical(record), encoding="utf-8")
    print(
        f"{language}/{task}/{trial_id}: "
        f"{'successful' if success else ','.join(classifications)}"
    )
    return record_path


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    starts = subparsers.add_parser("task-starts")
    mode = starts.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--write-lock", action="store_true")
    build = subparsers.add_parser("build-task-start")
    build.add_argument("--language", required=True, choices=LANGUAGES)
    build.add_argument("--task", required=True, choices=TASKS)
    build.add_argument("--output", type=Path, required=True)
    trial = subparsers.add_parser("run-agent-trial")
    trial.add_argument("--language", required=True, choices=LANGUAGES)
    trial.add_argument("--task", required=True, choices=TASKS)
    trial.add_argument("--trial-id", required=True)
    trial.add_argument("--evidence-root", type=Path, required=True)
    trial.add_argument("--workspace-root", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        if args.command == "run-agent-trial":
            run_agent_trial(
                args.language,
                args.task,
                args.trial_id,
                args.evidence_root.resolve(),
                args.workspace_root.resolve(),
            )
        elif args.command == "build-task-start":
            build_task_start(args.language, args.task, args.output.resolve())
            records = _tree_records(args.output.resolve())
            print(
                f"Built {args.language}/{args.task}: "
                f"{len(records)} files, {_tree_digest(records)}."
            )
        elif args.write_lock:
            write_task_start_lock()
        else:
            check_task_start_lock()
    except CalibrationError as error:
        print(f"ERROR [{error.code}]: {error.message}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
