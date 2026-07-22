#!/usr/bin/env python3
"""Verify the M11 five-construct language and protocol contract."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any, NoReturn


ROOT = Path(__file__).resolve().parents[2]
SPECS = ROOT / "specs"
CONTRACT_PATH = SPECS / "core-contract.json"
PROTOCOL_PATH = SPECS / "protocol.json"
RUNTIME_CONTRACT_PATH = SPECS / "runtime-contract.json"
RUNTIME_PROTOCOL_PATH = SPECS / "runtime-protocol.json"
REQUIREMENTS_PATH = ROOT / "docs" / "requirements" / "reference-slice.md"

EXPECTED_CONSTRUCTS = (
    "record",
    "variant",
    "function",
    "let",
    "capability-call",
)
EXPECTED_CATEGORIES = (
    "positive",
    "formatting",
    "recovery",
    "type-error",
    "capability-error",
    "rename",
    "stale-revision",
)
EXPECTED_DIAGNOSTICS = {
    "recovery": "AIL.PARSE.EXPECTED_TOKEN",
    "type-error": "AIL.TYPE.FIELD_MISMATCH",
    "capability-error": "AIL.CAPABILITY.UNDECLARED_EFFECT",
    "stale-revision": "AIL.PROTOCOL.STALE_REVISION",
}
REQUIRED_PROTOCOL_SHAPES = {
    "Revision",
    "Handle",
    "Diagnostic",
    "InspectionRequest",
    "InspectionResult",
    "RenameRequest",
    "RenameSuccess",
    "RenameFailure",
    "IdentityMap",
}
REQUIRED_FIXTURE_KEYS = {
    "fixture_format",
    "id",
    "category",
    "requirements",
    "constructs",
    "rules",
    "input",
    "expected",
}
REQUIRED_EXPECTED_KEYS = {
    "canonical_source",
    "type_result",
    "primary_diagnostic",
    "protocol_result",
}
REQUIRED_DIAGNOSTIC_FIELDS = {
    "code",
    "revision_id",
    "category",
    "primary_handle",
    "expected",
    "actual",
    "related_handles",
    "causal_chain",
}
RUNTIME_PROTOCOL_SHAPES = {
    "RuntimeValue",
    "RuntimeFault",
    "ObservedCapabilityCall",
    "ExecutionRequest",
    "ExecutionSuccess",
    "ExecutionFailure",
}
RUNTIME_DIAGNOSTICS = {
    "non-exhaustive-match": "AIL.TYPE.NON_EXHAUSTIVE_MATCH",
    "if-branch-mismatch": "AIL.TYPE.IF_BRANCH_MISMATCH",
    "match-arm-mismatch": "AIL.TYPE.MATCH_ARM_MISMATCH",
}
RUNTIME_CASES = (
    "invalid-zero-calls",
    "inserted",
    "duplicate",
    "unavailable-before-commit",
    "v1-request-priority-adaptation",
    "v1-stored-job-adaptation",
    "revision-scoped-retained-parent",
    "stable-malformed-argument-fault",
)


class ContractError(Exception):
    """A stable contract verification failure."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message


def _raise(code: str, message: str) -> NoReturn:
    raise ContractError(code, message)


def _require(condition: bool, code: str, message: str) -> None:
    if not condition:
        _raise(code, message)


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            _raise("json_duplicate_key", f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def _load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_unique_object,
        )
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        _raise("json_invalid", f"{path.relative_to(ROOT)}: {error}")
    _require(isinstance(value, dict), "json_invalid", f"{path}: expected object")
    _require(
        path.read_bytes().endswith(b"\n"),
        "json_final_newline",
        f"{path.relative_to(ROOT)}: missing final newline",
    )
    return value


def _string_list(
    value: Any, code: str, field: str, *, allow_empty: bool = True
) -> list[str]:
    _require(isinstance(value, list), code, f"{field}: expected list")
    _require(
        all(isinstance(item, str) and item for item in value),
        code,
        f"{field}: expected non-empty strings",
    )
    if not allow_empty:
        _require(bool(value), code, f"{field}: must not be empty")
    _require(len(value) == len(set(value)), code, f"{field}: duplicate values")
    return value


def _accepted_requirements() -> set[str]:
    text = REQUIREMENTS_PATH.read_text(encoding="utf-8")
    headings = list(re.finditer(r"^### ([A-Z]+-\d{3}) — ", text, re.MULTILINE))
    accepted: set[str] = set()
    for index, match in enumerate(headings):
        end = headings[index + 1].start() if index + 1 < len(headings) else len(text)
        section = text[match.end() : end]
        if "Status: **Accepted" in section:
            accepted.add(match.group(1))
    _require(accepted, "requirements_missing", "no accepted requirements found")
    return accepted


def _validate_rule_documents(rule_ids: set[str]) -> None:
    core = (SPECS / "core.md").read_text(encoding="utf-8")
    protocol = (SPECS / "protocol.md").read_text(encoding="utf-8")
    documented = set(
        re.findall(
            r"^### (M11-(?:LANG|PROTO)-\d{3}) — ", core + "\n" + protocol, re.MULTILINE
        )
    )
    _require(
        documented == rule_ids,
        "rule_document_mismatch",
        f"manifest-only={sorted(rule_ids - documented)}, document-only={sorted(documented - rule_ids)}",
    )
    for rule_id in sorted(rule_ids):
        count = len(
            re.findall(
                rf"^### {re.escape(rule_id)} — ", core + "\n" + protocol, re.MULTILINE
            )
        )
        _require(
            count == 1,
            "rule_document_duplicate",
            f"{rule_id}: documented {count} times",
        )


