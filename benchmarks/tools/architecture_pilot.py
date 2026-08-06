#!/usr/bin/env python3
"""Verify the retained non-official M27 architecture-feedback pilot."""

from __future__ import annotations

import copy
import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, NoReturn

import fixtures as fixture_tool


ROOT = Path(__file__).resolve().parents[2]
PILOT_ROOT = ROOT / "benchmarks" / "architecture-pilot"
PILOT_MANIFEST = PILOT_ROOT / "pilot.json"
PILOT_LOCK = PILOT_ROOT / "pilot.lock.json"
CANDIDATES = ROOT / "specs" / "architecture-acceptance-fixtures" / "candidates.json"
RESULTS = ROOT / "specs" / "architecture-fixtures" / "results.json"
FIXTURE_SET_DIGEST = "ab362d96d89cbba779743dd8a3050b2bd4452ff6daddf3e7ae65109207f7e3ed"
PROMPT_DIGEST = "17810a023bae4104ea15f66bdd5a3395a7b5103996e7110d6c1fede25cd59c4e"
OPERATOR_REPORT_DIGEST = (
    "495634ae7ffdabb793b819a20a20496bdfe0788a9f30cf2b7bbb4cce6bb5855c"
)
REPAIRED_CANDIDATE_DIGEST = (
    "8bfd3f223b2b15f4eca72eba215e25450d61ded2cd22ae0c6d3223fa25df567d"
)
EXPECTED_OPERATOR = {
    "agent": "Amp",
    "version": "0.0.1785901465-g80049c",
    "mode": "medium",
    "executor": "Amp orb",
    "project": "segfault/ail",
    "working_directory": "/home/user/workspace/repo",
    "thread_url": (
        "https://ampcode.com/threads/T-019fd051-5244-724a-a492-c766d9944ccc"
    ),
    "task_thread_url": (
        "https://ampcode.com/threads/T-019fd049-ddc4-74e3-b3dc-0b1e92d11e7f"
    ),
    "original_output_paths": [
        "/home/user/workspace/repo/m27-repaired-candidate.json",
        "/home/user/workspace/repo/m27-operator-report.json",
    ],
    "permissions": [
        "read-repository",
        "write-pilot-candidate",
        "run-local-checks",
    ],
    "restrictions": [
        "work alone in the checked-out repository",
        "use only finder, shell commands, and apply_patch",
        "do not use the network, delegate, install anything, or modify tracked files",
        "do not inspect the valid or helper-split candidate before final comparison",
        "do not change the repaired candidate after final comparison",
    ],
    "authorization_changes": [
        {
            "restriction": "do not use the network or install anything",
            "user_instruction": "Ah - do it. complete the task in full.",
            "command": (
                "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | "
                "sh -s -- -y --profile minimal --default-toolchain 1.87.0"
            ),
            "effect": (
                "Installed Rust 1.87.0 after the first Cargo attempt failed, "
                "then ran the required focused test."
            ),
        }
    ],
    "tools": ["finder", "shell_command", "apply_patch"],
}
EXPECTED_VALIDATIONS = (
    ("initial-policy-evaluator-path-error", "failed-and-corrected"),
    ("local-policy-evaluation", "passed"),
    ("focused-test-before-toolchain-install", "partially-passed-then-blocked"),
    ("final-candidate-comparison", "comparison-recorded-with-follow-up"),
    ("result-key-and-environment-inspection", "passed"),
    ("completion-field-inspection", "passed"),
    ("canonical-artifact-and-worktree-check", "passed"),
    ("pre-install-candidate-integrity", "passed"),
    ("focused-test-after-authorized-install", "passed"),
    ("final-report-and-candidate-integrity", "passed"),
)
REDUNDANT_EDGE = [
    "domain:handle:job.cancel",
    "contract:job.cancel",
    "type-use",
]
EXPECTED_DIFFERENCES = [
    "The repaired candidate retains an empty changed_units field.",
    "The repaired candidate retains one redundant domain-to-contract type-use edge.",
]


@dataclass(frozen=True)
class ArchitecturePilotError(Exception):
    code: str
    message: str

    def __str__(self) -> str:
        return f"{self.code}: {self.message}"


def _raise(code: str, message: str) -> NoReturn:
    raise ArchitecturePilotError(code, message)


