#!/usr/bin/env python3
"""Prepare and dry-test the locked M8 interactive agent trial workflow."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import secrets
import signal
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, NoReturn, Protocol

import task_starts as task_start_tool
from responses_recorder import LoopbackResponsesRecorder, RecorderFailure


ROOT = Path(__file__).resolve().parents[2]
CALIBRATION_ROOT = ROOT / "benchmarks" / "calibration"
EXPERIMENT_CONTRACT = CALIBRATION_ROOT / "experiment-contract.json"
TASK_START_LOCK = ROOT / "benchmarks" / "task-starts" / "task-starts.json"
M7_PARITY_REPORT = ROOT / "benchmarks" / "m7-parity-report.json"
CONFIGURATION_ID = "m8-agent-experiment-v1"
TOKEN_CATEGORIES = (
    "initial_context",
    "source_reads",
    "semantic_tool_output",
    "diagnostics",
    "build_and_test_output",
    "other_tool_output",
)
TERMINAL_CAUSES = (
    "complete",
    "nonzero_exit",
    "timed_out",
    "permission_violation",
    "input_token_limit",
    "incomplete_evidence",
)
PROMPT_PREFIX = (
    "Complete the frozen task below in the supplied answer-free workspace.\n"
    "\n"
    "Work as one coding agent using the available local tools. Inspect the workspace,\n"
    "make the required source changes, and run the relevant visible formatter,\n"
    "static-analysis, build, and test checks. You may iterate after a failed visible\n"
    "check. Do not delegate to another agent, use the network, install dependencies,\n"
    "read or write outside the workspace, or change protected task, test, fixture,\n"
    "contract, or tool-configuration files. There is no human follow-up during the\n"
    "trial. Stop when the task is complete or a locked limit prevents further work.\n"
    "In the final message, summarize the source changes and checks run; do not claim\n"
    "that unavailable hidden checks passed.\n"
    "\n"
    "--- BEGIN FROZEN TASK ---\n"
)
PROMPT_SUFFIX = "--- END FROZEN TASK ---\n"


@dataclass(frozen=True)
class RunnerError(Exception):
    code: str
    message: str

    def __str__(self) -> str:
        return f"{self.code}: {self.message}"


@dataclass(frozen=True)
class PreparedTrial:
    language: str
    task: str
    workspace: Path
    prompt: bytes
    task_start_tree_sha256: str
    permission_profile: dict[str, Any]
    permission_profile_sha256: str


@dataclass(frozen=True)
class RuntimeGate:
    workspace: Path
    observations_sha256: str


@dataclass(frozen=True)
class StreamResult:
    terminal_class: str
    terminal_cause: str
    events: tuple[dict[str, Any], ...]
    token_accounting: dict[str, Any]
    activity: dict[str, int]
    permissions: dict[str, Any]
    exit_status: int | None
    process_actions: tuple[str, ...]


class ProcessGroup(Protocol):
    def terminate(self) -> None: ...

    def wait(self, timeout: float) -> int: ...

    def kill(self) -> None: ...


class DryProcessGroup:
    """Record process-group actions without starting a process."""

    def __init__(self, *, exit_status: int = 0, exits_on_terminate: bool = True):
        self.exit_status = exit_status
        self.exits_on_terminate = exits_on_terminate
        self.killed = False
        self.actions: list[str] = []

    def terminate(self) -> None:
        self.actions.append("SIGTERM")

    def wait(self, timeout: float) -> int:
        self.actions.append(f"wait:{timeout:g}")
        if (
            self.actions
            and self.actions[0] == "SIGTERM"
            and not self.exits_on_terminate
            and not self.killed
        ):
            raise subprocess.TimeoutExpired("dry-process-group", timeout)
        return self.exit_status

    def kill(self) -> None:
        self.actions.append("SIGKILL")
        self.killed = True


class SpawnedProcessGroup:
    """Control a real trial subprocess as one POSIX process group."""

    def __init__(self, process: subprocess.Popen[bytes]):
        self.process = process

    def terminate(self) -> None:
        os.killpg(self.process.pid, signal.SIGTERM)

    def wait(self, timeout: float) -> int:
        return self.process.wait(timeout=timeout)

    def kill(self) -> None:
        os.killpg(self.process.pid, signal.SIGKILL)


def _raise(code: str, message: str) -> NoReturn:
    raise RunnerError(code, message)


def _load_object(path: Path, code: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        _raise(code, f"{path}: {error}")
    if not isinstance(value, dict):
        _raise(code, f"{path}: expected JSON object")
    return value


def _canonical_payload(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _sha256(path: Path) -> str:
    return _sha256_bytes(path.read_bytes())


def _configuration(language: str, task: str, lock: dict[str, Any]) -> dict[str, Any]:
    for configuration in lock.get("configurations", ()):
        if (
            isinstance(configuration, dict)
            and configuration.get("language") == language
            and configuration.get("task") == task
        ):
            return configuration
    _raise("runner_configuration_invalid", f"unknown configuration {language}/{task}")


def _render_prompt(task_path: Path) -> bytes:
    task = task_path.read_bytes()
    try:
        task.decode("utf-8")
    except UnicodeDecodeError as error:
        _raise("runner_prompt_invalid", str(error))
    if b"\r" in task or not task.endswith(b"\n"):
        _raise("runner_prompt_invalid", "TASK.md must use LF and end with LF")
    return PROMPT_PREFIX.encode() + task + PROMPT_SUFFIX.encode()


def _profile(configuration: dict[str, Any]) -> dict[str, Any]:
    editable = sorted(
        entry["path"] for entry in configuration["files"] if entry["role"] == "editable"
    )
    protected = sorted(
        entry["path"]
        for entry in configuration["files"]
        if entry["role"] == "protected"
    )
    generated_by_language = {
        "rust": ["benchmarks/baselines/rust/target"],
        "go": [".trial-cache/go-build"],
        "python": [
            ".trial-cache/mypy",
            ".trial-cache/pytest",
            ".trial-cache/ruff",
        ],
        "typescript": [
            ".trial-cache/npm",
            "benchmarks/baselines/typescript/node_modules",
        ],
    }
    source_roots_by_language = {
        "rust": ["benchmarks/baselines/rust/v2/src"],
        "go": [
            "benchmarks/baselines/go/v2/cmd/runner",
            "benchmarks/baselines/go/v2/domain",
            "benchmarks/baselines/go/v2/fixture",
            "benchmarks/baselines/go/v2/service",
            "benchmarks/baselines/go/v2/store",
        ],
        "python": ["benchmarks/baselines/python/v2"],
        "typescript": ["benchmarks/baselines/typescript/v2"],
    }
    source_roots = (
        source_roots_by_language[configuration["language"]]
        if configuration["task"] == "UC-003"
        else []
    )
    return {
        "permission_profile_format": 1,
        "workspace_read": "all",
        "editable_files": editable,
        "workspace_generated_write_roots": [
            ".git",
            ".trial-cache",
            ".trial-home",
            ".trial-tmp",
            "build",
            "coverage",
            "dist",
            "out",
            "target",
            *generated_by_language[configuration["language"]],
        ],
        "workspace_source_write_roots": source_roots,
        "protected_files": protected,
        "parent_repository_read": "denied",
        "private_package_read": "denied",
        "evidence_read": "denied",
        "subprocess_network": "denied",
        "control_plane": "loopback-recorder-only",
        "host_environment": "allowlist-only",
    }


def _check_contract_match(
    language: str,
    task: str,
    configuration: dict[str, Any],
    prompt: bytes,
    contract: dict[str, Any],
) -> None:
    if contract.get("contract_id") != CONFIGURATION_ID:
        _raise("runner_manifest_gate_failed", "experiment contract ID differs")
    config_id = f"{language}-{task.lower().replace('-', '')}"
    start = next(
        (
            entry
            for entry in contract.get("task_starts", ())
            if entry.get("configuration_id") == config_id
        ),
        None,
    )
    if start is None or start.get("tree_sha256") != configuration["tree_sha256"]:
        _raise("runner_manifest_gate_failed", "task-start lock differs")
    rendered = next(
        (
            entry
            for entry in contract.get("prompt", {}).get("rendered", ())
            if entry.get("task") == task
        ),
        None,
    )
    if rendered is None or rendered.get("rendered_prompt_sha256") != _sha256_bytes(
        prompt
    ):
        _raise("runner_manifest_gate_failed", "rendered prompt digest differs")
    expected_command = [
        "codex",
        "exec",
        "--json",
        "--ephemeral",
        "--strict-config",
        "--color",
        "never",
        "-C",
        "{workspace}",
        "-m",
        "gpt-5.6-sol",
        "-",
    ]
    if contract.get("protocol", {}).get("command") != expected_command:
        _raise("runner_manifest_gate_failed", "agent command differs")


def prepare_trial(
    language: str,
    task: str,
    destination: Path,
    *,
    run_starting_state: bool = False,
) -> PreparedTrial:
    """Build a fresh task workspace and complete the pre-start manifest gate."""

    contract = _load_object(EXPERIMENT_CONTRACT, "runner_manifest_gate_failed")
    lock = _load_object(TASK_START_LOCK, "runner_manifest_gate_failed")
    configuration = _configuration(language, task, lock)
    destination = destination.resolve()
    if destination.exists() and any(destination.iterdir()):
        _raise("runner_workspace_not_fresh", f"{destination} is not empty")
    task_start_tool.build_task_start(language, task, destination)
    task_start_tool.verify_locked_workspace(destination, configuration)
    if run_starting_state:
        task_start_tool._run_starting_state(language, task, destination, configuration)
    else:
        completed = subprocess.run(
            ["git", "init", "--quiet"],
            cwd=destination,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        if completed.returncode != 0:
            _raise("runner_materialization_failed", "local Git initialization failed")
    prompt = _render_prompt(destination / configuration["task_path"])
    _check_contract_match(language, task, configuration, prompt, contract)
    profile = _profile(configuration)
    return PreparedTrial(
        language=language,
        task=task,
        workspace=destination,
        prompt=prompt,
        task_start_tree_sha256=configuration["tree_sha256"],
        permission_profile=profile,
        permission_profile_sha256=_sha256_bytes(_canonical_payload(profile)),
    )


def expected_prestart_observations(prepared: PreparedTrial) -> dict[str, Any]:
    """Return the exact observation shape that real probes must populate."""

    contract = _load_object(EXPERIMENT_CONTRACT, "runner_manifest_gate_failed")
    parity = _load_object(M7_PARITY_REPORT, "runner_manifest_gate_failed")
    return {
        "configuration_sha256": _sha256(EXPERIMENT_CONTRACT),
        "task_start_tree_sha256": prepared.task_start_tree_sha256,
        "rendered_prompt_sha256": _sha256_bytes(prepared.prompt),
        "agent_version": contract["agent"]["agent_version"],
        "agent_sha256": contract["agent"]["agent_sha256"],
        "model_request": contract["agent"]["model_request"],
        "reasoning_effort": contract["agent"]["reasoning_effort"],
        "normal_tools": contract["tools"][prepared.language],
        "private_package_sha256": parity["hidden_package_sha256"],
        "recorder": "ready",
        "tokenizer": "ready",
        "permission_profile_sha256": prepared.permission_profile_sha256,
        "permissions_enforced": "yes",
        "network_denial_enforced": "yes",
        "evidence_destination_isolated": "yes",
        "starting_state": "locked-expected-result-observed",
    }


def verify_prestart_observations(
    prepared: PreparedTrial, observations: dict[str, Any]
) -> RuntimeGate:
    """Reject any missing, extra, or changed pre-start observation."""

    expected = expected_prestart_observations(prepared)
    if observations != expected:
        missing = sorted(set(expected) - set(observations))
        extra = sorted(set(observations) - set(expected))
        changed = sorted(
            key
            for key in set(expected) & set(observations)
            if expected[key] != observations[key]
        )
        _raise(
            "runner_manifest_gate_failed",
            f"pre-start observations differ; missing={missing}, "
            f"extra={extra}, changed={changed}",
        )
    return RuntimeGate(
        workspace=prepared.workspace,
        observations_sha256=_sha256_bytes(_canonical_payload(observations)),
    )


def check_write_permission(prepared: PreparedTrial, raw_path: str) -> bool:
    """Decide a proposed durable write without following paths outside the workspace."""

    path = Path(raw_path)
    if path.is_absolute() or ".." in path.parts:
        return False
    normalized = path.as_posix()
    candidate = prepared.workspace / path
    for parent in (candidate, *candidate.parents):
        if parent == prepared.workspace.parent:
            break
        if parent.is_symlink():
            return False
    profile = prepared.permission_profile
    if normalized in profile["editable_files"]:
        return True
    return any(
        normalized == root or normalized.startswith(root + "/")
        for root in (
            *profile["workspace_generated_write_roots"],
            *profile["workspace_source_write_roots"],
        )
    )


def check_read_permission(prepared: PreparedTrial, raw_path: str) -> bool:
    path = Path(raw_path)
    if path.is_absolute() or ".." in path.parts:
        return False
    resolved = (prepared.workspace / path).resolve()
    try:
        resolved.relative_to(prepared.workspace)
    except ValueError:
        return False
    return not resolved.is_symlink()


def apply_recorded_edit(
    prepared: PreparedTrial,
    stream: InteractiveStream,
    raw_path: str,
    contents: bytes,
) -> bool:
    """Apply one authorized edit and emit permission/edit evidence around it."""

    allowed = check_write_permission(prepared, raw_path)
    stream.permission("write", raw_path, allowed=allowed)
    if not allowed:
        return False
    path = prepared.workspace / raw_path
    before = _sha256(path) if path.is_file() else _sha256_bytes(b"")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(contents)
    stream.edit(raw_path, before, _sha256(path))
    return True


def environment_allowlist(prepared: PreparedTrial, tool_path: str) -> dict[str, str]:
    """Return the complete secret-free subprocess environment for a trial."""

    workspace = prepared.workspace
    return {
        "CARGO_NET_OFFLINE": "true",
        "GOTOOLCHAIN": "local",
        "HOME": str(workspace / ".trial-home"),
        "LANG": "C",
        "LC_ALL": "C",
        "NO_COLOR": "1",
        "PATH": tool_path,
        "PIP_NO_INDEX": "1",
        "PYTHONHASHSEED": "0",
        "TMPDIR": str(workspace / ".trial-tmp"),
        "TZ": "UTC",
    }


def _toml_string(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def materialize_codex_config(
    prepared: PreparedTrial,
    codex_home: Path,
    *,
    recorder_url: str,
    tool_path: str,
) -> str:
    """Write the isolated user-level Codex config and return its SHA-256."""

    if not (
        recorder_url.startswith("http://127.0.0.1:")
        or recorder_url.startswith("http://localhost:")
    ):
        _raise("runner_configuration_invalid", "recorder must use loopback HTTP")
    codex_home.mkdir(parents=True, exist_ok=True)
    config_path = codex_home / "config.toml"
    if config_path.exists():
        _raise("runner_configuration_invalid", "isolated config already exists")
    profile = prepared.permission_profile
    filesystem_lines = ['"." = "read"']
    for path in profile["editable_files"]:
        filesystem_lines.append(f'{_toml_string(path)} = "write"')
    for path in profile["workspace_generated_write_roots"]:
        filesystem_lines.append(f'{_toml_string(path + "/**")} = "write"')
    for path in profile["workspace_source_write_roots"]:
        filesystem_lines.append(f'{_toml_string(path + "/**")} = "write"')
    environment = environment_allowlist(prepared, tool_path)
    environment_lines = [
        f"{key} = {_toml_string(value)}" for key, value in sorted(environment.items())
    ]
    config = "\n".join(
        [
            'model = "gpt-5.6-sol"',
            'model_provider = "m8_recorder"',
            'model_reasoning_effort = "high"',
            'model_reasoning_summary = "none"',
            'model_verbosity = "medium"',
            'personality = "none"',
            'approval_policy = "never"',
            'default_permissions = "m8_trial"',
            'web_search = "disabled"',
            "check_for_update_on_startup = false",
            "allow_login_shell = false",
            "",
            "[history]",
            'persistence = "none"',
            "",
            "[analytics]",
            "enabled = false",
            "",
            "[feedback]",
            "enabled = false",
            "",
            "[features]",
            "apps = false",
            "hooks = false",
            "memories = false",
            "multi_agent = false",
            "remote_plugin = false",
            "skill_mcp_dependency_install = false",
            "",
            "[agents]",
            "max_threads = 1",
            "max_depth = 1",
            "",
            "[shell_environment_policy]",
            'inherit = "none"',
            "ignore_default_excludes = false",
            "experimental_use_profile = false",
            "",
            "[shell_environment_policy.set]",
            *environment_lines,
            "",
            "[model_providers.m8_recorder]",
            'name = "M8 loopback recorder"',
            f"base_url = {_toml_string(recorder_url)}",
            'wire_api = "responses"',
            'env_key = "AIL_RECORDER_CLIENT_TOKEN"',
            "request_max_retries = 0",
            "stream_max_retries = 0",
            "stream_idle_timeout_ms = 300000",
            "supports_websockets = false",
            "",
            "[permissions.m8_trial]",
            'description = "Locked M8 agent trial workspace"',
            'extends = ":read-only"',
            "",
            '[permissions.m8_trial.filesystem.":workspace_roots"]',
            *filesystem_lines,
            "",
            "[permissions.m8_trial.network]",
            "enabled = false",
            "",
        ]
    )
    config_path.write_text(config, encoding="utf-8")
    try:
        tomllib.loads(config)
    except tomllib.TOMLDecodeError as error:
        _raise("runner_configuration_invalid", f"generated Codex config: {error}")
    return _sha256(config_path)


class EventRecorder:
    def __init__(self, trial_id: str, clock: Callable[[], int]):
        self.trial_id = trial_id
        self.clock = clock
        self.events: list[dict[str, Any]] = []

    def record(self, kind: str, phase: str, payload: dict[str, Any]) -> None:
        sequence = len(self.events) + 1
        self.events.append(
            {
                "event_format": 1,
                "sequence": sequence,
                "event_id": f"{self.trial_id}.event-{sequence}",
                "trial_id": self.trial_id,
                "configuration_id": CONFIGURATION_ID,
                "phase": phase,
                "kind": kind,
                "monotonic_ns": self.clock(),
                "payload": payload,
                "payload_sha256": _sha256_bytes(_canonical_payload(payload)),
            }
        )

    def write_jsonl(self, destination: Path) -> str:
        destination.parent.mkdir(parents=True, exist_ok=True)
        content = b"".join(_canonical_payload(event) + b"\n" for event in self.events)
        destination.write_bytes(content)
        return _sha256(destination)


class InteractiveStream:
    """Derive the M8c record from recorder/sandbox events."""

    def __init__(
        self,
        trial_id: str,
        *,
        wall_limit_seconds: int = 600,
        token_limit: int = 500_000,
        termination_grace_seconds: int = 5,
        clock: Callable[[], int] = time.monotonic_ns,
        process: ProcessGroup | None = None,
    ):
        self.clock = clock
        self.recorder = EventRecorder(trial_id, clock)
        self.wall_limit_ns = wall_limit_seconds * 1_000_000_000
        self.token_limit = token_limit
        self.termination_grace_seconds = termination_grace_seconds
        self.process = process or DryProcessGroup()
        self.started_ns: int | None = None
        self.requests: list[dict[str, Any]] = []
        self.categories = {category: 0 for category in TOKEN_CATEGORIES}
        self.total_input = 0
        self.total_cached = 0
        self.edits = 0
        self.validations = 0
        self.incomplete_validations = 0
        self.repairs = 0
        self.edit_since_validation = False
        self.permission_violations = 0
        self.external_access_attempts = 0
        self.exit_status: int | None = None
        self.cause: str | None = None
        self.open_tools: set[str] = set()
        self.open_requests: set[str] = set()

    def start(self) -> None:
        if self.started_ns is not None:
            _raise("runner_stream_invalid", "trial already started")
        self.started_ns = self.clock()
        self.recorder.record("trial.started", "start", {})

    def _require_active(self) -> None:
        if self.started_ns is None or self.cause is not None:
            _raise("runner_stream_invalid", "trial is not active")
        if self.clock() - self.started_ns > self.wall_limit_ns:
            self.stop_for_limit("timed_out")
            _raise("runner_stream_stopped", "wall limit reached")

    def model_request(
        self,
        request_id: str,
        *,
        preflight_input_tokens: int,
        provider_input_tokens: int,
        cached_input_tokens: int,
        protocol_overhead_tokens: int,
        categories: dict[str, int],
        body: dict[str, Any],
    ) -> bool:
        self._require_active()
        if (
            tuple(categories) != TOKEN_CATEGORIES
            or any(
                not isinstance(value, int) or isinstance(value, bool) or value < 0
                for value in categories.values()
            )
            or cached_input_tokens < 0
            or protocol_overhead_tokens < 0
        ):
            self.stop_for_limit("incomplete_evidence")
            return False
        category_total = sum(categories.values())
        if (
            preflight_input_tokens != provider_input_tokens
            or category_total + protocol_overhead_tokens != provider_input_tokens
            or cached_input_tokens > provider_input_tokens
        ):
            self.stop_for_limit("incomplete_evidence")
            return False
        if self.total_input + preflight_input_tokens > self.token_limit:
            self.stop_for_limit("input_token_limit")
            return False
        request = {
            "request_id": request_id,
            "preflight_input_tokens": preflight_input_tokens,
            "provider_input_tokens": provider_input_tokens,
            "cached_input_tokens": cached_input_tokens,
            "protocol_overhead_tokens": protocol_overhead_tokens,
            "categories": dict(categories),
        }
        self.requests.append(request)
        self.open_requests.add(request_id)
        self.total_input += provider_input_tokens
        self.total_cached += cached_input_tokens
        for category, value in categories.items():
            self.categories[category] += value
        self.recorder.record(
            "model.request",
            "agent",
            {
                "request_id": request_id,
                "body": body,
                "input": request,
                "redaction": "authorization-and-cookies-removed",
            },
        )
        return True

    def model_response(self, request_id: str, payload: dict[str, Any]) -> None:
        self._require_active()
        if request_id not in self.open_requests:
            self.stop_for_limit("incomplete_evidence")
            return
        self.open_requests.remove(request_id)
        self.recorder.record(
            "model.response",
            "agent",
            {"request_id": request_id, "response": payload},
        )

    def tool_call(self, call_id: str, payload: dict[str, Any]) -> None:
        self._require_active()
        if call_id in self.open_tools:
            self.stop_for_limit("incomplete_evidence")
            return
        self.open_tools.add(call_id)
        self.recorder.record("tool.call", "agent", {"call_id": call_id, **payload})

    def tool_result(self, call_id: str, payload: dict[str, Any]) -> None:
        self._require_active()
        if call_id not in self.open_tools:
            self.stop_for_limit("incomplete_evidence")
            return
        self.open_tools.remove(call_id)
        self.recorder.record("tool.result", "agent", {"call_id": call_id, **payload})

    def edit(
        self, path: str, before_sha256: str, after_sha256: str, *, durable: bool = True
    ) -> None:
        self._require_active()
        if durable and before_sha256 != after_sha256:
            self.edits += 1
            self.edit_since_validation = True
        self.recorder.record(
            "workspace.edit",
            "agent",
            {
                "path": path,
                "before_sha256": before_sha256,
                "after_sha256": after_sha256,
                "durable": durable,
            },
        )

    def validation(self, action: str, *, complete: bool, exit_status: int) -> None:
        self._require_active()
        self.validations += 1
        if not complete:
            self.incomplete_validations += 1
            if self.edit_since_validation:
                self.repairs += 1
        self.edit_since_validation = False
        self.recorder.record(
            "validation.result",
            "agent",
            {
                "action": action,
                "complete": complete,
                "exit_status": exit_status,
            },
        )

    def permission(
        self,
        operation: str,
        target: str,
        *,
        allowed: bool,
        external_access: bool = False,
    ) -> None:
        self._require_active()
        if external_access:
            self.external_access_attempts += 1
        if not allowed:
            self.permission_violations += 1
        self.recorder.record(
            "permission.result",
            "agent",
            {
                "operation": operation,
                "target": target,
                "allowed": allowed,
                "external_access": external_access,
            },
        )
        if not allowed:
            self.stop_for_limit("permission_violation")

    def process_result(self, exit_status: int) -> None:
        self._require_active()
        self.exit_status = exit_status
        self.recorder.record("process.result", "stop", {"exit_status": exit_status})

    def _terminate(self) -> None:
        self.process.terminate()
        try:
            self.process.wait(float(self.termination_grace_seconds))
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(float(self.termination_grace_seconds))

    def stop_for_limit(self, cause: str) -> None:
        if cause not in TERMINAL_CAUSES[2:]:
            _raise("runner_stream_invalid", f"invalid enforced cause {cause}")
        if self.cause is not None:
            return
        self.cause = cause
        self.recorder.record("process.result", "stop", {"enforced_stop": cause})
        self._terminate()

    def finish(self) -> StreamResult:
        if self.started_ns is None:
            _raise("runner_stream_invalid", "trial never started")
        if self.cause is None:
            if self.clock() - self.started_ns > self.wall_limit_ns:
                self.stop_for_limit("timed_out")
            elif self.open_tools or self.open_requests or not self.requests:
                self.stop_for_limit("incomplete_evidence")
            elif self.exit_status is None:
                self.stop_for_limit("incomplete_evidence")
            elif self.exit_status != 0:
                self.cause = "nonzero_exit"
            else:
                self.cause = "complete"
        assert self.cause is not None
        terminal_class = (
            "timed_out"
            if self.cause == "timed_out"
            else "successful"
            if self.cause == "complete"
            else "failed"
        )
        self.recorder.record(
            "trial.stopped",
            "stop",
            {"class": terminal_class, "cause": self.cause},
        )
        actions = tuple(getattr(self.process, "actions", ()))
        return StreamResult(
            terminal_class=terminal_class,
            terminal_cause=self.cause,
            events=tuple(self.recorder.events),
            token_accounting={
                "requests": self.requests,
                "total_input_tokens": self.total_input,
                "cached_input_tokens": self.total_cached,
                "categories": self.categories,
            },
            activity={
                "edits": self.edits,
                "validation_attempts": self.validations,
                "incomplete_validations": self.incomplete_validations,
                "repair_cycles": self.repairs,
            },
            permissions={
                "violation_count": self.permission_violations,
                "external_access_attempts": self.external_access_attempts,
            },
            exit_status=self.exit_status,
            process_actions=actions,
        )


def spawn_agent(
    prepared: PreparedTrial,
    *,
    codex_path: Path,
    codex_home: Path,
    recorder_url: str,
    recorder_client_token: str,
    tool_path: str,
    prestart_observations: dict[str, Any],
) -> tuple[subprocess.Popen[bytes], SpawnedProcessGroup]:
    """Spawn the locked Codex command; callers must wire and retain its JSONL."""

    verify_prestart_observations(prepared, prestart_observations)
    materialize_codex_config(
        prepared,
        codex_home,
        recorder_url=recorder_url,
        tool_path=tool_path,
    )
    command = [
        str(codex_path),
        "exec",
        "--json",
        "--ephemeral",
        "--strict-config",
        "--color",
        "never",
        "-C",
        str(prepared.workspace),
        "-m",
        "gpt-5.6-sol",
        "-",
    ]
    environment = environment_allowlist(prepared, tool_path)
    environment.update(
        {
            "CODEX_HOME": str(codex_home),
            "AIL_RESPONSES_RECORDER": recorder_url,
            "AIL_RECORDER_CLIENT_TOKEN": recorder_client_token,
        }
    )
    process = subprocess.Popen(
        command,
        cwd=prepared.workspace,
        env=environment,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=True,
    )
    assert process.stdin is not None
    process.stdin.write(prepared.prompt)
    process.stdin.close()
    process.stdin = None
    return process, SpawnedProcessGroup(process)


def _source_records(workspace: Path) -> list[dict[str, str]]:
    ignored_names = {
        ".git",
        ".mypy_cache",
        ".pytest_cache",
        ".ruff_cache",
        ".trial-cache",
        ".trial-home",
        ".trial-tmp",
        "__pycache__",
        "build",
        "coverage",
        "dist",
        "node_modules",
        "out",
        "target",
    }
    records: list[dict[str, str]] = []
    for path in sorted(workspace.rglob("*")):
        relative = path.relative_to(workspace)
        if any(part in ignored_names for part in relative.parts):
            continue
        if path.is_symlink():
            _raise("runner_final_source_invalid", f"symlink at {relative.as_posix()}")
        if path.is_file():
            records.append({"path": relative.as_posix(), "sha256": _sha256(path)})
    return records


def capture_final_source(workspace: Path, destination: Path) -> tuple[str, str]:
    """Write a deterministic, symlink-free ZIP and return tree/archive digests."""

    records = _source_records(workspace)
    tree_sha256 = task_start_tool._tree_digest(records)
    destination.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_STORED) as archive:
        for record in records:
            source = workspace / record["path"]
            info = zipfile.ZipInfo(record["path"], date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            archive.writestr(info, source.read_bytes())
        manifest = (
            json.dumps(
                {"tree_sha256": tree_sha256, "files": records},
                ensure_ascii=False,
                indent=2,
            )
            + "\n"
        ).encode("utf-8")
        info = zipfile.ZipInfo(
            ".ail-final-source-manifest.json", date_time=(1980, 1, 1, 0, 0, 0)
        )
        info.create_system = 3
        info.external_attr = 0o100644 << 16
        archive.writestr(info, manifest)
    return tree_sha256, _sha256(destination)


def _request_tokens(total: int = 12) -> dict[str, Any]:
    categories = {category: 0 for category in TOKEN_CATEGORIES}
    categories["initial_context"] = total - 2
    return {
        "preflight_input_tokens": total,
        "provider_input_tokens": total,
        "cached_input_tokens": 0,
        "protocol_overhead_tokens": 2,
        "categories": categories,
        "body": {
            "model": "gpt-5.6-sol",
            "input": [{"role": "user", "content": "dry"}],
            "tools": [],
        },
    }


def verify_fake_and_dry_streams() -> list[tuple[str, str]]:
    """Exercise the M8c terminal and accounting matrix without a model call."""

    outcomes: list[tuple[str, str]] = []

    def fresh(
        *,
        clock: Callable[[], int] = lambda: 1_000_000_000,
        process: ProcessGroup | None = None,
        token_limit: int = 500_000,
    ) -> InteractiveStream:
        stream = InteractiveStream(
            "dry.m8c",
            clock=clock,
            process=process,
            token_limit=token_limit,
        )
        stream.start()
        return stream

    success = fresh()
    success.model_request("request-1", **_request_tokens())
    success.model_response("request-1", {"type": "response.completed"})
    success.edit("source", "0" * 64, "1" * 64)
    success.validation("tests", complete=False, exit_status=1)
    success.validation("tests", complete=False, exit_status=1)
    success.edit("source", "1" * 64, "2" * 64)
    success.validation("tests", complete=True, exit_status=0)
    success.process_result(0)
    result = success.finish()
    if result.activity != {
        "edits": 2,
        "validation_attempts": 3,
        "incomplete_validations": 2,
        "repair_cycles": 1,
    }:
        _raise("runner_self_test_failed", "M2 activity accounting differs")
    outcomes.append(("success", result.terminal_cause))

    failure = fresh()
    failure.model_request("request-1", **_request_tokens())
    failure.model_response("request-1", {})
    failure.process_result(7)
    outcomes.append(("failure", failure.finish().terminal_cause))

    ticks = iter((0, *([601_000_000_001] * 12)))
    timed = fresh(
        clock=lambda: next(ticks),
        process=DryProcessGroup(exits_on_terminate=False),
    )
    outcomes.append(("timeout", timed.finish().terminal_cause))

    denied = fresh()
    denied.permission(
        "network", "example.invalid:443", allowed=False, external_access=True
    )
    outcomes.append(("permission", denied.finish().terminal_cause))

    limited = fresh(token_limit=10)
    limited.model_request("request-1", **_request_tokens(12))
    outcomes.append(("token-limit", limited.finish().terminal_cause))

    incomplete = fresh()
    incomplete.model_request("request-1", **_request_tokens())
    outcomes.append(("incomplete-evidence", incomplete.finish().terminal_cause))

    expected = [
        ("success", "complete"),
        ("failure", "nonzero_exit"),
        ("timeout", "timed_out"),
        ("permission", "permission_violation"),
        ("token-limit", "input_token_limit"),
        ("incomplete-evidence", "incomplete_evidence"),
    ]
    if outcomes != expected:
        _raise("runner_self_test_failed", f"expected {expected}, received {outcomes}")

    with tempfile.TemporaryDirectory(prefix="ail-m8c-dry-") as raw:
        root = Path(raw)
        prepared = prepare_trial("python", "UC-003", root / "workspace")
        observations = expected_prestart_observations(prepared)
        verify_prestart_observations(prepared, observations)
        config_home = root / "codex-home"
        config_digest = materialize_codex_config(
            prepared,
            config_home,
            recorder_url="http://127.0.0.1:43123/v1",
            tool_path="/locked/tools",
        )
        config_text = (config_home / "config.toml").read_text(encoding="utf-8")
        if (
            len(config_digest) != 64
            or 'model_provider = "m8_recorder"' not in config_text
            or "request_max_retries = 0" not in config_text
            or "stream_max_retries = 0" not in config_text
            or "[permissions.m8_trial.network]\nenabled = false" not in config_text
            or "multi_agent = false" not in config_text
        ):
            _raise("runner_self_test_failed", "isolated Codex config differs")
        changed = dict(observations)
        changed["recorder"] = "bypassed"
        try:
            verify_prestart_observations(prepared, changed)
        except RunnerError as error:
            if error.code != "runner_manifest_gate_failed":
                raise
        else:
            _raise("runner_self_test_failed", "changed pre-start gate was accepted")
        if not check_write_permission(
            prepared, "benchmarks/baselines/python/v1/job_service.py"
        ):
            _raise("runner_self_test_failed", "editable source was denied")
        if not check_write_permission(
            prepared, "benchmarks/baselines/python/v2/domain.py"
        ):
            _raise("runner_self_test_failed", "required V2 source creation was denied")
        if check_write_permission(prepared, "TASK.md"):
            _raise("runner_self_test_failed", "protected task was writable")
        if check_write_permission(
            prepared, "benchmarks/baselines/python/tests/test_service.py"
        ):
            _raise("runner_self_test_failed", "protected test was writable")
        if check_read_permission(prepared, "../benchmarks/calibration"):
            _raise("runner_self_test_failed", "parent repository was readable")
        first = root / "first.zip"
        second = root / "second.zip"
        first_tree, first_digest = capture_final_source(prepared.workspace, first)
        second_tree, second_digest = capture_final_source(prepared.workspace, second)
        if (
            first_tree != prepared.task_start_tree_sha256
            or first_tree != second_tree
            or first_digest != second_digest
        ):
            _raise("runner_self_test_failed", "final source capture is unstable")
        events_path = root / "events.jsonl"
        events_digest = success.recorder.write_jsonl(events_path)
        if len(events_digest) != 64 or len(
            events_path.read_text(encoding="utf-8").splitlines()
        ) != len(success.recorder.events):
            _raise("runner_self_test_failed", "raw events were not retained")
    return outcomes


def _verify_codex_binary(codex_path: Path) -> tuple[str, str]:
    contract = _load_object(EXPERIMENT_CONTRACT, "runner_manifest_gate_failed")
    if not codex_path.is_file():
        _raise("runner_configuration_invalid", f"Codex binary missing: {codex_path}")
    completed = subprocess.run(
        [str(codex_path), "--version"],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=30,
    )
    version = completed.stdout.decode("utf-8", "replace").strip()
    digest = _sha256(codex_path.resolve())
    if completed.returncode != 0 or version != contract["agent"]["agent_version"]:
        _raise(
            "runner_configuration_invalid",
            f"Codex version differs: {version!r}",
        )
    if digest != contract["agent"]["agent_sha256"]:
        _raise("runner_configuration_invalid", "Codex binary digest differs")
    return version, digest


def _write_json(path: Path, value: Any) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    return _sha256(path)


def run_live_trial(
    *,
    language: str,
    task: str,
    destination: Path,
    api_key: str,
    codex_path: Path,
    tool_path: str,
    upstream_base_url: str = "https://api.openai.com/v1",
) -> dict[str, Any]:
    """Run one non-official Codex pilot through the live loopback recorder."""

    destination = destination.resolve()
    if destination.exists() and any(destination.iterdir()):
        _raise("runner_workspace_not_fresh", f"{destination} is not empty")
    destination.mkdir(parents=True, exist_ok=True)
    version, agent_digest = _verify_codex_binary(codex_path)
    trial_id = (
        f"readiness.{language}.{task.lower().replace('-', '')}."
        f"{time.time_ns()}"
    )
    workspace = destination / "workspace"
    prepared = prepare_trial(language, task, workspace, run_starting_state=True)
    client_token = secrets.token_urlsafe(32)
    limits = _load_object(EXPERIMENT_CONTRACT, "runner_manifest_gate_failed")[
        "limits"
    ]

    with LoopbackResponsesRecorder(
        upstream_base_url=upstream_base_url,
        upstream_api_key=api_key,
        client_token=client_token,
        prompt_cache_key=_sha256_bytes(trial_id.encode("utf-8")),
        token_limit=limits["cumulative_input_tokens"],
    ) as recorder:
        # A real endpoint probe turns the two readiness observations into evidence
        # before Codex is spawned.
        recorder.probe("gpt-5.6-sol")
        observations = expected_prestart_observations(prepared)
        process, process_group = spawn_agent(
            prepared,
            codex_path=codex_path,
            codex_home=destination / "codex-home",
            recorder_url=recorder.url,
            recorder_client_token=client_token,
            tool_path=tool_path,
            prestart_observations=observations,
        )
        started_ns = time.monotonic_ns()
        try:
            stdout, stderr = process.communicate(timeout=limits["agent_wall_seconds"])
        except subprocess.TimeoutExpired:
            process_group.terminate()
            try:
                stdout, stderr = process.communicate(
                    timeout=limits["termination_grace_seconds"]
                )
            except subprocess.TimeoutExpired:
                process_group.kill()
                stdout, stderr = process.communicate(
                    timeout=limits["termination_grace_seconds"]
                )
            timed_out = True
        else:
            timed_out = False
        stopped_ns = time.monotonic_ns()

    (destination / "codex.stdout.jsonl").write_bytes(stdout)
    (destination / "codex.stderr.txt").write_bytes(stderr)
    archive_path = destination / "final-source.zip"
    revision_sha256, archive_sha256 = capture_final_source(workspace, archive_path)

    stream = InteractiveStream(
        trial_id,
        wall_limit_seconds=limits["agent_wall_seconds"],
        token_limit=limits["cumulative_input_tokens"],
        termination_grace_seconds=limits["termination_grace_seconds"],
    )
    stream.start()
    for request in recorder.requests:
        forwarded = stream.model_request(
            request["request_id"],
            preflight_input_tokens=request["preflight_input_tokens"],
            provider_input_tokens=request["provider_input_tokens"],
            cached_input_tokens=request["cached_input_tokens"],
            protocol_overhead_tokens=request["protocol_overhead_tokens"],
            categories=request["categories"],
            body=request["body"],
        )
        if forwarded:
            stream.model_response(request["request_id"], request["response"])
    if stream.cause is None:
        if timed_out:
            stream.stop_for_limit("timed_out")
        elif recorder.limit_reached:
            stream.stop_for_limit("input_token_limit")
        elif recorder.failure is not None:
            stream.stop_for_limit("incomplete_evidence")
        else:
            stream.process_result(process.returncode)
    result = stream.finish()
    events_path = destination / "events.jsonl"
    events_sha256 = stream.recorder.write_jsonl(events_path)
    bundle = {
        "live_trial_format": 1,
        "official": "no",
        "trial_id": trial_id,
        "configuration_id": CONFIGURATION_ID,
        "language": language,
        "task": task,
        "terminal": {
            "class": result.terminal_class,
            "cause": result.terminal_cause,
        },
        "model": {
            "requested": "gpt-5.6-sol",
            "reasoning_effort": "high",
            "agent_version": version,
            "agent_sha256": agent_digest,
        },
        "inputs": {
            "task_start_tree_sha256": prepared.task_start_tree_sha256,
            "rendered_prompt_sha256": _sha256_bytes(prepared.prompt),
            "permission_profile_sha256": prepared.permission_profile_sha256,
            "prestart_observations_sha256": _sha256_bytes(
                _canonical_payload(observations)
            ),
        },
        "timing": {
            "started_monotonic_ns": started_ns,
            "stopped_monotonic_ns": stopped_ns,
            "elapsed_ns": stopped_ns - started_ns,
        },
        "token_accounting": result.token_accounting,
        "recorder": {
            "request_count": len(recorder.requests),
            "failure": recorder.failure,
            "limit_reached": recorder.limit_reached,
            "authorization_evidence": "redacted",
        },
        "process": {
            "exit_status": process.returncode,
            "timed_out": timed_out,
            "stdout_path": "codex.stdout.jsonl",
            "stdout_sha256": _sha256(destination / "codex.stdout.jsonl"),
            "stderr_path": "codex.stderr.txt",
            "stderr_sha256": _sha256(destination / "codex.stderr.txt"),
        },
        "artifacts": {
            "events_path": "events.jsonl",
            "events_sha256": events_sha256,
            "events_count": len(result.events),
            "final_source_path": "final-source.zip",
            "final_source_tree_sha256": revision_sha256,
            "final_source_sha256": archive_sha256,
        },
    }
    bundle_path = destination / "live-trial.json"
    _write_json(bundle_path, bundle)
    verify_live_trial_bundle(bundle_path)
    return bundle


def verify_live_trial_bundle(bundle_path: Path) -> dict[str, Any]:
    """Verify the secret-free live bundle and every referenced local artifact."""

    bundle = _load_object(bundle_path, "runner_live_evidence_invalid")
    root = bundle_path.resolve().parent
    if (
        bundle.get("live_trial_format") != 1
        or bundle.get("official") != "no"
        or bundle.get("configuration_id") != CONFIGURATION_ID
        or bundle.get("language") not in ("rust", "go", "python", "typescript")
        or bundle.get("task") not in ("UC-001", "UC-003")
    ):
        _raise("runner_live_evidence_invalid", "live trial identity differs")
    contract = _load_object(EXPERIMENT_CONTRACT, "runner_live_evidence_invalid")
    model = bundle.get("model")
    if not isinstance(model, dict) or model != {
        "requested": contract["agent"]["model_request"],
        "reasoning_effort": contract["agent"]["reasoning_effort"],
        "agent_version": contract["agent"]["agent_version"],
        "agent_sha256": contract["agent"]["agent_sha256"],
    }:
        _raise("runner_live_evidence_invalid", "model or agent identity differs")

    accounting = bundle.get("token_accounting")
    if not isinstance(accounting, dict):
        _raise("runner_live_evidence_invalid", "token accounting is missing")
    requests = accounting.get("requests")
    if not isinstance(requests, list):
        _raise("runner_live_evidence_invalid", "request accounting is missing")
    categories = {category: 0 for category in TOKEN_CATEGORIES}
    total_input = 0
    total_cached = 0
    for index, request in enumerate(requests, start=1):
        if not isinstance(request, dict):
            _raise("runner_live_evidence_invalid", f"request {index} is invalid")
        per_category = request.get("categories")
        if (
            not isinstance(per_category, dict)
            or tuple(per_category) != TOKEN_CATEGORIES
        ):
            _raise(
                "runner_live_evidence_invalid",
                f"request {index} categories differ",
            )
        values = tuple(per_category.values())
        if any(
            not isinstance(value, int) or isinstance(value, bool) or value < 0
            for value in values
        ):
            _raise(
                "runner_live_evidence_invalid",
                f"request {index} category count is invalid",
            )
        preflight = request.get("preflight_input_tokens")
        provider = request.get("provider_input_tokens")
        cached = request.get("cached_input_tokens")
        overhead = request.get("protocol_overhead_tokens")
        if (
            not all(
                isinstance(value, int) and not isinstance(value, bool) and value >= 0
                for value in (preflight, provider, cached, overhead)
            )
            or preflight != provider
            or sum(values) + overhead != provider
            or cached > provider
        ):
            _raise(
                "runner_live_evidence_invalid",
                f"request {index} token reconciliation differs",
            )
        total_input += provider
        total_cached += cached
        for category, value in per_category.items():
            categories[category] += value
    if (
        accounting.get("total_input_tokens") != total_input
        or accounting.get("cached_input_tokens") != total_cached
        or accounting.get("categories") != categories
        or total_input > contract["limits"]["cumulative_input_tokens"]
    ):
        _raise("runner_live_evidence_invalid", "cumulative accounting differs")

    def checked_artifact(relative: Any, digest: Any) -> Path:
        if not isinstance(relative, str) or Path(relative).is_absolute():
            _raise("runner_live_evidence_invalid", "artifact path is invalid")
        path = (root / relative).resolve()
        try:
            path.relative_to(root)
        except ValueError:
            _raise("runner_live_evidence_invalid", "artifact leaves bundle root")
        if not path.is_file() or _sha256(path) != digest:
            _raise(
                "runner_live_evidence_invalid",
                f"{relative}: artifact digest differs",
            )
        return path

    process = bundle.get("process")
    artifacts = bundle.get("artifacts")
    if not isinstance(process, dict) or not isinstance(artifacts, dict):
        _raise("runner_live_evidence_invalid", "artifact metadata is missing")
    checked_artifact(process.get("stdout_path"), process.get("stdout_sha256"))
    checked_artifact(process.get("stderr_path"), process.get("stderr_sha256"))
    events_path = checked_artifact(
        artifacts.get("events_path"), artifacts.get("events_sha256")
    )
    checked_artifact(
        artifacts.get("final_source_path"),
        artifacts.get("final_source_sha256"),
    )
    try:
        events = [
            json.loads(line)
            for line in events_path.read_text(encoding="utf-8").splitlines()
        ]
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        _raise("runner_live_evidence_invalid", f"raw events: {error}")
    if len(events) != artifacts.get("events_count"):
        _raise("runner_live_evidence_invalid", "raw event count differs")
    for sequence, event in enumerate(events, start=1):
        if (
            not isinstance(event, dict)
            or event.get("sequence") != sequence
            or event.get("trial_id") != bundle.get("trial_id")
            or event.get("configuration_id") != CONFIGURATION_ID
            or event.get("payload_sha256")
            != _sha256_bytes(_canonical_payload(event.get("payload")))
        ):
            _raise(
                "runner_live_evidence_invalid",
                f"raw event {sequence} differs",
            )
    terminal = bundle.get("terminal")
    if (
        not events
        or events[0].get("kind") != "trial.started"
        or events[-1].get("kind") != "trial.stopped"
        or events[-1].get("payload") != terminal
    ):
        _raise("runner_live_evidence_invalid", "terminal event differs")
    if terminal == {"class": "successful", "cause": "complete"} and not requests:
        _raise("runner_live_evidence_invalid", "successful bundle has no request")
    return bundle


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command")
    subparsers.add_parser("self-test", help="run fake/dry runner checks")
    live = subparsers.add_parser(
        "run-live-trial", help="run one non-official readiness pilot"
    )
    live.add_argument(
        "--language",
        required=True,
        choices=("rust", "go", "python", "typescript"),
    )
    live.add_argument("--task", required=True, choices=("UC-001", "UC-003"))
    live.add_argument("--output", required=True, type=Path)
    live.add_argument(
        "--codex",
        type=Path,
        default=Path(shutil.which("codex") or "codex"),
    )
    live.add_argument("--tool-path", default=os.environ.get("PATH", ""))
    live.add_argument("--api-key-env", default="OPENAI_API_KEY")
    live.add_argument(
        "--upstream-base-url", default="https://api.openai.com/v1"
    )
    verify = subparsers.add_parser(
        "verify-live-trial", help="verify one retained live trial bundle"
    )
    verify.add_argument("bundle", type=Path)
    return parser


def main() -> int:
    arguments = _parser().parse_args()
    if arguments.command in (None, "self-test"):
        outcomes = verify_fake_and_dry_streams()
        print(
            "M8c interactive runner passed: "
            f"{len(outcomes)} stable fake/dry terminal outcomes."
        )
        return 0
    if arguments.command == "verify-live-trial":
        bundle = verify_live_trial_bundle(arguments.bundle)
        print(f"{bundle['trial_id']}: live trial evidence verified")
        return 0
    api_key = os.environ.get(arguments.api_key_env)
    if not api_key:
        _raise(
            "runner_credential_missing",
            f"export {arguments.api_key_env} with an OpenAI API key; "
            "the key is retained only by the loopback recorder and is not "
            "placed in the trial tool environment",
        )
    bundle = run_live_trial(
        language=arguments.language,
        task=arguments.task,
        destination=arguments.output,
        api_key=api_key,
        codex_path=arguments.codex,
        tool_path=arguments.tool_path,
        upstream_base_url=arguments.upstream_base_url,
    )
    print(
        f"{bundle['trial_id']}: "
        f"{bundle['terminal']['class']}/{bundle['terminal']['cause']}"
    )
    return 0 if bundle["terminal"]["cause"] == "complete" else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RunnerError, RecorderFailure) as error:
        print(error, file=sys.stderr)
        raise SystemExit(2)