def _validate_handle(
    value: Any, revision_id: str, handle_kinds: set[str], field: str
) -> None:
    _require(isinstance(value, dict), "handle_invalid", f"{field}: expected object")
    _require(
        set(value) == {"revision_id", "kind", "local_id"},
        "handle_invalid",
        f"{field}: fields must match Handle",
    )
    _require(
        value["revision_id"] == revision_id,
        "handle_revision_mismatch",
        f"{field}: expected {revision_id}",
    )
    _require(
        value["kind"] in handle_kinds,
        "handle_kind_invalid",
        f"{field}: {value['kind']!r}",
    )
    _require(
        isinstance(value["local_id"], str) and value["local_id"],
        "handle_invalid",
        f"{field}: empty local_id",
    )


def _validate_diagnostic(
    value: Any, revision_id: str, handle_kinds: set[str], field: str
) -> None:
    _require(isinstance(value, dict), "diagnostic_invalid", f"{field}: expected object")
    missing = REQUIRED_DIAGNOSTIC_FIELDS - set(value)
    _require(
        not missing, "diagnostic_missing_field", f"{field}: missing {sorted(missing)}"
    )
    _require(
        value["revision_id"] == revision_id,
        "diagnostic_revision_mismatch",
        f"{field}: expected {revision_id}",
    )
    _require(
        isinstance(value["code"], str) and value["code"].startswith("AIL."),
        "diagnostic_code_invalid",
        field,
    )
    _require(
        isinstance(value["category"], str) and value["category"],
        "diagnostic_category_invalid",
        field,
    )
    _validate_handle(
        value["primary_handle"], revision_id, handle_kinds, f"{field}.primary_handle"
    )
    _require(isinstance(value["expected"], dict), "diagnostic_expected_invalid", field)
    _require(isinstance(value["actual"], dict), "diagnostic_actual_invalid", field)
    _require(
        isinstance(value["related_handles"], list), "diagnostic_related_invalid", field
    )
    for index, handle in enumerate(value["related_handles"]):
        _validate_handle(
            handle, revision_id, handle_kinds, f"{field}.related_handles[{index}]"
        )
    _require(
        isinstance(value["causal_chain"], list) and value["causal_chain"],
        "diagnostic_cause_invalid",
        field,
    )


def _validate_type_result(
    value: Any, revision_id: str, handle_kinds: set[str], field: str
) -> None:
    _require(
        isinstance(value, dict), "type_result_invalid", f"{field}: expected object"
    )
    _require(
        set(value) == {"status", "facts"}, "type_result_invalid", f"{field}: fields"
    )
    _require(
        value["status"] in {"ok", "error", "not-run"},
        "type_status_invalid",
        f"{field}: {value['status']!r}",
    )
    _require(isinstance(value["facts"], list), "type_facts_invalid", field)
    if value["status"] == "not-run":
        _require(
            not value["facts"],
            "type_facts_invalid",
            f"{field}: not-run must have no facts",
        )
    for index, fact in enumerate(value["facts"]):
        _require(isinstance(fact, dict), "type_fact_invalid", f"{field}.facts[{index}]")
        _require(
            set(fact) == {"handle", "explicit_type", "inferred_type"},
            "type_fact_invalid",
            f"{field}.facts[{index}]: fields",
        )
        _validate_handle(
            fact["handle"], revision_id, handle_kinds, f"{field}.facts[{index}].handle"
        )
        _require(
            fact["explicit_type"] is not None or fact["inferred_type"] is not None,
            "type_fact_empty",
            f"{field}.facts[{index}]",
        )


def _validate_source_digest(revision: dict[str, Any], source: str, field: str) -> None:
    expected = "sha256:" + hashlib.sha256(source.encode("utf-8")).hexdigest()
    _require(
        revision.get("source_digest") == expected,
        "source_digest_mismatch",
        f"{field}: expected {expected}",
    )


def _apply_edits(source: str, edits: list[dict[str, Any]], path: str) -> str:
    encoded = source.encode("utf-8")
    previous_end = -1
    for index, edit in enumerate(edits):
        _require(isinstance(edit, dict), "rename_edit_invalid", f"edits[{index}]")
        _require(
            set(edit) == {"path", "start_utf8_byte", "end_utf8_byte", "replacement"},
            "rename_edit_invalid",
            f"edits[{index}]: fields",
        )
        _require(edit["path"] == path, "rename_edit_path", f"edits[{index}]")
        start = edit["start_utf8_byte"]
        end = edit["end_utf8_byte"]
        _require(
            isinstance(start, int)
            and isinstance(end, int)
            and 0 <= start < end <= len(encoded),
            "rename_edit_range",
            f"edits[{index}]",
        )
        _require(start >= previous_end, "rename_edit_order", f"edits[{index}]")
        _require(
            isinstance(edit["replacement"], str),
            "rename_edit_invalid",
            f"edits[{index}].replacement",
        )
        previous_end = end
    for edit in reversed(edits):
        start = edit["start_utf8_byte"]
        end = edit["end_utf8_byte"]
        encoded = encoded[:start] + edit["replacement"].encode("utf-8") + encoded[end:]
    try:
        return encoded.decode("utf-8")
    except UnicodeDecodeError as error:
        _raise("rename_edit_utf8", str(error))


def _validate_inspection_result(
    value: Any, revision_id: str, handle_kinds: set[str], required: set[str], field: str
) -> None:
    _require(isinstance(value, dict), "inspection_invalid", field)
    _require(
        required <= set(value),
        "inspection_missing_field",
        f"{field}: {sorted(required - set(value))}",
    )
    _require(value["revision_id"] == revision_id, "inspection_revision_mismatch", field)
    _validate_handle(value["handle"], revision_id, handle_kinds, f"{field}.handle")
    for list_field in ("effects", "capabilities", "dependencies"):
        _string_list(
            value[list_field], "inspection_list_invalid", f"{field}.{list_field}"
        )


