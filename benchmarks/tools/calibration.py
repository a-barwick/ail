#!/usr/bin/env python3
"""Validate M8 calibration contracts, evidence, reports, and synthetic campaigns."""

from __future__ import annotations

import hashlib
import json
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, NoReturn

import fixtures as fixture_tool


ROOT = Path(__file__).resolve().parents[2]
CALIBRATION_ROOT = ROOT / "benchmarks" / "calibration"
SCHEMA_ROOT = ROOT / "benchmarks" / "schemas"
EXPERIMENT_CONTRACT = CALIBRATION_ROOT / "experiment-contract.json"
CALIBRATION_LOCK = CALIBRATION_ROOT / "contract-lock.json"
SYNTHETIC_ROOT = CALIBRATION_ROOT / "synthetic"
TASK_START_LOCK = ROOT / "benchmarks" / "task-starts" / "task-starts.json"
M7_PARITY_REPORT = ROOT / "benchmarks" / "m7-parity-report.json"

LANGUAGES = ("rust", "go", "python", "typescript")
TASKS = ("UC-001", "UC-003")
TOKEN_CATEGORIES = (
    "initial_context",
    "source_reads",
    "semantic_tool_output",
    "diagnostics",
    "build_and_test_output",
    "other_tool_output",
)
CONFIGURATION_ID = "m8-agent-experiment-v1"
OFFICIAL_REQUIREMENTS = {
    "successful_agent_trials_per_configuration": 10,
    "warm_measurements_per_language": 30,
    "cold_measurements_per_language": 30,
}
SYNTHETIC_REQUIREMENTS = {
    "successful_agent_trials_per_configuration": 1,
    "warm_measurements_per_language": 1,
    "cold_measurements_per_language": 1,
}
SCHEMAS = {
    "experiment_contract": SCHEMA_ROOT / "calibration-experiment-contract.schema.json",
    "campaign": SCHEMA_ROOT / "calibration-campaign.schema.json",
    "agent_trial": SCHEMA_ROOT / "calibration-agent-trial.schema.json",
    "raw_events": SCHEMA_ROOT / "calibration-raw-event.schema.json",
    "warm_measurement": SCHEMA_ROOT / "calibration-warm-measurement.schema.json",
    "cold_measurement": SCHEMA_ROOT / "calibration-cold-measurement.schema.json",
    "evidence_index": SCHEMA_ROOT / "calibration-evidence-index.schema.json",
    "report": SCHEMA_ROOT / "calibration-report.schema.json",
}
LOCKED_ARTIFACTS = (
    "benchmarks/calibration/README.md",
    "benchmarks/calibration/experiment-contract.json",
    "benchmarks/calibration/synthetic/complete.json",
    "benchmarks/calibration/synthetic/empty.json",
    "benchmarks/calibration/synthetic/malformed.json",
    "benchmarks/calibration/synthetic/partial.json",
    "benchmarks/calibration/synthetic/pilot.json",
    "benchmarks/calibration/synthetic/rejections.json",
    "benchmarks/schemas/calibration-agent-trial.schema.json",
    "benchmarks/schemas/calibration-campaign.schema.json",
    "benchmarks/schemas/calibration-cold-measurement.schema.json",
    "benchmarks/schemas/calibration-evidence-index.schema.json",
    "benchmarks/schemas/calibration-experiment-contract.schema.json",
    "benchmarks/schemas/calibration-raw-event.schema.json",
    "benchmarks/schemas/calibration-report.schema.json",
    "benchmarks/schemas/calibration-warm-measurement.schema.json",
    "benchmarks/tests/test_calibration.py",
    "benchmarks/tools/calibration.py",
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
class CalibrationError(Exception):
    code: str
    message: str

    def __str__(self) -> str:
        return f"{self.code}: {self.message}"


@dataclass(frozen=True)
class CampaignResult:
    campaign_id: str
    completion_state: str
    agent_trials: int
    warm_measurements: int
    cold_measurements: int


def _raise(code: str, message: str) -> NoReturn:
    raise CalibrationError(code, message)


def _canonical(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def _canonical_payload(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")


def _event_line(value: dict[str, Any]) -> str:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    )


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _sha256(path: Path) -> str:
    return _sha256_bytes(path.read_bytes())


def _load_object(path: Path, code: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        _raise(code, f"{path}: {error}")
    if not isinstance(value, dict):
        _raise(code, f"{path}: must contain a JSON object")
    return value


def _require_canonical(path: Path, value: dict[str, Any], code: str) -> None:
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        _raise(code, f"{path}: {error}")
    if text != _canonical(value):
        _raise(code, f"{path}: must be canonical two-space JSON")


def _validate_schema(value: dict[str, Any], schema_path: Path, code: str) -> None:
    schema = fixture_tool._load_json(schema_path)
    problems = fixture_tool._schema_errors(value, schema, schema, "$")
    if problems:
        _raise(code, "; ".join(str(problem) for problem in problems[:8]))


def _repo_path(raw_path: str, code: str) -> Path:
    path = (ROOT / raw_path).resolve()
    try:
        path.relative_to(ROOT)
    except ValueError:
        _raise(code, f"path leaves repository: {raw_path!r}")
    return path


def _campaign_path(root: Path, raw_path: str, code: str) -> Path:
    if not raw_path or Path(raw_path).is_absolute():
        _raise(code, f"invalid campaign-relative path: {raw_path!r}")
    path = (root / raw_path).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError:
        _raise(code, f"path leaves campaign: {raw_path!r}")
    return path


def _nonnegative_integer(value: Any, code: str, location: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        _raise(code, f"{location}: must be a non-negative integer")
    return value


def _check_exact_keys(
    value: dict[str, Any], expected: tuple[str, ...], code: str, location: str
) -> None:
    if tuple(value) != expected:
        _raise(code, f"{location}: expected ordered keys {list(expected)}")


def _rendered_prompt(task_path: Path) -> bytes:
    task_bytes = task_path.read_bytes()
    try:
        task_bytes.decode("utf-8")
    except UnicodeDecodeError as error:
        _raise("calibration_contract_invalid", f"{task_path}: {error}")
    if b"\r" in task_bytes or not task_bytes.endswith(b"\n"):
        _raise(
            "calibration_contract_invalid",
            f"{task_path}: task must use LF and end with one LF",
        )
    return PROMPT_PREFIX.encode() + task_bytes + PROMPT_SUFFIX.encode()


def _check_experiment_contract(contract: dict[str, Any]) -> None:
    _validate_schema(
        contract, SCHEMAS["experiment_contract"], "calibration_contract_invalid"
    )
    _require_canonical(EXPERIMENT_CONTRACT, contract, "calibration_contract_invalid")
    if contract["authority"] != {
        "decision": "docs/decisions/0002-m8-agent-experiment-contract.md",
        "requirements": ["NFR-001", "NFR-002", "NFR-003", "NFR-004", "NFR-005"],
    }:
        _raise("calibration_contract_invalid", "authority differs from M8a")
    agent = contract["agent"]
    expected_agent = {
        "provider": "openai-responses",
        "model_request": "gpt-5.6-sol",
        "reasoning_effort": "high",
        "mode": "standard",
        "agent": "one-root-codex-cli",
        "agent_version": "codex-cli 0.144.6",
        "agent_sha256": (
            "80a3933d11a9d13ef806aa24f7bb8afc9169cfe4e9b09d6da6a92922cbde9cff"
        ),
        "delegation": "disabled",
        "sampling": {
            "temperature": "unavailable",
            "top_p": "unavailable",
            "seed": "unavailable",
        },
    }
    if agent != expected_agent:
        _raise("calibration_contract_invalid", "agent treatment differs from M8a")
    protocol = contract["protocol"]
    if (
        protocol.get("command")
        != [
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
        or protocol.get("model_turns") != "adaptive-local-tools"
        or protocol.get("request_retries") != 0
        or protocol.get("stream_retries") != 0
        or protocol.get("stream_idle_seconds") != 300
    ):
        _raise("calibration_contract_invalid", "interactive protocol differs from M8a")
    prompt = contract["prompt"]
    if prompt.get("prefix_sha256") != _sha256_bytes(PROMPT_PREFIX.encode()):
        _raise("calibration_contract_invalid", "prompt prefix digest differs")
    if prompt.get("suffix_sha256") != _sha256_bytes(PROMPT_SUFFIX.encode()):
        _raise("calibration_contract_invalid", "prompt suffix digest differs")
    task_paths = {
        "UC-001": ROOT / "benchmarks" / "tasks" / "uc001-implement-create-job.md",
        "UC-003": ROOT / "benchmarks" / "tasks" / "uc003-add-priority.md",
    }
    expected_prompts = [
        {
            "task": task,
            "task_sha256": _sha256(path),
            "rendered_prompt_sha256": _sha256_bytes(_rendered_prompt(path)),
        }
        for task, path in task_paths.items()
    ]
    if prompt.get("rendered") != expected_prompts:
        _raise("calibration_contract_invalid", "rendered prompt lock differs")
    if contract["initial_context"] != [
        "version-bound-codex-base-instructions",
        "built-in-local-tool-schemas",
        "exact-rendered-task-prompt",
        "minimal-fixed-transport-metadata",
    ]:
        _raise("calibration_contract_invalid", "initial context differs from M8a")

    task_start_lock = _load_object(TASK_START_LOCK, "calibration_contract_invalid")
    expected_starts = [
        {
            "configuration_id": entry["id"],
            "language": entry["language"],
            "task": entry["task"],
            "tree_sha256": entry["tree_sha256"],
        }
        for entry in task_start_lock["configurations"]
    ]
    if contract["task_starts"] != expected_starts:
        _raise("calibration_contract_invalid", "task-start lock differs from M7")
    if contract["tools"] != {
        "common": [
            "codex-shell",
            "codex-file-edit",
            "read-only-discovery",
            "git-status",
            "git-diff",
        ],
        "rust": [
            "rustc 1.88.0",
            "cargo 1.88.0",
            "rustfmt 1.8.0",
            "clippy 0.1.88",
            "rust-analyzer 1.88.0",
        ],
        "go": [
            "go 1.26.0",
            "gofmt 1.26.0",
            "go vet 1.26.0",
            "gopls 0.21.1",
        ],
        "python": [
            "cpython 3.13.5",
            "uv 0.7.12",
            "mypy 1.17.0",
            "ruff 0.12.4",
            "pytest 8.4.1",
        ],
        "typescript": [
            "node 23.10.0",
            "npm 10.9.2",
            "typescript 5.8.3",
            "eslint 9.31.0",
            "prettier 3.6.2",
            "tsx 4.20.3",
            "c8 10.1.3",
        ],
    }:
        _raise("calibration_contract_invalid", "normal tool set differs from M8a")
    token = contract["token_accounting"]
    if (
        token.get("authoritative_total") != "provider-reported-request-usage"
        or token.get("preflight_counter") != "POST /v1/responses/input_tokens"
        or token.get("attribution") != "ordered-cumulative-prefix-delta-v1"
        or token.get("categories") != list(TOKEN_CATEGORIES)
        or token.get("cached_input_deducted") != "no"
        or token.get("repeated_delivery_counted") != "yes"
        or token.get("reconciliation_tolerance_tokens") != 0
    ):
        _raise("calibration_contract_invalid", "token accounting differs from M8a")
    if contract["limits"] != {
        "agent_wall_seconds": 600,
        "cumulative_input_tokens": 500000,
        "termination_grace_seconds": 5,
        "warm_corpus_seconds": 30,
        "cold_start_milliseconds": 2000,
        "peak_rss_bytes": 536870912,
    }:
        _raise("calibration_contract_invalid", "limits differ from accepted rules")
    if contract["terminal_classes"] != [
        "configuration_rejection",
        "failed",
        "timed_out",
        "successful",
    ]:
        _raise("calibration_contract_invalid", "terminal classes differ from M8a")
    if contract["environment"] != {
        "reference_host": "ail-m8-reference-mac-01",
        "operating_system": "macOS 26.5.2",
        "operating_system_build": "25F84",
        "architecture": "apple-arm64",
        "container": "none",
        "campaign_concurrency": "sequential",
        "external_dependencies": "preprovisioned-offline",
    }:
        _raise("calibration_contract_invalid", "reference environment differs from M8a")
    expected_schema_paths = [
        path.relative_to(ROOT).as_posix() for path in SCHEMAS.values()
    ]
    if contract["schemas"] != expected_schema_paths:
        _raise("calibration_contract_invalid", "schema registry is incomplete")


def check_contract_lock() -> dict[str, Any]:
    lock = _load_object(CALIBRATION_LOCK, "calibration_contract_changed")
    _require_canonical(CALIBRATION_LOCK, lock, "calibration_contract_changed")
    if tuple(lock) != ("calibration_contract_lock_format", "artifacts"):
        _raise("calibration_contract_changed", "unexpected lock shape")
    if lock["calibration_contract_lock_format"] != 1:
        _raise("calibration_contract_changed", "unsupported lock format")
    artifacts = lock["artifacts"]
    if not isinstance(artifacts, list):
        _raise("calibration_contract_changed", "artifacts must be an array")
    paths = [entry.get("path") for entry in artifacts if isinstance(entry, dict)]
    if paths != list(LOCKED_ARTIFACTS):
        _raise("calibration_contract_changed", "locked artifact set or order differs")
    for entry in artifacts:
        path = _repo_path(entry["path"], "calibration_contract_changed")
        if not path.is_file() or _sha256(path) != entry.get("sha256"):
            _raise(
                "calibration_contract_changed",
                f"{entry['path']}: digest differs",
            )
        if path.suffix == ".json":
            value = _load_object(path, "calibration_contract_invalid")
            _require_canonical(path, value, "calibration_contract_invalid")
    contract = _load_object(EXPERIMENT_CONTRACT, "calibration_contract_invalid")
    _check_experiment_contract(contract)
    return contract


def _load_index(index_path: Path, lock_path: Path) -> tuple[dict[str, Any], Path]:
    lock = _load_object(lock_path, "evidence_index_lock_invalid")
    _require_canonical(lock_path, lock, "evidence_index_lock_invalid")
    if (
        tuple(lock)
        != (
            "evidence_index_lock_format",
            "index_path",
            "index_sha256",
        )
        or lock.get("evidence_index_lock_format") != 1
    ):
        _raise("evidence_index_lock_invalid", "unexpected evidence lock shape")
    locked_index = _campaign_path(
        lock_path.parent, lock["index_path"], "evidence_index_lock_invalid"
    )
    if locked_index != index_path.resolve():
        _raise("evidence_index_lock_invalid", "lock names another evidence index")
    if not index_path.is_file():
        _raise("evidence_index_lock_invalid", "evidence index does not exist")
    if _sha256(index_path) != lock["index_sha256"]:
        _raise("evidence_index_changed", "evidence index digest differs")
    index = _load_object(index_path, "evidence_index_invalid")
    _validate_schema(index, SCHEMAS["evidence_index"], "evidence_index_invalid")
    _require_canonical(index_path, index, "evidence_index_invalid")
    return index, index_path.parent.resolve()


def _load_raw_events(path: Path) -> list[dict[str, Any]]:
    try:
        lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    except (OSError, UnicodeDecodeError) as error:
        _raise("raw_events_invalid", f"{path}: {error}")
    if not lines:
        _raise("raw_events_invalid", f"{path}: raw event stream is empty")
    events: list[dict[str, Any]] = []
    for line_number, line in enumerate(lines, 1):
        try:
            value = json.loads(line)
        except json.JSONDecodeError as error:
            _raise("raw_events_invalid", f"{path}:{line_number}: {error.msg}")
        if not isinstance(value, dict):
            _raise("raw_events_invalid", f"{path}:{line_number}: event must be object")
        _validate_schema(value, SCHEMAS["raw_events"], "raw_events_invalid")
        if line != _event_line(value):
            _raise(
                "raw_events_invalid",
                f"{path}:{line_number}: event must be canonical JSONL",
            )
        if value["payload_sha256"] != _sha256_bytes(
            _canonical_payload(value["payload"])
        ):
            _raise(
                "raw_events_invalid",
                f"{path}:{line_number}: payload digest differs",
            )
        events.append(value)
    return events


def _artifact_records(
    index: dict[str, Any], campaign_root: Path
) -> tuple[dict[str, dict[str, Any]], dict[str, Path]]:
    artifacts = index["artifacts"]
    paths = [entry["path"] for entry in artifacts]
    record_ids = [entry["record_id"] for entry in artifacts]
    if paths != sorted(paths):
        _raise("evidence_index_invalid", "artifact entries must use lexical path order")
    if len(paths) != len(set(paths)) or len(record_ids) != len(set(record_ids)):
        _raise("duplicate_evidence_identity", "artifact path or record_id repeats")
    by_id: dict[str, dict[str, Any]] = {}
    resolved: dict[str, Path] = {}
    for entry in artifacts:
        path = _campaign_path(campaign_root, entry["path"], "evidence_index_invalid")
        if not path.is_file():
            _raise("evidence_artifact_missing", f"{entry['path']}: file is missing")
        if _sha256(path) != entry["sha256"]:
            _raise("evidence_hash_invalid", f"{entry['path']}: digest differs")
        by_id[entry["record_id"]] = entry
        resolved[entry["record_id"]] = path
    return by_id, resolved


def _load_kind_records(
    index: dict[str, Any],
    paths: dict[str, Path],
    kind: str,
) -> list[tuple[dict[str, Any], dict[str, Any]]]:
    records: list[tuple[dict[str, Any], dict[str, Any]]] = []
    schema = SCHEMAS[kind]
    for entry in index["artifacts"]:
        if entry["kind"] != kind:
            continue
        value = _load_object(paths[entry["record_id"]], f"{kind}_invalid")
        _validate_schema(value, schema, f"{kind}_invalid")
        _require_canonical(paths[entry["record_id"]], value, f"{kind}_invalid")
        records.append((entry, value))
    return records


def _check_token_accounting(trial: dict[str, Any]) -> None:
    accounting = trial["token_accounting"]
    total_categories = {category: 0 for category in TOKEN_CATEGORIES}
    total_input = 0
    total_cached = 0
    request_ids: set[str] = set()
    for index, request in enumerate(accounting["requests"]):
        location = f"{trial['trial_id']}.token_accounting.requests[{index}]"
        request_id = request["request_id"]
        if request_id in request_ids:
            _raise("token_accounting_invalid", f"{location}: duplicate request_id")
        request_ids.add(request_id)
        categories = request["categories"]
        if tuple(categories) != TOKEN_CATEGORIES:
            _raise(
                "token_categories_incomplete",
                f"{location}: categories must be complete and ordered",
            )
        category_total = 0
        for category in TOKEN_CATEGORIES:
            count = _nonnegative_integer(
                categories[category],
                "token_accounting_invalid",
                f"{location}.{category}",
            )
            total_categories[category] += count
            category_total += count
        preflight = _nonnegative_integer(
            request["preflight_input_tokens"],
            "token_accounting_invalid",
            f"{location}.preflight_input_tokens",
        )
        provider = _nonnegative_integer(
            request["provider_input_tokens"],
            "token_accounting_invalid",
            f"{location}.provider_input_tokens",
        )
        cached = _nonnegative_integer(
            request["cached_input_tokens"],
            "token_accounting_invalid",
            f"{location}.cached_input_tokens",
        )
        overhead = _nonnegative_integer(
            request["protocol_overhead_tokens"],
            "token_accounting_invalid",
            f"{location}.protocol_overhead_tokens",
        )
        if preflight != provider or category_total + overhead != provider:
            _raise(
                "token_reconciliation_failed",
                f"{location}: preflight, categories, overhead, and provider differ",
            )
        if cached > provider:
            _raise("token_accounting_invalid", f"{location}: cached exceeds input")
        total_input += provider
        total_cached += cached
    if tuple(accounting["categories"]) != TOKEN_CATEGORIES:
        _raise(
            "token_categories_incomplete",
            f"{trial['trial_id']}: total categories must be complete and ordered",
        )
    if accounting["categories"] != total_categories:
        _raise(
            "token_accounting_invalid", f"{trial['trial_id']}: category totals differ"
        )
    if accounting["total_input_tokens"] != total_input:
        _raise("token_accounting_invalid", f"{trial['trial_id']}: input total differs")
    if accounting["cached_input_tokens"] != total_cached:
        _raise("token_accounting_invalid", f"{trial['trial_id']}: cached total differs")
    if total_input > 500000:
        if trial["terminal"] != {"class": "failed", "cause": "input_token_limit"}:
            _raise("trial_classification_invalid", "token limit cause is inconsistent")


def _check_trial(
    trial: dict[str, Any],
    campaign: dict[str, Any],
    contract: dict[str, Any],
    artifacts: dict[str, dict[str, Any]],
    paths: dict[str, Path],
    raw_by_id: dict[str, list[dict[str, Any]]],
) -> None:
    if trial["campaign_id"] != campaign["campaign_id"]:
        _raise("mixed_configuration", f"{trial['trial_id']}: campaign_id differs")
    if trial["configuration_id"] != CONFIGURATION_ID:
        _raise("mixed_configuration", f"{trial['trial_id']}: configuration differs")
    profile = campaign["profile"]
    expected_official = "yes" if profile == "official" else "no"
    if trial["official"] != expected_official:
        _raise("mixed_configuration", f"{trial['trial_id']}: official flag differs")
    config_id = f"{trial['language']}-{trial['task'].lower().replace('-', '')}"
    starts = {entry["configuration_id"]: entry for entry in contract["task_starts"]}
    start = starts.get(config_id)
    if start is None:
        _raise("changed_campaign_input", f"{trial['trial_id']}: task start is unknown")
    prompt = {entry["task"]: entry for entry in contract["prompt"]["rendered"]}[
        trial["task"]
    ]
    parity = _load_object(M7_PARITY_REPORT, "calibration_contract_invalid")
    expected_inputs = {
        "task_start_tree_sha256": start["tree_sha256"],
        "task_sha256": prompt["task_sha256"],
        "rendered_prompt_sha256": prompt["rendered_prompt_sha256"],
        "private_package_sha256": parity["hidden_package_sha256"],
    }
    if trial["inputs"] != expected_inputs:
        _raise("changed_campaign_input", f"{trial['trial_id']}: input lock differs")
    if trial["model"]["returned"] != "gpt-5.6-sol":
        _raise("mixed_configuration", f"{trial['trial_id']}: returned model differs")
    if (
        trial["model"]["agent_sha256"] != contract["agent"]["agent_sha256"]
        or trial["model"]["backend_snapshot"] != "unavailable"
    ):
        _raise("mixed_configuration", f"{trial['trial_id']}: model evidence differs")
    timing = trial["timing"]
    for key in ("started_monotonic_ns", "stopped_monotonic_ns", "elapsed_ns"):
        _nonnegative_integer(timing[key], "trial_invalid", f"{trial['trial_id']}.{key}")
    if (
        timing["stopped_monotonic_ns"] - timing["started_monotonic_ns"]
        != timing["elapsed_ns"]
    ):
        _raise("trial_invalid", f"{trial['trial_id']}: elapsed time differs")
    for key, value in trial["activity"].items():
        _nonnegative_integer(value, "trial_invalid", f"{trial['trial_id']}.{key}")
    _check_token_accounting(trial)

    raw_reference = trial["raw_events"]
    record_id = raw_reference["record_id"]
    entry = artifacts.get(record_id)
    if entry is None or entry["kind"] != "raw_events":
        _raise("raw_events_missing", f"{trial['trial_id']}: raw events not indexed")
    if (
        entry["path"] != raw_reference["path"]
        or entry["sha256"] != raw_reference["sha256"]
    ):
        _raise("raw_events_invalid", f"{trial['trial_id']}: raw event lock differs")
    events = raw_by_id[record_id]
    if len(events) != raw_reference["count"]:
        _raise("raw_events_invalid", f"{trial['trial_id']}: event count differs")
    if events[0]["kind"] != "trial.started" or events[-1]["kind"] != "trial.stopped":
        _raise("raw_events_invalid", f"{trial['trial_id']}: terminal events missing")
    request_ids = {
        event["payload"].get("request_id")
        for event in events
        if event["kind"] == "model.request"
    }
    expected_request_ids = {
        request["request_id"] for request in trial["token_accounting"]["requests"]
    }
    if request_ids != expected_request_ids:
        _raise("raw_events_invalid", f"{trial['trial_id']}: model requests differ")
    for sequence, event in enumerate(events, 1):
        if event["sequence"] != sequence:
            _raise("raw_events_invalid", f"{trial['trial_id']}: sequence differs")
        if (
            event["trial_id"] != trial["trial_id"]
            or event["configuration_id"] != CONFIGURATION_ID
        ):
            _raise("mixed_configuration", f"{trial['trial_id']}: raw event differs")

    artifact_paths = {
        entry["path"]: entry for entry in artifacts.values() if entry["kind"] == "blob"
    }
    for reference in trial["artifacts"]:
        indexed = artifact_paths.get(reference["path"])
        if indexed is None or indexed["sha256"] != reference["sha256"]:
            _raise("evidence_artifact_missing", f"{reference['path']}: not indexed")
    terminal = trial["terminal"]["class"]
    correctness = trial["correctness"]
    success = all(
        correctness[key] == "passed"
        for key in ("public", "private", "seeded_consumers", "completion_evidence")
    )
    clean_permissions = (
        trial["permissions"]["violation_count"] == 0
        and trial["permissions"]["protected_artifacts_match"] == "yes"
    )
    if terminal == "successful" and not (success and clean_permissions):
        _raise("trial_classification_invalid", f"{trial['trial_id']}: false success")


def _check_measurement(
    record: dict[str, Any],
    kind: str,
    campaign: dict[str, Any],
    artifacts: dict[str, dict[str, Any]],
) -> None:
    record_id = record["measurement_id"]
    if record["campaign_id"] != campaign["campaign_id"]:
        _raise("mixed_configuration", f"{record_id}: campaign differs")
    if record["configuration_id"] != CONFIGURATION_ID:
        _raise("mixed_configuration", f"{record_id}: configuration differs")
    expected_official = "yes" if campaign["profile"] == "official" else "no"
    if record["official"] != expected_official:
        _raise("mixed_configuration", f"{record_id}: official flag differs")
    reason = record["exclusion_reason"]
    if (record["status"] == "excluded") != bool(reason):
        _raise(
            "exclusion_unaccounted", f"{record_id}: exclusion reason is inconsistent"
        )
    if kind == "warm_measurement":
        reference_path = record["latency"]["samples_path"]
        reference_sha = record["latency"]["samples_sha256"]
        for key in ("sample_count", "p50_ns", "p95_ns", "p99_ns"):
            _nonnegative_integer(
                record["latency"][key], "warm_measurement_invalid", f"{record_id}.{key}"
            )
        _nonnegative_integer(
            record["throughput_milli_requests_per_second"],
            "warm_measurement_invalid",
            f"{record_id}.throughput",
        )
        latency = record["latency"]
        if not (
            latency["sample_count"] > 0
            and latency["p50_ns"] <= latency["p95_ns"] <= latency["p99_ns"]
        ):
            _raise(
                "warm_measurement_invalid",
                f"{record_id}: latency sample or percentile order is invalid",
            )
        corpus_elapsed = _nonnegative_integer(
            record["corpus"].get("elapsed_ns"),
            "warm_measurement_invalid",
            f"{record_id}.corpus.elapsed_ns",
        )
        included_ok = (
            record["readiness"].get("observed") == "yes"
            and record["corpus"].get("status") == "passed"
            and record["corpus"].get("trace") == "passed"
            and corpus_elapsed <= 30_000_000_000
            and record["throughput_milli_requests_per_second"] > 0
        )
    else:
        reference_path = record["package_manifest_path"]
        reference_sha = record["package_manifest_sha256"]
        for key in (
            "process_creation_ns",
            "readiness_ns",
            "idle_rss_bytes",
            "peak_rss_bytes",
            "external_access_attempts",
        ):
            _nonnegative_integer(
                record[key], "cold_measurement_invalid", f"{record_id}.{key}"
            )
        included_ok = (
            record["exit_status"] == 0
            and record["corpus"].get("status") == "passed"
            and record["corpus"].get("trace") == "passed"
            and record["readiness_ns"] <= 2_000_000_000
            and record["peak_rss_bytes"] <= 536_870_912
            and record["external_access_attempts"] == 0
        )
    if record["status"] == "included" and not included_ok:
        _raise(
            f"{kind}_invalid",
            f"{record_id}: an included measurement violates correctness or safety",
        )
    matching = [
        entry
        for entry in artifacts.values()
        if entry["kind"] == "blob" and entry["path"] == reference_path
    ]
    if len(matching) != 1 or matching[0]["sha256"] != reference_sha:
        _raise("evidence_artifact_missing", f"{reference_path}: not indexed")


def _derive_report(
    campaign: dict[str, Any],
    trials: list[dict[str, Any]],
    warm: list[dict[str, Any]],
    cold: list[dict[str, Any]],
) -> dict[str, Any]:
    agent_distributions: list[dict[str, Any]] = []
    for language in LANGUAGES:
        for task in TASKS:
            records = sorted(
                (
                    trial
                    for trial in trials
                    if trial["language"] == language and trial["task"] == task
                ),
                key=lambda item: item["trial_id"],
            )
            categories = {
                category: [
                    record["token_accounting"]["categories"][category]
                    for record in records
                ]
                for category in TOKEN_CATEGORIES
            }
            agent_distributions.append(
                {
                    "configuration": f"{language}-{task.lower().replace('-', '')}",
                    "attempts": len(records),
                    "successful": sum(
                        record["terminal"]["class"] == "successful"
                        for record in records
                    ),
                    "failed": sum(
                        record["terminal"]["class"] == "failed" for record in records
                    ),
                    "timed_out": sum(
                        record["terminal"]["class"] == "timed_out" for record in records
                    ),
                    "input_tokens": [
                        record["token_accounting"]["total_input_tokens"]
                        for record in records
                    ],
                    "token_categories": categories,
                    "edits": [record["activity"]["edits"] for record in records],
                    "validation_attempts": [
                        record["activity"]["validation_attempts"] for record in records
                    ],
                    "repair_cycles": [
                        record["activity"]["repair_cycles"] for record in records
                    ],
                    "elapsed_ns": [
                        record["timing"]["elapsed_ns"] for record in records
                    ],
                }
            )

    warm_distributions: list[dict[str, Any]] = []
    cold_distributions: list[dict[str, Any]] = []
    exclusions: list[dict[str, Any]] = []
    for language in LANGUAGES:
        warm_records = sorted(
            (record for record in warm if record["language"] == language),
            key=lambda item: item["measurement_id"],
        )
        included_warm = [
            record for record in warm_records if record["status"] == "included"
        ]
        warm_distributions.append(
            {
                "language": language,
                "records": len(warm_records),
                "included": len(included_warm),
                "excluded": len(warm_records) - len(included_warm),
                "throughput_milli_requests_per_second": [
                    record["throughput_milli_requests_per_second"]
                    for record in included_warm
                ],
                "p50_latency_ns": [
                    record["latency"]["p50_ns"] for record in included_warm
                ],
                "p95_latency_ns": [
                    record["latency"]["p95_ns"] for record in included_warm
                ],
                "p99_latency_ns": [
                    record["latency"]["p99_ns"] for record in included_warm
                ],
            }
        )
        cold_records = sorted(
            (record for record in cold if record["language"] == language),
            key=lambda item: item["measurement_id"],
        )
        included_cold = [
            record for record in cold_records if record["status"] == "included"
        ]
        cold_distributions.append(
            {
                "language": language,
                "records": len(cold_records),
                "included": len(included_cold),
                "excluded": len(cold_records) - len(included_cold),
                "process_creation_ns": [
                    record["process_creation_ns"] for record in included_cold
                ],
                "readiness_ns": [record["readiness_ns"] for record in included_cold],
                "idle_rss_bytes": [
                    record["idle_rss_bytes"] for record in included_cold
                ],
                "peak_rss_bytes": [
                    record["peak_rss_bytes"] for record in included_cold
                ],
            }
        )
    for kind, records in (("warm_measurement", warm), ("cold_measurement", cold)):
        exclusions.extend(
            {
                "record_id": record["measurement_id"],
                "kind": kind,
                "reason": record["exclusion_reason"],
            }
            for record in records
            if record["status"] == "excluded"
        )
    exclusions.sort(key=lambda item: item["record_id"])
    return {
        "report_format": 1,
        "campaign_id": campaign["campaign_id"],
        "configuration_id": CONFIGURATION_ID,
        "completion_state": campaign["completion_state"],
        "agent_distributions": agent_distributions,
        "warm_distributions": warm_distributions,
        "cold_distributions": cold_distributions,
        "exclusions": exclusions,
        "ail_comparison": "not_permitted_in_m8",
        "ail_targets": "not_permitted_in_m8",
    }


def _check_completion_counts(
    campaign: dict[str, Any],
    trials: list[dict[str, Any]],
    warm: list[dict[str, Any]],
    cold: list[dict[str, Any]],
) -> None:
    profile = campaign["profile"]
    expected = (
        OFFICIAL_REQUIREMENTS if profile == "official" else SYNTHETIC_REQUIREMENTS
    )
    if profile == "pilot":
        expected = SYNTHETIC_REQUIREMENTS
    if campaign["requirements"] != expected:
        _raise("changed_campaign_input", "campaign count requirements differ")
    if campaign["completion_state"] != "complete":
        return
    for language in LANGUAGES:
        for task in TASKS:
            successes = sum(
                trial["language"] == language
                and trial["task"] == task
                and trial["terminal"]["class"] == "successful"
                for trial in trials
            )
            if successes < expected["successful_agent_trials_per_configuration"]:
                _raise(
                    "campaign_counts_missing",
                    f"{language}/{task}: {successes} successful trials",
                )
        included_warm = sum(
            record["language"] == language and record["status"] == "included"
            for record in warm
        )
        if included_warm < expected["warm_measurements_per_language"]:
            _raise(
                "campaign_counts_missing",
                f"{language}: {included_warm} warm measurements",
            )
        included_cold = sum(
            record["language"] == language and record["status"] == "included"
            for record in cold
        )
        if included_cold < expected["cold_measurements_per_language"]:
            _raise(
                "campaign_counts_missing",
                f"{language}: {included_cold} cold measurements",
            )


def verify_campaign(
    index_path: Path,
    lock_path: Path | None = None,
    *,
    contract: dict[str, Any] | None = None,
) -> CampaignResult:
    contract = check_contract_lock() if contract is None else contract
    index_path = index_path.resolve()
    lock_path = (
        index_path.with_name("evidence-index.lock.json")
        if lock_path is None
        else lock_path.resolve()
    )
    index, campaign_root = _load_index(index_path, lock_path)
    contract_digest = _sha256(EXPERIMENT_CONTRACT)
    if (
        index["configuration_id"] != CONFIGURATION_ID
        or index["configuration_sha256"] != contract_digest
    ):
        _raise("changed_campaign_input", "evidence index configuration lock differs")
    artifacts, paths = _artifact_records(index, campaign_root)
    campaign_records = _load_kind_records(index, paths, "campaign")
    report_records = _load_kind_records(index, paths, "report")
    if len(campaign_records) != 1 or len(report_records) != 1:
        _raise("evidence_index_invalid", "campaign and report must appear exactly once")
    campaign = campaign_records[0][1]
    report = report_records[0][1]
    if (
        campaign["campaign_id"] != index["campaign_id"]
        or report["campaign_id"] != index["campaign_id"]
    ):
        _raise("mixed_configuration", "campaign identifiers differ")
    if campaign["configuration"] != {
        "id": CONFIGURATION_ID,
        "path": "benchmarks/calibration/experiment-contract.json",
        "sha256": contract_digest,
    }:
        _raise("changed_campaign_input", "campaign configuration lock differs")

    trial_pairs = _load_kind_records(index, paths, "agent_trial")
    warm_pairs = _load_kind_records(index, paths, "warm_measurement")
    cold_pairs = _load_kind_records(index, paths, "cold_measurement")
    trials = [value for _, value in trial_pairs]
    warm = [value for _, value in warm_pairs]
    cold = [value for _, value in cold_pairs]
    trial_ids = [trial["trial_id"] for trial in trials]
    if len(trial_ids) != len(set(trial_ids)):
        _raise("duplicate_trial_identity", "trial_id repeats")
    measurement_ids = [record["measurement_id"] for record in warm + cold]
    if len(measurement_ids) != len(set(measurement_ids)):
        _raise("duplicate_evidence_identity", "measurement_id repeats")

    raw_by_id: dict[str, list[dict[str, Any]]] = {}
    for entry in index["artifacts"]:
        if entry["kind"] == "raw_events":
            raw_by_id[entry["record_id"]] = _load_raw_events(paths[entry["record_id"]])
    for trial in trials:
        _check_trial(trial, campaign, contract, artifacts, paths, raw_by_id)
    for record in warm:
        _check_measurement(record, "warm_measurement", campaign, artifacts)
    for record in cold:
        _check_measurement(record, "cold_measurement", campaign, artifacts)

    scheduled = [entry["schedule_id"] for entry in campaign["ordering"]]
    if len(scheduled) != len(set(scheduled)):
        _raise("campaign_order_invalid", "schedule_id repeats")
    observed = [trial["schedule_id"] for trial in trials] + [
        record["schedule_id"] for record in warm + cold
    ]
    if sorted(scheduled) != sorted(observed):
        _raise("campaign_order_invalid", "schedule and evidence identities differ")
    expected_report = _derive_report(campaign, trials, warm, cold)
    if report != expected_report:
        if report.get("exclusions") != expected_report["exclusions"]:
            _raise("exclusion_unaccounted", "report exclusions differ from evidence")
        _raise("report_summary_incorrect", "report does not match raw evidence")
    _check_completion_counts(campaign, trials, warm, cold)
    return CampaignResult(
        campaign_id=campaign["campaign_id"],
        completion_state=campaign["completion_state"],
        agent_trials=len(trials),
        warm_measurements=len(warm),
        cold_measurements=len(cold),
    )


def _write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(_canonical(value), encoding="utf-8")


def _blob(root: Path, relative: str, content: bytes) -> dict[str, Any]:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)
    return {
        "record_id": "blob-" + relative.replace("/", "-").replace(".", "-"),
        "kind": "blob",
        "path": relative,
        "sha256": _sha256(path),
    }


def _raw_event(
    sequence: int,
    trial_id: str,
    kind: str,
    phase: str,
    payload: dict[str, Any],
) -> dict[str, Any]:
    return {
        "event_format": 1,
        "sequence": sequence,
        "event_id": f"{trial_id}.event-{sequence}",
        "trial_id": trial_id,
        "configuration_id": CONFIGURATION_ID,
        "phase": phase,
        "kind": kind,
        "monotonic_ns": 1_000_000_000 + sequence,
        "payload": payload,
        "payload_sha256": _sha256_bytes(_canonical_payload(payload)),
    }


def _synthetic_trial(
    root: Path,
    contract: dict[str, Any],
    campaign_id: str,
    language: str,
    task: str,
    round_number: int = 1,
) -> tuple[dict[str, Any], list[dict[str, Any]], list[dict[str, Any]]]:
    short_task = task.lower().replace("-", "")
    configuration = f"{language}-{short_task}"
    trial_id = f"synthetic.{configuration}.{round_number:02d}"
    schedule_id = f"agent.{configuration}.{round_number:02d}"
    events = [
        _raw_event(1, trial_id, "trial.started", "start", {}),
        _raw_event(
            2,
            trial_id,
            "model.request",
            "agent",
            {"request_id": f"{trial_id}.request-1"},
        ),
        _raw_event(
            3,
            trial_id,
            "tool.call",
            "agent",
            {"tool": "exec", "call_id": f"{trial_id}.tool-1"},
        ),
        _raw_event(
            4,
            trial_id,
            "tool.result",
            "agent",
            {"tool": "exec", "call_id": f"{trial_id}.tool-1", "exit": 0},
        ),
        _raw_event(5, trial_id, "trial.stopped", "stop", {"exit": 0}),
    ]
    event_path = root / "events" / f"{trial_id}.jsonl"
    event_path.parent.mkdir(parents=True, exist_ok=True)
    event_path.write_text(
        "".join(_event_line(event) for event in events), encoding="utf-8"
    )
    raw_entry = {
        "record_id": f"events-{trial_id}",
        "kind": "raw_events",
        "path": event_path.relative_to(root).as_posix(),
        "sha256": _sha256(event_path),
    }
    final_blob = _blob(
        root,
        f"artifacts/{trial_id}.final-source.txt",
        f"{trial_id} synthetic final source\n".encode(),
    )
    starts = {entry["configuration_id"]: entry for entry in contract["task_starts"]}
    prompts = {entry["task"]: entry for entry in contract["prompt"]["rendered"]}
    private_digest = _load_object(M7_PARITY_REPORT, "calibration_contract_invalid")[
        "hidden_package_sha256"
    ]
    categories = {
        "initial_context": 100,
        "source_reads": 20,
        "semantic_tool_output": 5,
        "diagnostics": 5,
        "build_and_test_output": 30,
        "other_tool_output": 10,
    }
    request = {
        "request_id": f"{trial_id}.request-1",
        "preflight_input_tokens": 172,
        "provider_input_tokens": 172,
        "cached_input_tokens": 20,
        "protocol_overhead_tokens": 2,
        "categories": categories,
    }
    trial = {
        "trial_format": 1,
        "trial_id": trial_id,
        "schedule_id": schedule_id,
        "campaign_id": campaign_id,
        "configuration_id": CONFIGURATION_ID,
        "language": language,
        "task": task,
        "official": "no",
        "terminal": {"class": "successful", "cause": "complete"},
        "model": {
            "requested": "gpt-5.6-sol",
            "returned": "gpt-5.6-sol",
            "backend_snapshot": "unavailable",
            "service_tier": "unavailable",
            "reasoning_effort": "high",
            "agent_version": "codex-cli 0.144.6",
            "agent_sha256": contract["agent"]["agent_sha256"],
        },
        "inputs": {
            "task_start_tree_sha256": starts[configuration]["tree_sha256"],
            "task_sha256": prompts[task]["task_sha256"],
            "rendered_prompt_sha256": prompts[task]["rendered_prompt_sha256"],
            "private_package_sha256": private_digest,
        },
        "timing": {
            "started_monotonic_ns": 1_000_000_000,
            "stopped_monotonic_ns": 2_000_000_000,
            "elapsed_ns": 1_000_000_000,
        },
        "raw_events": {
            "record_id": raw_entry["record_id"],
            "path": raw_entry["path"],
            "sha256": raw_entry["sha256"],
            "count": len(events),
        },
        "token_accounting": {
            "requests": [request],
            "total_input_tokens": 172,
            "cached_input_tokens": 20,
            "categories": categories,
        },
        "activity": {
            "edits": 1,
            "validation_attempts": 1,
            "incomplete_validations": 0,
            "repair_cycles": 0,
        },
        "permissions": {
            "profile_sha256": "1" * 64,
            "violation_count": 0,
            "protected_artifacts_match": "yes",
            "external_access_attempts": 0,
        },
        "correctness": {
            "revision_sha256": "2" * 64,
            "public": "passed",
            "private": "passed",
            "seeded_consumers": "passed",
            "completion_evidence": "passed",
        },
        "artifacts": [
            {
                "role": "final_source",
                "path": final_blob["path"],
                "sha256": final_blob["sha256"],
            }
        ],
    }
    return (
        trial,
        [raw_entry, final_blob],
        [
            {
                "schedule_id": schedule_id,
                "kind": "agent_trial",
                "round": round_number,
                "language": language,
                "task": task,
            }
        ],
    )


def _synthetic_warm(
    root: Path, campaign_id: str, language: str, round_number: int = 1
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    measurement_id = f"synthetic.warm.{language}.{round_number:02d}"
    schedule_id = f"warm.{language}.{round_number:02d}"
    samples = _blob(
        root,
        f"artifacts/{measurement_id}.samples.txt",
        b"100\n110\n120\n",
    )
    record = {
        "warm_measurement_format": 1,
        "measurement_id": measurement_id,
        "schedule_id": schedule_id,
        "campaign_id": campaign_id,
        "configuration_id": CONFIGURATION_ID,
        "language": language,
        "official": "no",
        "round": round_number,
        "status": "included",
        "exclusion_reason": "",
        "environment_sha256": "3" * 64,
        "package_sha256": "4" * 64,
        "dependency_lock_sha256": "5" * 64,
        "readiness": {"signal": "ready", "observed": "yes"},
        "warmup": {"iterations": 3},
        "clock": {"name": "monotonic"},
        "affinity": {"policy": "recorded"},
        "load": {"concurrent_processes": 0},
        "corpus": {
            "status": "passed",
            "trace": "passed",
            "elapsed_ns": 1_000_000,
        },
        "latency": {
            "sample_count": 3,
            "samples_path": samples["path"],
            "samples_sha256": samples["sha256"],
            "p50_ns": 110,
            "p95_ns": 120,
            "p99_ns": 120,
        },
        "throughput_milli_requests_per_second": 1_000_000,
    }
    schedule = {
        "schedule_id": schedule_id,
        "kind": "warm_measurement",
        "round": round_number,
        "language": language,
        "task": "not_applicable",
    }
    return record, samples, schedule


def _synthetic_cold(
    root: Path, campaign_id: str, language: str, round_number: int = 1
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    measurement_id = f"synthetic.cold.{language}.{round_number:02d}"
    schedule_id = f"cold.{language}.{round_number:02d}"
    package = _blob(
        root,
        f"artifacts/{measurement_id}.package.txt",
        f"{language} synthetic package\n".encode(),
    )
    record = {
        "cold_measurement_format": 1,
        "measurement_id": measurement_id,
        "schedule_id": schedule_id,
        "campaign_id": campaign_id,
        "configuration_id": CONFIGURATION_ID,
        "language": language,
        "official": "no",
        "round": round_number,
        "status": "included",
        "exclusion_reason": "",
        "environment_sha256": "3" * 64,
        "package_manifest_path": package["path"],
        "package_manifest_sha256": package["sha256"],
        "dependency_lock_sha256": "5" * 64,
        "clock": {"name": "monotonic"},
        "affinity": {"policy": "recorded"},
        "load": {"concurrent_processes": 0},
        "readiness_signal": "ready",
        "process_creation_ns": 100,
        "readiness_ns": 1_000,
        "idle_rss_bytes": 1_000_000,
        "peak_rss_bytes": 2_000_000,
        "exit_status": 0,
        "external_access_attempts": 0,
        "corpus": {"status": "passed", "trace": "passed"},
    }
    schedule = {
        "schedule_id": schedule_id,
        "kind": "cold_measurement",
        "round": round_number,
        "language": language,
        "task": "not_applicable",
    }
    return record, package, schedule


def _refresh_index(root: Path, entries: list[dict[str, Any]]) -> None:
    index_path = root / "evidence-index.json"
    entries.sort(key=lambda item: item["path"])
    index = {
        "evidence_index_format": 1,
        "campaign_id": "synthetic-campaign",
        "configuration_id": CONFIGURATION_ID,
        "configuration_sha256": _sha256(EXPERIMENT_CONTRACT),
        "artifacts": entries,
    }
    _write_json(index_path, index)
    _write_json(
        root / "evidence-index.lock.json",
        {
            "evidence_index_lock_format": 1,
            "index_path": "evidence-index.json",
            "index_sha256": _sha256(index_path),
        },
    )


def _build_synthetic(
    root: Path, state: str, contract: dict[str, Any]
) -> list[dict[str, Any]]:
    campaign_id = "synthetic-campaign"
    entries: list[dict[str, Any]] = []
    trials: list[dict[str, Any]] = []
    warm: list[dict[str, Any]] = []
    cold: list[dict[str, Any]] = []
    ordering: list[dict[str, Any]] = []
    if state != "empty":
        selected = (
            [(LANGUAGES[0], TASKS[0])]
            if state == "pilot"
            else [(language, task) for language in LANGUAGES for task in TASKS]
        )
        if state == "partial":
            selected = selected[:4]
        for language, task in selected:
            trial, related, schedule = _synthetic_trial(
                root, contract, campaign_id, language, task
            )
            path = root / "records" / f"{trial['trial_id']}.json"
            _write_json(path, trial)
            entries.append(
                {
                    "record_id": f"trial-{trial['trial_id']}",
                    "kind": "agent_trial",
                    "path": path.relative_to(root).as_posix(),
                    "sha256": _sha256(path),
                }
            )
            entries.extend(related)
            trials.append(trial)
            ordering.extend(schedule)
        if state in {"partial", "complete"}:
            selected_languages = LANGUAGES[:2] if state == "partial" else LANGUAGES
            for language in selected_languages:
                warm_record, warm_blob, warm_schedule = _synthetic_warm(
                    root, campaign_id, language
                )
                warm_path = root / "records" / f"{warm_record['measurement_id']}.json"
                _write_json(warm_path, warm_record)
                entries.extend(
                    [
                        {
                            "record_id": f"warm-{warm_record['measurement_id']}",
                            "kind": "warm_measurement",
                            "path": warm_path.relative_to(root).as_posix(),
                            "sha256": _sha256(warm_path),
                        },
                        warm_blob,
                    ]
                )
                warm.append(warm_record)
                ordering.append(warm_schedule)
                cold_record, cold_blob, cold_schedule = _synthetic_cold(
                    root, campaign_id, language
                )
                cold_path = root / "records" / f"{cold_record['measurement_id']}.json"
                _write_json(cold_path, cold_record)
                entries.extend(
                    [
                        {
                            "record_id": f"cold-{cold_record['measurement_id']}",
                            "kind": "cold_measurement",
                            "path": cold_path.relative_to(root).as_posix(),
                            "sha256": _sha256(cold_path),
                        },
                        cold_blob,
                    ]
                )
                cold.append(cold_record)
                ordering.append(cold_schedule)
    completion_state = state
    campaign = {
        "campaign_format": 1,
        "campaign_id": campaign_id,
        "profile": "pilot" if state == "pilot" else "synthetic",
        "completion_state": completion_state,
        "configuration": {
            "id": CONFIGURATION_ID,
            "path": "benchmarks/calibration/experiment-contract.json",
            "sha256": _sha256(EXPERIMENT_CONTRACT),
        },
        "requirements": SYNTHETIC_REQUIREMENTS,
        "ordering": ordering,
    }
    campaign_path = root / "campaign.json"
    _write_json(campaign_path, campaign)
    entries.append(
        {
            "record_id": "campaign",
            "kind": "campaign",
            "path": "campaign.json",
            "sha256": _sha256(campaign_path),
        }
    )
    report = _derive_report(campaign, trials, warm, cold)
    report_path = root / "report.json"
    _write_json(report_path, report)
    entries.append(
        {
            "record_id": "report",
            "kind": "report",
            "path": "report.json",
            "sha256": _sha256(report_path),
        }
    )
    _refresh_index(root, entries)
    return entries


def _replace_record(
    root: Path,
    entries: list[dict[str, Any]],
    record_id: str,
    value: dict[str, Any],
) -> None:
    entry = next(item for item in entries if item["record_id"] == record_id)
    path = root / entry["path"]
    _write_json(path, value)
    entry["sha256"] = _sha256(path)


def _refresh_synthetic_report(root: Path, entries: list[dict[str, Any]]) -> None:
    campaign = _load_object(root / "campaign.json", "synthetic_invalid")
    trials = [
        _load_object(root / entry["path"], "synthetic_invalid")
        for entry in entries
        if entry["kind"] == "agent_trial"
    ]
    warm = [
        _load_object(root / entry["path"], "synthetic_invalid")
        for entry in entries
        if entry["kind"] == "warm_measurement"
    ]
    cold = [
        _load_object(root / entry["path"], "synthetic_invalid")
        for entry in entries
        if entry["kind"] == "cold_measurement"
    ]
    _replace_record(
        root,
        entries,
        "report",
        _derive_report(campaign, trials, warm, cold),
    )


def _apply_synthetic_mutation(
    root: Path,
    entries: list[dict[str, Any]],
    mutation: str,
) -> None:
    trial_entries = [entry for entry in entries if entry["kind"] == "agent_trial"]
    first_trial_entry = trial_entries[0]
    first_trial = _load_object(root / first_trial_entry["path"], "synthetic_invalid")
    if mutation == "missing_counts":
        first_trial["terminal"] = {
            "class": "failed",
            "cause": "synthetic_incomplete",
        }
        _replace_record(root, entries, first_trial_entry["record_id"], first_trial)
        _refresh_synthetic_report(root, entries)
    elif mutation == "duplicate_trial_identity":
        second = _load_object(root / trial_entries[1]["path"], "synthetic_invalid")
        second["trial_id"] = first_trial["trial_id"]
        _replace_record(root, entries, trial_entries[1]["record_id"], second)
    elif mutation == "changed_inputs":
        campaign = _load_object(root / "campaign.json", "synthetic_invalid")
        campaign["configuration"]["sha256"] = "0" * 64
        _replace_record(root, entries, "campaign", campaign)
    elif mutation == "invalid_hash":
        first_trial_entry["sha256"] = "0" * 64
    elif mutation == "incomplete_token_categories":
        del first_trial["token_accounting"]["requests"][0]["categories"][
            "other_tool_output"
        ]
        _replace_record(root, entries, first_trial_entry["record_id"], first_trial)
    elif mutation == "missing_raw_events":
        raw_id = first_trial["raw_events"]["record_id"]
        entries[:] = [entry for entry in entries if entry["record_id"] != raw_id]
    elif mutation == "unaccounted_exclusion":
        warm_entry = next(
            entry for entry in entries if entry["kind"] == "warm_measurement"
        )
        warm = _load_object(root / warm_entry["path"], "synthetic_invalid")
        warm["status"] = "excluded"
        warm["exclusion_reason"] = ""
        _replace_record(root, entries, warm_entry["record_id"], warm)
    elif mutation == "incorrect_summary":
        report = _load_object(root / "report.json", "synthetic_invalid")
        report["agent_distributions"][0]["successful"] = 99
        _replace_record(root, entries, "report", report)
    elif mutation == "mixed_configuration":
        first_trial["configuration_id"] = "another-configuration"
        _replace_record(root, entries, first_trial_entry["record_id"], first_trial)
    elif mutation == "malformed":
        path = root / first_trial_entry["path"]
        path.write_text("{not-json\n", encoding="utf-8")
        first_trial_entry["sha256"] = _sha256(path)
    else:
        _raise("synthetic_invalid", f"unknown mutation {mutation!r}")
    _refresh_index(root, entries)


def _synthetic_recipe(path: Path) -> dict[str, Any]:
    recipe = _load_object(path, "synthetic_invalid")
    _require_canonical(path, recipe, "synthetic_invalid")
    expected_keys = (
        "synthetic_fixture_format",
        "id",
        "state",
        "mutation",
        "expected",
    )
    _check_exact_keys(recipe, expected_keys, "synthetic_invalid", path.name)
    if recipe["synthetic_fixture_format"] != 1:
        _raise("synthetic_invalid", f"{path}: unsupported format")
    return recipe


def verify_synthetic_campaigns(
    contract: dict[str, Any] | None = None,
) -> list[tuple[str, str]]:
    contract = check_contract_lock() if contract is None else contract
    recipe_paths = [
        SYNTHETIC_ROOT / "empty.json",
        SYNTHETIC_ROOT / "pilot.json",
        SYNTHETIC_ROOT / "partial.json",
        SYNTHETIC_ROOT / "malformed.json",
        SYNTHETIC_ROOT / "complete.json",
    ]
    rejections = _load_object(SYNTHETIC_ROOT / "rejections.json", "synthetic_invalid")
    _require_canonical(
        SYNTHETIC_ROOT / "rejections.json", rejections, "synthetic_invalid"
    )
    recipes = [_synthetic_recipe(path) for path in recipe_paths]
    recipes.extend(rejections["scenarios"])
    outcomes: list[tuple[str, str]] = []
    for recipe in recipes:
        with tempfile.TemporaryDirectory(prefix="ail-m8b-synthetic-") as raw:
            root = Path(raw)
            state = recipe["state"]
            entries = _build_synthetic(root, state, contract)
            mutation = recipe["mutation"]
            if mutation != "none":
                _apply_synthetic_mutation(root, entries, mutation)
            expected = recipe["expected"]
            try:
                result = verify_campaign(
                    root / "evidence-index.json",
                    root / "evidence-index.lock.json",
                    contract=contract,
                )
            except CalibrationError as error:
                actual = error.code
            else:
                actual = f"accepted:{result.completion_state}"
            if actual != expected:
                _raise(
                    "synthetic_expectation_failed",
                    f"{recipe['id']}: expected {expected}, received {actual}",
                )
            outcomes.append((recipe["id"], actual))
    return outcomes


def verify_calibration() -> None:
    contract = check_contract_lock()
    outcomes = verify_synthetic_campaigns(contract)
    print(
        "M8 calibration verifier passed: "
        f"{len(LOCKED_ARTIFACTS)} contract artifacts and "
        f"{len(outcomes)} stable synthetic outcomes verified."
    )