def _canonical(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _load_object(path: Path, *, canonical: bool = False) -> dict[str, Any]:
    try:
        text = path.read_text(encoding="utf-8")
        value = json.loads(text)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        _raise("architecture_pilot_invalid", f"{path}: {error}")
    if not isinstance(value, dict):
        _raise("architecture_pilot_invalid", f"{path}: must contain a JSON object")
    if canonical and text != _canonical(value):
        _raise(
            "architecture_pilot_noncanonical",
            f"{path}: must be canonical two-space JSON",
        )
    return value


def _validate_schema(value: dict[str, Any], schema_path: Path) -> None:
    schema = fixture_tool._load_json(schema_path)
    problems = fixture_tool._schema_errors(value, schema, schema, "$")
    if problems:
        rendered = "; ".join(str(problem) for problem in problems[:8])
        _raise("architecture_pilot_invalid", rendered)


def _load_locked_manifest(manifest_path: Path, lock_path: Path) -> dict[str, Any]:
    lock = _load_object(lock_path, canonical=True)
    if set(lock) != {"lock_format", "manifest_path", "manifest_sha256"}:
        _raise("architecture_pilot_invalid", "pilot lock is malformed")
    if lock["lock_format"] != 1 or lock["manifest_path"] != manifest_path.name:
        _raise("architecture_pilot_invalid", "pilot lock identity differs")
    if lock["manifest_sha256"] != _sha256(manifest_path):
        _raise("architecture_pilot_changed", "pilot manifest digest differs")
    return _load_object(manifest_path, canonical=True)


def _artifact(root: Path, item: Any, name: str) -> Path:
    if not isinstance(item, dict) or set(item) != {"path", "sha256"}:
        _raise("architecture_pilot_invalid", f"{name} artifact is malformed")
    relative = item["path"]
    if not isinstance(relative, str) or not relative or Path(relative).is_absolute():
        _raise("architecture_pilot_invalid", f"{name} path is invalid")
    path = (root / relative).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError:
        _raise("architecture_pilot_invalid", f"{name} path leaves the pilot directory")
    if not path.is_file():
        _raise("architecture_pilot_invalid", f"{name} artifact is missing")
    if not isinstance(item["sha256"], str) or item["sha256"] != _sha256(path):
        _raise("architecture_pilot_changed", f"{name} artifact digest differs")
    return path


def _operation(results: dict[str, Any], candidate: str) -> dict[str, Any]:
    operations = results.get("operations")
    if not isinstance(operations, list):
        _raise("architecture_pilot_invalid", "architecture results are malformed")
    matches = [
        item
        for item in operations
        if isinstance(item, dict) and item.get("candidate") == candidate
    ]
    if len(matches) != 1 or matches[0].get("operation") != "validate_architecture_change":
        _raise("architecture_pilot_invalid", f"missing locked {candidate} result")
    response = matches[0].get("expected_response")
    if not isinstance(response, dict):
        _raise("architecture_pilot_invalid", f"locked {candidate} result is malformed")
    return response


def _candidate(candidates: dict[str, Any], candidate: str) -> dict[str, Any]:
    values = candidates.get("candidates")
    if not isinstance(values, list):
        _raise("architecture_pilot_invalid", "architecture candidates are malformed")
    matches = [
        item
        for item in values
        if isinstance(item, dict) and item.get("id") == candidate
    ]
    if len(matches) != 1:
        _raise("architecture_pilot_invalid", f"missing locked {candidate} candidate")
    return matches[0]


def _check_report(
    report: dict[str, Any],
    initial: dict[str, Any],
    final: dict[str, Any],
) -> None:
    expected_initial = {
        "candidate": "centralized",
        "status": initial["status"],
        "behavior": "passed 6/6",
        "current_revision_id": initial["current_revision_id"],
        "published_child_revision_id": initial["published_child_revision_id"],
        "snapshot_compact": initial["snapshot"]["compact"],
        "delta_compact": initial["delta"]["compact"],
    }
    if report.get("initial_attempt") != expected_initial:
        _raise("architecture_pilot_output_changed", "initial compact output differs")
    expected_final = {
        "candidate": "valid",
        "status": final["status"],
        "behavior": "passed 6/6",
        "published_child_revision_id": final["completion"]["revision_id"],
        "snapshot_compact": final["snapshot"]["compact"],
        "delta_compact": final["delta"]["compact"],
    }
    if report.get("final_validation") != expected_final:
        _raise("architecture_pilot_output_changed", "final compact output differs")
    if report.get("report_format") != 1 or report.get("task") != (
        "Add CancelJob without moving domain authority into transport"
    ):
        _raise("architecture_pilot_invalid", "operator report identity differs")
    actions = report.get("actions")
    if not isinstance(actions, list) or [
        item.get("kind") if isinstance(item, dict) else None for item in actions
    ] != ["inspect", "inspect", "edit", "validate"]:
        _raise("architecture_pilot_invalid", "operator action sequence is incomplete")
    if any(
        set(item) != {"kind", "detail"}
        or not isinstance(item["detail"], str)
        or not item["detail"]
        for item in actions
    ):
        _raise("architecture_pilot_invalid", "operator action is malformed")
    if not isinstance(report.get("repair"), str) or not report["repair"]:
        _raise("architecture_pilot_invalid", "repair explanation is missing")
    limitations = report.get("limitations")
    if not isinstance(limitations, list) or not limitations:
        _raise("architecture_pilot_invalid", "operator limitations are missing")


def _check_inspection(manifest: dict[str, Any], initial: dict[str, Any]) -> None:
    findings = initial.get("snapshot", {}).get("findings")
    if not isinstance(findings, list):
        _raise("architecture_pilot_invalid", "locked centralized findings are missing")
    expected = [
        {
            "code": finding["code"],
            "scope": finding["scope"],
            "contributors": finding["contributors"],
        }
        for finding in findings
    ]
    if manifest.get("initial_inspection") != {
        "compact_source": "locked centralized expected response",
        "structured_findings": expected,
    }:
        _raise(
            "architecture_pilot_evidence_changed",
            "structured diagnostic evidence differs",
        )


def _check_validation_log(manifest: dict[str, Any]) -> None:
    log = manifest.get("validation_log")
    if not isinstance(log, list):
        _raise("architecture_pilot_invalid", "validation log is missing")
    observed = tuple(
        (item.get("id"), item.get("outcome"))
        for item in log
        if isinstance(item, dict)
    )
    if observed != EXPECTED_VALIDATIONS or len(observed) != len(log):
        _raise("architecture_pilot_invalid", "validation log is incomplete or reordered")
    if "passed 2 tests with 0 failures" not in log[-2]["result"]:
        _raise("architecture_pilot_invalid", "final focused test result is missing")


def _check_candidate(repaired: dict[str, Any], valid: dict[str, Any]) -> None:
    observed = repaired.get("observed_results")
    if observed != valid.get("observed_results") or not isinstance(observed, list):
        _raise("architecture_pilot_behavior_changed", "six-case behavior evidence differs")
    if len(observed) != 6 or len({item.get("id") for item in observed}) != 6:
        _raise("architecture_pilot_behavior_changed", "behavior evidence is not 6/6")

    operation = repaired.get("operation")
    if not isinstance(operation, dict):
        _raise("architecture_pilot_invalid", "repair operation is missing")
    owner = "domain:handle:job.cancel"
    if operation.get("implementation_owner") != owner:
        _raise("architecture_pilot_ownership_changed", "cancellation is not domain-owned")

    units = repaired.get("added_units")
    if not isinstance(units, list) or any(not isinstance(item, dict) for item in units):
        _raise("architecture_pilot_invalid", "repair units are malformed")
    by_id = {item.get("id"): item for item in units}
    if len(by_id) != len(units) or owner not in by_id:
        _raise("architecture_pilot_invalid", "domain repair unit is missing")
    domain = by_id[owner]
    if (
        domain.get("group") != "domain"
        or domain.get("capabilities") != ["jobs_store"]
        or domain.get("state_reads") != ["jobs"]
        or domain.get("state_writes") != ["jobs"]
    ):
        _raise(
            "architecture_pilot_ownership_changed",
            "jobs-store or jobs-state authority is not domain-owned",
        )
    transport_units = [item for item in units if item.get("group") == "transport"]
    if not transport_units or any(
        item.get("capabilities") != []
        or item.get("state_reads") != []
        or item.get("state_writes") != []
        for item in transport_units
    ):
        _raise(
            "architecture_pilot_transport_owned",
            "transport acquired capability or state authority",
        )
    changed_units = repaired.get("changed_units", [])
    if not isinstance(changed_units, list) or any(
        isinstance(item, dict)
        and (item.get("group") == "transport" or item.get("id") == "transport:dispatch")
        for item in changed_units
    ):
        _raise("architecture_pilot_transport_owned", "repair changes transport authority")

    edges = repaired.get("added_edges")
    if not isinstance(edges, list) or any(
        not isinstance(edge, list)
        or len(edge) != 3
        or any(not isinstance(value, str) for value in edge)
        for edge in edges
    ):
        _raise("architecture_pilot_invalid", "repair edges are malformed")
    required = [
        ["transport:adapt:job.cancel", owner, "calls"],
        [owner, "capability:jobs_store.cancel_if_active", "capability-use"],
        [owner, "state:jobs", "state-read"],
        [owner, "state:jobs", "state-write"],
    ]
    if any(edge not in edges for edge in required):
        _raise("architecture_pilot_ownership_changed", "domain ownership edges are missing")
    if any(
        edge[0].startswith("transport:")
        and (edge[1].startswith("capability:") or edge[1].startswith("state:"))
        for edge in edges
    ):
        _raise(
            "architecture_pilot_transport_owned",
            "transport directly accesses jobs-store authority or jobs state",
        )
    adapter_edges = [edge for edge in edges if edge[0] == "transport:adapt:job.cancel"]
    if adapter_edges != [["transport:adapt:job.cancel", owner, "calls"]]:
        _raise("architecture_pilot_transport_owned", "transport is not only an adapter")


def _check_comparison(
    comparison: Any,
    repaired: dict[str, Any],
    valid: dict[str, Any],
    candidate_digest: str,
) -> None:
    if not isinstance(comparison, dict):
        _raise("architecture_pilot_invalid", "final comparison is missing")
    if (
        comparison.get("candidate_sha256_at_comparison") != candidate_digest
        or comparison.get("candidate_sha256_after_comparison") != candidate_digest
        or comparison.get("candidate_changed_after_comparison") is not False
    ):
        _raise(
            "architecture_pilot_post_comparison_change",
            "candidate integrity after final comparison is not accounted for",
        )
    if comparison.get("matches_locked_valid") != (repaired == valid):
        _raise("architecture_pilot_comparison_changed", "comparison result is incorrect")
    if comparison.get("differences") != EXPECTED_DIFFERENCES:
        _raise("architecture_pilot_comparison_changed", "comparison differences differ")

    normalized = copy.deepcopy(repaired)
    if normalized.pop("changed_units", None) != []:
        _raise("architecture_pilot_comparison_changed", "empty-field difference differs")
    edges = normalized.get("added_edges")
    if not isinstance(edges, list) or edges.count(REDUNDANT_EDGE) != 1:
        _raise("architecture_pilot_comparison_changed", "redundant-edge difference differs")
    edges.remove(REDUNDANT_EDGE)
    if normalized != valid:
        _raise(
            "architecture_pilot_comparison_changed",
            "unrecorded differences from the locked valid candidate remain",
        )


def verify_architecture_pilot(
    manifest_path: Path = PILOT_MANIFEST,
    *,
    lock_path: Path | None = None,
    root: Path = ROOT,
) -> dict[str, Any]:
    lock_path = manifest_path.with_name("pilot.lock.json") if lock_path is None else lock_path
    manifest = _load_locked_manifest(manifest_path, lock_path)
    schema_path = root / "benchmarks" / "schemas" / "architecture-pilot.schema.json"
    _validate_schema(manifest, schema_path)
    if manifest.get("fixture_set_digest") != FIXTURE_SET_DIGEST:
        _raise("architecture_pilot_changed", "accepted fixture-set digest differs")
    if manifest.get("operator") != EXPECTED_OPERATOR:
        _raise("architecture_pilot_invalid", "operator configuration differs")

    artifacts = manifest.get("artifacts")
    if not isinstance(artifacts, dict):
        _raise("architecture_pilot_invalid", "pilot artifact index is missing")
    pilot_root = manifest_path.parent
    prompt_path = _artifact(pilot_root, artifacts.get("prompt"), "prompt")
    report_path = _artifact(
        pilot_root, artifacts.get("operator_report"), "operator report"
    )
    repaired_path = _artifact(
        pilot_root, artifacts.get("repaired_candidate"), "repaired candidate"
    )
    if _sha256(prompt_path) != PROMPT_DIGEST:
        _raise("architecture_pilot_changed", "pilot prompt differs")
    if _sha256(report_path) != OPERATOR_REPORT_DIGEST:
        _raise("architecture_pilot_changed", "operator report differs")
    candidate_digest = _sha256(repaired_path)
    if candidate_digest != REPAIRED_CANDIDATE_DIGEST:
        _raise("architecture_pilot_changed", "retained repair differs")

    report = _load_object(report_path, canonical=True)
    repaired = _load_object(repaired_path, canonical=True)
    candidates = _load_object(
        root / "specs" / "architecture-acceptance-fixtures" / "candidates.json"
    )
    results = _load_object(root / "specs" / "architecture-fixtures" / "results.json")
    if results.get("accepted_input_fixture_set_digest") != FIXTURE_SET_DIGEST:
        _raise("architecture_pilot_changed", "compiler results use another fixture set")

    initial = _operation(results, "centralized")
    final = _operation(results, "valid")
    valid = _candidate(candidates, "valid")
    _check_report(report, initial, final)
    _check_inspection(manifest, initial)
    _check_validation_log(manifest)
    _check_candidate(repaired, valid)
    _check_comparison(
        manifest.get("final_comparison"), repaired, valid, candidate_digest
    )
    limitations = manifest.get("limitations")
    if not isinstance(limitations, list) or len(limitations) < 3:
        _raise("architecture_pilot_invalid", "concrete pilot limitations are missing")
    return manifest