def _validate_identity_map(
    value: Any,
    from_revision: str,
    to_revision: str,
    handle_kinds: set[str],
    classifications: set[str],
) -> None:
    _require(
        isinstance(value, dict), "identity_map_invalid", "identity_map: expected object"
    )
    _require(
        set(value) == {"from_revision_id", "to_revision_id", "entries", "new_handles"},
        "identity_map_invalid",
        "identity_map: fields",
    )
    _require(
        value["from_revision_id"] == from_revision,
        "identity_map_revision",
        "from revision",
    )
    _require(
        value["to_revision_id"] == to_revision, "identity_map_revision", "to revision"
    )
    _require(
        isinstance(value["entries"], list) and value["entries"],
        "identity_map_entries",
        "entries",
    )
    seen: set[tuple[str, str]] = set()
    used_classifications: set[str] = set()
    for index, entry in enumerate(value["entries"]):
        _require(isinstance(entry, dict), "identity_entry_invalid", f"entries[{index}]")
        _require(
            "old_handle" in entry and "classification" in entry,
            "identity_entry_invalid",
            f"entries[{index}]",
        )
        _validate_handle(
            entry["old_handle"],
            from_revision,
            handle_kinds,
            f"entries[{index}].old_handle",
        )
        key = (entry["old_handle"]["kind"], entry["old_handle"]["local_id"])
        _require(key not in seen, "identity_entry_duplicate", f"entries[{index}]")
        seen.add(key)
        classification = entry["classification"]
        _require(
            classification in classifications,
            "identity_classification_invalid",
            f"entries[{index}]",
        )
        used_classifications.add(classification)
        if classification in {"surviving", "replaced"}:
            _require(
                "new_handle" in entry,
                "identity_new_handle_missing",
                f"entries[{index}]",
            )
            _validate_handle(
                entry["new_handle"],
                to_revision,
                handle_kinds,
                f"entries[{index}].new_handle",
            )
        else:
            _require(
                "new_handle" not in entry,
                "identity_new_handle_forbidden",
                f"entries[{index}]",
            )
    _require(
        {"surviving", "replaced"} <= used_classifications,
        "identity_rename_coverage",
        "rename must show surviving and replaced",
    )
    _require(
        isinstance(value["new_handles"], list),
        "identity_new_handles_invalid",
        "new_handles",
    )
    for index, handle in enumerate(value["new_handles"]):
        _validate_handle(handle, to_revision, handle_kinds, f"new_handles[{index}]")


def _validate_fixture(
    path: Path,
    fixture: dict[str, Any],
    accepted_requirements: set[str],
    construct_ids: set[str],
    rule_by_id: dict[str, dict[str, Any]],
    protocol: dict[str, Any],
) -> None:
    label = path.relative_to(ROOT).as_posix()
    _require(
        set(fixture) == REQUIRED_FIXTURE_KEYS, "fixture_fields", f"{label}: fields"
    )
    _require(fixture["fixture_format"] == 1, "fixture_format", label)
    _require(isinstance(fixture["id"], str) and fixture["id"], "fixture_id", label)
    _require(fixture["category"] in EXPECTED_CATEGORIES, "fixture_category", label)
    requirements = _string_list(
        fixture["requirements"],
        "fixture_requirements",
        f"{label}.requirements",
        allow_empty=False,
    )
    _require(
        set(requirements) <= accepted_requirements,
        "fixture_requirement_unknown",
        f"{label}: {sorted(set(requirements) - accepted_requirements)}",
    )
    constructs = _string_list(
        fixture["constructs"],
        "fixture_constructs",
        f"{label}.constructs",
        allow_empty=False,
    )
    _require(
        set(constructs) <= construct_ids,
        "fixture_construct_unknown",
        f"{label}: {sorted(set(constructs) - construct_ids)}",
    )
    rules = _string_list(
        fixture["rules"], "fixture_rules", f"{label}.rules", allow_empty=False
    )
    _require(
        set(rules) <= set(rule_by_id),
        "fixture_rule_unknown",
        f"{label}: {sorted(set(rules) - set(rule_by_id))}",
    )

    input_value = fixture["input"]
    _require(isinstance(input_value, dict), "fixture_input", label)
    _require(
        {"revision", "path", "source", "environment"} <= set(input_value),
        "fixture_input",
        f"{label}: fields",
    )
    revision = input_value["revision"]
    _require(isinstance(revision, dict), "revision_invalid", label)
    _require(
        set(revision)
        in (
            {"workspace_id", "revision_id", "source_digest"},
            {"workspace_id", "revision_id", "parent_revision_id", "source_digest"},
        ),
        "revision_invalid",
        f"{label}: fields",
    )
    revision_id = revision["revision_id"]
    _require(isinstance(revision_id, str) and revision_id, "revision_invalid", label)
    source = input_value["source"]
    _require(
        isinstance(source, str) and source.endswith("\n"),
        "fixture_source",
        f"{label}: source must end in newline",
    )
    _require(
        isinstance(input_value["path"], str) and input_value["path"],
        "fixture_path",
        label,
    )
    _require(isinstance(input_value["environment"], dict), "fixture_environment", label)
    _validate_source_digest(revision, source, f"{label}.input.revision")

    expected = fixture["expected"]
    _require(isinstance(expected, dict), "fixture_expected", label)
    _require(
        set(expected) == REQUIRED_EXPECTED_KEYS,
        "fixture_expected_fields",
        f"{label}: fields",
    )
    canonical = expected["canonical_source"]
    _require(
        canonical is None or (isinstance(canonical, str) and canonical.endswith("\n")),
        "canonical_source_invalid",
        label,
    )

    handle_kinds = set(protocol["handle_kinds"])
    type_revision_id = revision_id
    if fixture["category"] == "rename":
        rename_result = expected.get("protocol_result")
        if isinstance(rename_result, dict) and isinstance(
            rename_result.get("revision"), dict
        ):
            candidate_revision_id = rename_result["revision"].get("revision_id")
            if isinstance(candidate_revision_id, str):
                type_revision_id = candidate_revision_id
    _validate_type_result(
        expected["type_result"], type_revision_id, handle_kinds, f"{label}.type_result"
    )
    diagnostic = expected["primary_diagnostic"]
    if fixture["category"] in EXPECTED_DIAGNOSTICS:
        _validate_diagnostic(
            diagnostic, revision_id, handle_kinds, f"{label}.primary_diagnostic"
        )
        _require(
            diagnostic["code"] == EXPECTED_DIAGNOSTICS[fixture["category"]],
            "diagnostic_code_unexpected",
            label,
        )
    else:
        _require(diagnostic is None, "diagnostic_unexpected", label)

    protocol_result = expected["protocol_result"]
    category = fixture["category"]
    if category == "positive":
        _require(
            isinstance(protocol_result, dict)
            and protocol_result.get("operation") == "inspect",
            "inspection_missing",
            label,
        )
        _require(protocol_result.get("status") == "ok", "inspection_status", label)
        results = protocol_result.get("results")
        _require(isinstance(results, list) and results, "inspection_results", label)
        required = set(protocol["shapes"]["InspectionResult"]["required"])
        for index, result in enumerate(results):
            _validate_inspection_result(
                result, revision_id, handle_kinds, required, f"{label}.results[{index}]"
            )
    elif category == "recovery":
        _require(
            isinstance(protocol_result, dict)
            and protocol_result.get("status") == "recovered",
            "recovery_result",
            label,
        )
        inserted = protocol_result.get("inserted_tokens")
        _require(
            isinstance(inserted, list) and len(inserted) == 1,
            "recovery_inserted_token",
            label,
        )
        _require(
            inserted[0] == {"token": ":", "utf8_byte": 21},
            "recovery_inserted_token",
            label,
        )
        _require(
            protocol_result.get("retained_following_fields") == ["task"],
            "recovery_following_field",
            label,
        )
    elif category == "rename":
        _validate_rename_fixture(fixture, protocol)
    elif category == "stale-revision":
        _validate_stale_fixture(fixture, protocol)
    else:
        _require(protocol_result is None, "protocol_result_unexpected", label)

    if category == "positive":
        _require(source == canonical, "positive_not_canonical", label)
    if category == "formatting":
        _require(source != canonical, "formatting_no_change", label)
    if category in {"recovery", "stale-revision"}:
        _require(canonical is None, "canonical_source_unexpected", label)


def _validate_rename_fixture(fixture: dict[str, Any], protocol: dict[str, Any]) -> None:
    input_value = fixture["input"]
    expected = fixture["expected"]
    request = input_value.get("protocol_request")
    result = expected["protocol_result"]
    _require(
        isinstance(request, dict) and request.get("operation") == "rename",
        "rename_request",
        fixture["id"],
    )
    _require(
        isinstance(result, dict) and result.get("operation") == "rename",
        "rename_result",
        fixture["id"],
    )
    required = set(protocol["shapes"]["RenameSuccess"]["required"])
    _require(
        required <= set(result),
        "rename_result_fields",
        f"{fixture['id']}: {sorted(required - set(result))}",
    )
    _require(result["status"] == "committed", "rename_status", fixture["id"])
    base_revision = input_value["revision"]["revision_id"]
    _require(
        request["base_revision_id"] == base_revision == result["base_revision_id"],
        "rename_base_revision",
        fixture["id"],
    )
    handle_kinds = set(protocol["handle_kinds"])
    _validate_handle(
        request["handle"],
        base_revision,
        handle_kinds,
        f"{fixture['id']}.request.handle",
    )
    _require(request["handle"]["kind"] == "symbol", "rename_handle_kind", fixture["id"])
    _require(
        re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", request["new_name"]) is not None,
        "rename_name_invalid",
        fixture["id"],
    )
    revision = result["revision"]
    _require(isinstance(revision, dict), "rename_revision", fixture["id"])
    _require(
        revision.get("parent_revision_id") == base_revision,
        "rename_parent_revision",
        fixture["id"],
    )
    new_revision = revision.get("revision_id")
    _require(
        isinstance(new_revision, str) and new_revision != base_revision,
        "rename_revision",
        fixture["id"],
    )
    canonical = expected["canonical_source"]
    _validate_source_digest(revision, canonical, f"{fixture['id']}.result.revision")
    edits = result["edits"]
    _require(isinstance(edits, list) and edits, "rename_edits", fixture["id"])
    edited = _apply_edits(input_value["source"], edits, input_value["path"])
    _require(edited == canonical, "rename_edit_result", fixture["id"])
    _validate_identity_map(
        result["identity_map"],
        base_revision,
        new_revision,
        handle_kinds,
        set(protocol["identity_classifications"]),
    )
    _require(result["diagnostics"] == [], "rename_diagnostics", fixture["id"])
    _require(
        result["validation"] == {"parse": "ok", "types": "ok", "capabilities": "ok"},
        "rename_validation",
        fixture["id"],
    )


def _validate_stale_fixture(fixture: dict[str, Any], protocol: dict[str, Any]) -> None:
    input_value = fixture["input"]
    expected = fixture["expected"]
    request = input_value.get("protocol_request")
    result = expected["protocol_result"]
    _require(
        isinstance(request, dict) and request.get("operation") == "rename",
        "stale_request",
        fixture["id"],
    )
    _require(
        isinstance(result, dict) and result.get("operation") == "rename",
        "stale_result",
        fixture["id"],
    )
    required = set(protocol["shapes"]["RenameFailure"]["required"])
    _require(
        required <= set(result),
        "stale_result_fields",
        f"{fixture['id']}: {sorted(required - set(result))}",
    )
    _require(result["status"] == "rejected", "stale_status", fixture["id"])
    base = request["base_revision_id"]
    current = request["workspace_current_revision_id"]
    _require(base != current, "stale_same_revision", fixture["id"])
    _require(
        result["base_revision_id"] == base and result["current_revision_id"] == current,
        "stale_revisions",
        fixture["id"],
    )
    _require(result["edits"] == [], "stale_edits", fixture["id"])
    handle_kinds = set(protocol["handle_kinds"])
    _validate_handle(
        request["handle"], base, handle_kinds, f"{fixture['id']}.request.handle"
    )
    _validate_diagnostic(
        result["diagnostic"], base, handle_kinds, f"{fixture['id']}.result.diagnostic"
    )
    _require(
        result["diagnostic"] == expected["primary_diagnostic"],
        "stale_diagnostic_mismatch",
        fixture["id"],
    )


def _validate_protocol(protocol: dict[str, Any]) -> None:
    _require(
        protocol.get("protocol_contract_version") == 1,
        "protocol_version",
        "expected version 1",
    )
    _require(
        protocol.get("status") == "Proposed", "protocol_status", "expected Proposed"
    )
    _require(
        protocol.get("transport") == "independent",
        "protocol_transport",
        "must remain independent",
    )
    handle_kinds = _string_list(
        protocol.get("handle_kinds"),
        "protocol_handle_kinds",
        "handle_kinds",
        allow_empty=False,
    )
    _require(
        set(handle_kinds) == {"symbol", "syntax", "expression"},
        "protocol_handle_kinds",
        "unexpected kinds",
    )
    classifications = _string_list(
        protocol.get("identity_classifications"),
        "protocol_identity_classes",
        "identity_classifications",
        allow_empty=False,
    )
    _require(
        set(classifications) == {"surviving", "replaced", "removed", "unmapped"},
        "protocol_identity_classes",
        "unexpected classes",
    )
    shapes = protocol.get("shapes")
    _require(
        isinstance(shapes, dict) and set(shapes) == REQUIRED_PROTOCOL_SHAPES,
        "protocol_shapes",
        "shape set mismatch",
    )
    for name, shape in shapes.items():
        _require(
            isinstance(shape, dict) and set(shape) == {"required", "optional"},
            "protocol_shape_invalid",
            name,
        )
        required = _string_list(
            shape["required"],
            "protocol_shape_invalid",
            f"{name}.required",
            allow_empty=False,
        )
        optional = _string_list(
            shape["optional"], "protocol_shape_invalid", f"{name}.optional"
        )
        _require(not (set(required) & set(optional)), "protocol_shape_overlap", name)
    _require(
        set(protocol.get("operations", {})) == {"inspect", "rename"},
        "protocol_operations",
        "expected inspect and rename",
    )


def validate_contract(
    contract: dict[str, Any],
    protocol: dict[str, Any],
    fixture_values: list[tuple[Path, dict[str, Any]]],
) -> tuple[int, int, int]:
    accepted_requirements = _accepted_requirements()
    _validate_protocol(protocol)
    _require(
        contract.get("core_contract_version") == 1,
        "contract_version",
        "expected version 1",
    )
    _require(
        contract.get("status") == "Proposed", "contract_status", "expected Proposed"
    )

    documents = _string_list(
        contract.get("documents"), "contract_documents", "documents", allow_empty=False
    )
    for raw_path in documents:
        _require((SPECS / raw_path).is_file(), "contract_document_missing", raw_path)

    constructs = contract.get("constructs")
    _require(
        isinstance(constructs, list), "constructs_invalid", "constructs: expected list"
    )
    construct_ids = [
        item.get("id") if isinstance(item, dict) else None for item in constructs
    ]
    _require(
        tuple(construct_ids) == EXPECTED_CONSTRUCTS,
        "construct_set_invalid",
        f"expected exactly {EXPECTED_CONSTRUCTS}",
    )
    construct_by_id: dict[str, dict[str, Any]] = {}
    for item in constructs:
        _require(
            set(item) == {"id", "title", "required_facets"},
            "construct_fields",
            str(item.get("id")),
        )
        _require(
            isinstance(item["title"], str) and item["title"],
            "construct_title",
            item["id"],
        )
        _string_list(
            item["required_facets"],
            "construct_facets",
            f"{item['id']}.required_facets",
            allow_empty=False,
        )
        construct_by_id[item["id"]] = item

    rules = contract.get("rules")
    _require(
        isinstance(rules, list) and rules,
        "rules_invalid",
        "rules: expected non-empty list",
    )
    rule_by_id: dict[str, dict[str, Any]] = {}
    sequences: dict[str, list[int]] = {"language": [], "protocol": []}
    for rule in rules:
        _require(isinstance(rule, dict), "rule_invalid", "rule: expected object")
        _require(
            set(rule)
            == {"id", "kind", "title", "requirements", "constructs", "facets"},
            "rule_fields",
            str(rule.get("id")),
        )
        rule_id = rule["id"]
        _require(
            isinstance(rule_id, str) and rule_id not in rule_by_id,
            "rule_id_duplicate",
            str(rule_id),
        )
        match = re.fullmatch(r"M11-(LANG|PROTO)-(\d{3})", rule_id)
        _require(match is not None, "rule_id_invalid", rule_id)
        expected_kind = "language" if match.group(1) == "LANG" else "protocol"
        _require(rule["kind"] == expected_kind, "rule_kind_mismatch", rule_id)
        sequences[expected_kind].append(int(match.group(2)))
        requirements = _string_list(
            rule["requirements"],
            "rule_requirements",
            f"{rule_id}.requirements",
            allow_empty=False,
        )
        _require(
            set(requirements) <= accepted_requirements,
            "rule_requirement_unknown",
            f"{rule_id}: {sorted(set(requirements) - accepted_requirements)}",
        )
        rule_constructs = _string_list(
            rule["constructs"], "rule_constructs", f"{rule_id}.constructs"
        )
        _require(
            set(rule_constructs) <= set(EXPECTED_CONSTRUCTS),
            "rule_construct_unknown",
            rule_id,
        )
        _string_list(
            rule["facets"], "rule_facets", f"{rule_id}.facets", allow_empty=False
        )
        rule_by_id[rule_id] = rule
    for kind, numbers in sequences.items():
        _require(
            numbers == list(range(1, len(numbers) + 1)),
            "rule_sequence",
            f"{kind}: {numbers}",
        )
    _validate_rule_documents(set(rule_by_id))

    categories = _string_list(
        contract.get("required_fixture_categories"),
        "fixture_categories",
        "required_fixture_categories",
        allow_empty=False,
    )
    _require(
        tuple(categories) == EXPECTED_CATEGORIES,
        "fixture_categories",
        f"expected {EXPECTED_CATEGORIES}",
    )
    fixture_paths = _string_list(
        contract.get("fixtures"), "fixture_paths", "fixtures", allow_empty=False
    )
    _require(
        len(fixture_paths) == len(EXPECTED_CATEGORIES),
        "fixture_count",
        "expected one fixture per required category",
    )
    _require(
        [path.relative_to(SPECS).as_posix() for path, _ in fixture_values]
        == fixture_paths,
        "fixture_path_order",
        "loaded fixture paths differ",
    )

    fixture_ids: set[str] = set()
    seen_categories: list[str] = []
    referenced_rules: set[str] = set()
    covered_facets: dict[str, set[str]] = {
        construct: set() for construct in EXPECTED_CONSTRUCTS
    }
    for path, fixture in fixture_values:
        _validate_fixture(
            path,
            fixture,
            accepted_requirements,
            set(EXPECTED_CONSTRUCTS),
            rule_by_id,
            protocol,
        )
        _require(
            fixture["id"] not in fixture_ids, "fixture_id_duplicate", fixture["id"]
        )
        fixture_ids.add(fixture["id"])
        seen_categories.append(fixture["category"])
        for rule_id in fixture["rules"]:
            referenced_rules.add(rule_id)
            rule = rule_by_id[rule_id]
            for construct in set(fixture["constructs"]) & set(rule["constructs"]):
                covered_facets[construct].update(rule["facets"])
    _require(
        tuple(seen_categories) == EXPECTED_CATEGORIES,
        "fixture_category_order",
        str(seen_categories),
    )
    _require(
        referenced_rules == set(rule_by_id),
        "rule_fixture_coverage",
        f"uncovered={sorted(set(rule_by_id) - referenced_rules)}",
    )
    for construct, definition in construct_by_id.items():
        missing = set(definition["required_facets"]) - covered_facets[construct]
        _require(
            not missing,
            "construct_fixture_coverage",
            f"{construct}: missing {sorted(missing)}",
        )

    return len(constructs), len(rules), len(fixture_values)


def _require_canonical_json(path: Path, value: dict[str, Any]) -> None:
    expected = json.dumps(value, ensure_ascii=False, indent=2) + "\n"
    _require(
        path.read_text(encoding="utf-8") == expected,
        "runtime_json_not_canonical",
        path.relative_to(ROOT).as_posix(),
    )


def _validate_runtime_protocol(protocol: dict[str, Any]) -> None:
    _require(
        protocol.get("runtime_protocol_version") == 1,
        "runtime_protocol_version",
        "expected version 1",
    )
    _require(
        protocol.get("status") == "Accepted",
        "runtime_protocol_status",
        "expected Accepted",
    )
    _require(
        protocol.get("transport") == "independent",
        "runtime_protocol_transport",
        "must remain independent",
    )
    shapes = protocol.get("shapes")
    _require(
        isinstance(shapes, dict) and set(shapes) == RUNTIME_PROTOCOL_SHAPES,
        "runtime_protocol_shapes",
        "shape set mismatch",
    )
    for name, shape in shapes.items():
        _require(
            isinstance(shape, dict) and set(shape) == {"required", "optional"},
            "runtime_protocol_shape",
            name,
        )
        required = _string_list(
            shape["required"],
            "runtime_protocol_shape",
            f"{name}.required",
            allow_empty=False,
        )
        optional = _string_list(
            shape["optional"], "runtime_protocol_shape", f"{name}.optional"
        )
        _require(
            not set(required) & set(optional), "runtime_protocol_shape", name
        )
    _require(
        protocol.get("operations")
        == {
            "execute": {
                "request": "ExecutionRequest",
                "success": "ExecutionSuccess",
                "failure": "ExecutionFailure",
            }
        },
        "runtime_protocol_operation",
        "expected execute",
    )


def _validate_runtime_rules(
    contract: dict[str, Any], accepted_requirements: set[str]
) -> dict[str, dict[str, Any]]:
    rules = contract.get("rules")
    _require(
        isinstance(rules, list) and rules,
        "runtime_rules_invalid",
        "expected rules",
    )
    by_id: dict[str, dict[str, Any]] = {}
    sequences: dict[str, list[int]] = {"LANG": [], "RUNTIME": [], "PROTO": []}
    for rule in rules:
        _require(
            isinstance(rule, dict)
            and set(rule) == {"id", "kind", "title", "requirements"},
            "runtime_rule_fields",
            str(rule),
        )
        rule_id = rule["id"]
        match = re.fullmatch(r"M17-(LANG|RUNTIME|PROTO)-(\d{3})", rule_id)
        _require(
            match is not None and rule_id not in by_id,
            "runtime_rule_id",
            str(rule_id),
        )
        prefix = match.group(1)
        expected_kind = {
            "LANG": "language",
            "RUNTIME": "runtime",
            "PROTO": "protocol",
        }[prefix]
        _require(
            rule["kind"] == expected_kind,
            "runtime_rule_kind",
            rule_id,
        )
        _require(
            isinstance(rule["title"], str) and rule["title"],
            "runtime_rule_title",
            rule_id,
        )
        requirements = _string_list(
            rule["requirements"],
            "runtime_rule_requirements",
            rule_id,
            allow_empty=False,
        )
        _require(
            set(requirements) <= accepted_requirements,
            "runtime_rule_requirement_unknown",
            rule_id,
        )
        sequences[prefix].append(int(match.group(2)))
        by_id[rule_id] = rule
    for prefix, numbers in sequences.items():
        _require(
            numbers == list(range(1, len(numbers) + 1)),
            "runtime_rule_sequence",
            f"{prefix}: {numbers}",
        )
    document = (SPECS / "runtime.md").read_text(encoding="utf-8")
    documented = set(
        re.findall(r"^### (M17-(?:LANG|RUNTIME|PROTO)-\d{3}) — ", document, re.MULTILINE)
    )
    _require(
        documented == set(by_id),
        "runtime_rule_document_mismatch",
        f"metadata-only={sorted(set(by_id) - documented)}, document-only={sorted(documented - set(by_id))}",
    )
    return by_id


def _validate_runtime_fixtures(
    contract: dict[str, Any], rules: dict[str, dict[str, Any]]
) -> int:
    expected_paths = [
        "runtime-fixtures/diagnostics.json",
        "runtime-fixtures/execution.json",
    ]
    _require(
        contract.get("fixtures") == expected_paths,
        "runtime_fixture_paths",
        "fixture path set or order changed",
    )
    diagnostics_path = SPECS / expected_paths[0]
    execution_path = SPECS / expected_paths[1]
    diagnostics = _load_json(diagnostics_path)
    execution = _load_json(execution_path)
    _require_canonical_json(diagnostics_path, diagnostics)
    _require_canonical_json(execution_path, execution)

    _require(
        set(diagnostics) == {"fixture_format", "id", "rules", "cases"}
        and diagnostics["fixture_format"] == 1,
        "runtime_diagnostic_fixture",
        "unexpected shape",
    )
    diagnostic_rules = _string_list(
        diagnostics["rules"],
        "runtime_fixture_rules",
        "diagnostics.rules",
        allow_empty=False,
    )
    _require(
        set(diagnostic_rules) <= set(rules),
        "runtime_fixture_rules",
        "unknown diagnostic rule",
    )
    cases = diagnostics["cases"]
    _require(
        isinstance(cases, list)
        and [case.get("id") for case in cases] == list(RUNTIME_DIAGNOSTICS),
        "runtime_diagnostic_cases",
        "case set or order changed",
    )
    for case in cases:
        _require(
            set(case) == {"id", "source", "primary_diagnostic"},
            "runtime_diagnostic_case",
            str(case.get("id")),
        )
        _require(
            isinstance(case["source"], str) and case["source"].endswith("\n"),
            "runtime_diagnostic_source",
            case["id"],
        )
        _require(
            case["primary_diagnostic"] == RUNTIME_DIAGNOSTICS[case["id"]],
            "runtime_diagnostic_code",
            case["id"],
        )

    _require(
        set(execution) == {"fixture_format", "id", "source", "environment", "cases"}
        and execution["fixture_format"] == 1,
        "runtime_execution_fixture",
        "unexpected shape",
    )
    source = execution["source"]
    _require(
        source
        == {
            "path": "runtime-fixtures/job-service.ail",
            "sha256": source.get("sha256"),
            "canonical": True,
            "format_idempotent": True,
            "static_status": "ok",
        },
        "runtime_source_metadata",
        "unexpected source contract",
    )
    source_path = SPECS / source["path"]
    _require(source_path.is_file(), "runtime_source_missing", source["path"])
    _require(
        hashlib.sha256(source_path.read_bytes()).hexdigest() == source["sha256"],
        "runtime_source_digest",
        source["path"],
    )
    _require(
        execution["environment"]
        == {
            "capabilities": {
                "JobsStore": {
                    "insert_if_absent": {
                        "parameters": ["Job"],
                        "result": "InsertOutcome",
                    }
                }
            }
        },
        "runtime_environment",
        "JobsStore signature changed",
    )
    execution_cases = execution["cases"]
    _require(
        isinstance(execution_cases, list)
        and tuple(case.get("id") for case in execution_cases) == RUNTIME_CASES,
        "runtime_execution_cases",
        "case set or order changed",
    )
    referenced_rules = set(diagnostic_rules)
    for case in execution_cases:
        case_rules = _string_list(
            case.get("rules"),
            "runtime_fixture_rules",
            case.get("id", "case"),
            allow_empty=False,
        )
        _require(
            set(case_rules) <= set(rules),
            "runtime_fixture_rules",
            case["id"],
        )
        referenced_rules.update(case_rules)
        if "call_count" in case:
            _require(
                case["call_count"] in {0, 1},
                "runtime_call_count",
                case["id"],
            )
    _require(
        referenced_rules == set(rules),
        "runtime_rule_fixture_coverage",
        f"uncovered={sorted(set(rules) - referenced_rules)}",
    )
    return len(cases) + len(execution_cases)


def validate_runtime_contract(
    contract: dict[str, Any], protocol: dict[str, Any]
) -> tuple[int, int, int]:
    _validate_runtime_protocol(protocol)
    _require(
        contract.get("runtime_contract_version") == 1,
        "runtime_contract_version",
        "expected version 1",
    )
    _require(
        contract.get("status") == "Accepted" and contract.get("extends") == "M11",
        "runtime_contract_status",
        "must be accepted M11 extension",
    )
    documents = _string_list(
        contract.get("documents"),
        "runtime_documents",
        "documents",
        allow_empty=False,
    )
    _require(
        documents == ["runtime.md", "runtime-protocol.json"],
        "runtime_documents",
        "document set or order changed",
    )
    for document in documents:
        _require((SPECS / document).is_file(), "runtime_document_missing", document)
    intrinsics = contract.get("intrinsics")
    _require(
        isinstance(intrinsics, dict)
        and set(intrinsics)
        == {
            "bytes.length_gt",
            "text.byte_length_between",
            "text.contains_control",
            "text.first_ascii_alphanumeric",
            "text.is_empty",
            "text.rest_ascii_alphanumeric_or",
            "text.scalar_count_gt",
        },
        "runtime_intrinsics",
        "intrinsic set changed",
    )
    for name, signature in intrinsics.items():
        _require(
            isinstance(signature, dict)
            and set(signature) == {"parameters", "result"}
            and signature["result"] == "Bool",
            "runtime_intrinsic_signature",
            name,
        )
        _require(
            isinstance(signature["parameters"], list)
            and bool(signature["parameters"])
            and all(
                isinstance(parameter, str) and parameter
                for parameter in signature["parameters"]
            ),
            "runtime_intrinsic_signature",
            name,
        )
    rules = _validate_runtime_rules(contract, _accepted_requirements())
    fixture_count = _validate_runtime_fixtures(contract, rules)
    return len(rules), fixture_count, len(protocol["shapes"])


def _load_repository_runtime_contract() -> tuple[dict[str, Any], dict[str, Any]]:
    contract = _load_json(RUNTIME_CONTRACT_PATH)
    protocol = _load_json(RUNTIME_PROTOCOL_PATH)
    _require_canonical_json(RUNTIME_CONTRACT_PATH, contract)
    _require_canonical_json(RUNTIME_PROTOCOL_PATH, protocol)
    return contract, protocol


def _load_repository_contract() -> tuple[
    dict[str, Any], dict[str, Any], list[tuple[Path, dict[str, Any]]]
]:
    contract = _load_json(CONTRACT_PATH)
    protocol = _load_json(PROTOCOL_PATH)
    raw_paths = contract.get("fixtures")
    _require(isinstance(raw_paths, list), "fixture_paths", "fixtures must be a list")
    fixture_values: list[tuple[Path, dict[str, Any]]] = []
    for raw_path in raw_paths:
        _require(isinstance(raw_path, str), "fixture_path", repr(raw_path))
        path = SPECS / raw_path
        _require(path.is_file(), "fixture_missing", raw_path)
        fixture_values.append((path, _load_json(path)))
    return contract, protocol, fixture_values


def _self_test(
    contract: dict[str, Any],
    protocol: dict[str, Any],
    fixtures: list[tuple[Path, dict[str, Any]]],
) -> int:
    mutations: list[tuple[str, Any]] = []

    sixth = copy.deepcopy(contract)
    sixth["constructs"].append(
        {
            "id": "if",
            "title": "Forbidden sixth construct",
            "required_facets": ["grammar"],
        }
    )
    mutations.append(("sixth-construct", (sixth, protocol, fixtures)))

    unknown_requirement = copy.deepcopy(contract)
    unknown_requirement["rules"][0]["requirements"].append("LANG-999")
    mutations.append(("unknown-requirement", (unknown_requirement, protocol, fixtures)))

    missing_shape = copy.deepcopy(protocol)
    del missing_shape["shapes"]["IdentityMap"]
    mutations.append(("missing-protocol-shape", (contract, missing_shape, fixtures)))

    incomplete_fixture_values = [
        (path, copy.deepcopy(value)) for path, value in fixtures
    ]
    del incomplete_fixture_values[0][1]["expected"]["type_result"]
    mutations.append(
        ("missing-expected-field", (contract, protocol, incomplete_fixture_values))
    )

    bad_digest_values = [(path, copy.deepcopy(value)) for path, value in fixtures]
    bad_digest_values[0][1]["input"]["revision"]["source_digest"] = "sha256:" + "0" * 64
    mutations.append(("changed-source", (contract, protocol, bad_digest_values)))

    for name, values in mutations:
        try:
            validate_contract(*values)
        except ContractError:
            continue
        _raise("self_test_failed", f"mutation {name!r} was accepted")
    return len(mutations)


def _runtime_self_test(
    contract: dict[str, Any], protocol: dict[str, Any]
) -> int:
    mutations: list[tuple[str, dict[str, Any], dict[str, Any]]] = []

    changed_status = copy.deepcopy(contract)
    changed_status["status"] = "Proposed"
    mutations.append(("unaccepted-runtime", changed_status, protocol))

    unknown_requirement = copy.deepcopy(contract)
    unknown_requirement["rules"][0]["requirements"].append("LANG-999")
    mutations.append(("runtime-unknown-requirement", unknown_requirement, protocol))

    missing_shape = copy.deepcopy(protocol)
    del missing_shape["shapes"]["RuntimeFault"]
    mutations.append(("runtime-missing-shape", contract, missing_shape))

    for name, changed_contract, changed_protocol in mutations:
        try:
            validate_runtime_contract(changed_contract, changed_protocol)
        except ContractError:
            continue
        _raise("runtime_self_test_failed", f"mutation {name!r} was accepted")
    return len(mutations)


def check() -> None:
    contract, protocol, fixtures = _load_repository_contract()
    construct_count, rule_count, fixture_count = validate_contract(
        contract, protocol, fixtures
    )
    mutation_count = _self_test(contract, protocol, fixtures)
    runtime_contract, runtime_protocol = _load_repository_runtime_contract()
    runtime_rule_count, runtime_fixture_count, runtime_shape_count = (
        validate_runtime_contract(runtime_contract, runtime_protocol)
    )
    runtime_mutation_count = _runtime_self_test(runtime_contract, runtime_protocol)
    print(
        "M11 core contract passed: "
        f"{construct_count} constructs, {rule_count} numbered rules, "
        f"{fixture_count} canonical fixtures, {len(protocol['shapes'])} protocol shapes, "
        f"and {mutation_count} rejection mutations verified."
    )
    print(
        "M17 runtime contract passed: "
        f"{runtime_rule_count} numbered rules, {runtime_fixture_count} fixture cases, "
        f"{runtime_shape_count} protocol shapes, and "
        f"{runtime_mutation_count} rejection mutations verified."
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("check",))
    args = parser.parse_args()
    try:
        if args.command == "check":
            check()
    except ContractError as error:
        print(f"ERROR [{error.code}]: {error.message}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
