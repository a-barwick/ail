#!/usr/bin/env python3
"""Verify the standalone M29 bounded-list contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = ROOT / "specs" / "bounded-list-contract.json"
PROTOCOL_PATH = ROOT / "specs" / "bounded-list-protocol.json"
SPEC_PATH = ROOT / "specs" / "bounded-lists.md"
REQUIREMENTS_PATH = ROOT / "docs" / "requirements" / "bounded-ordered-lists.md"
USE_CASE_PATH = ROOT / "docs" / "use-cases" / "UC-009-bounded-ordered-batch-cancellation.md"

EXPECTED_REQUIREMENTS = ["APP-007", "LANG-007", "PROTO-008", "NFR-008"]
EXPECTED_RULES = [
    "M29-LANG-001",
    "M29-LANG-002",
    "M29-LANG-003",
    "M29-LANG-004",
    "M29-RUNTIME-001",
    "M29-RUNTIME-002",
    "M29-RUNTIME-003",
    "M29-PROTO-001",
    "M29-PROTO-002",
    "M29-PROTO-003",
]
EXPECTED_DIAGNOSTICS = [
    "AIL.TYPE.LIST_BOUND",
    "AIL.TYPE.LIST_ELEMENT",
    "AIL.TYPE.MAP_SOURCE",
    "AIL.RUNTIME.LIST_CARDINALITY",
    "AIL.RUNTIME.LIST_ELEMENT",
]
EXPECTED_SHAPES = {
    "TypeRef.List": ["element", "max_length"],
    "RuntimeValue.List": ["items"],
    "BoundedListInspection": ["element_type", "element_identity", "max_length"],
    "ValueParameterInspection": ["name", "value_type", "bounded_list"],
    "SourceSetFunctionInspection": [
        "revision_id",
        "function_handle",
        "module_identity",
        "function_identity",
        "parameters",
        "result_type",
        "result_list",
        "effects",
        "capabilities",
        "dependencies",
    ],
    "ListCardinalityFault": [
        "code",
        "maximum",
        "count",
        "element_type",
        "value_path",
        "calls",
    ],
    "ListElementFault": [
        "code",
        "element_type",
        "index",
        "actual_type",
        "value_path",
        "calls",
    ],
}
EXPECTED_TESTS = [
    "list_and_map_syntax_is_canonical_typed_and_contextual",
    "invalid_bounds_elements_and_map_sources_have_stable_diagnostics",
    "batch_cancellation_is_empty_ordered_aligned_and_duplicate_preserving",
    "external_lists_are_completely_validated_before_capability_checks_or_calls",
    "exact_bound_completes_and_provider_faults_stop_at_the_failing_index",
    "map_bodies_preserve_imported_transitive_effects",
    "aliases_element_identities_source_order_and_revisions_remain_deterministic",
    "inspection_exposes_list_bounds_map_types_binders_and_dependencies",
    "map_binding_rename_edits_the_binder_and_references_not_the_contextual_keyword",
    "atomic_candidate_validation_accepts_map_and_rejects_invalid_map_without_publication",
]


class ContractError(Exception):
    pass


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ContractError(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def load_canonical_json(path: Path) -> dict[str, Any]:
    try:
        raw = path.read_text(encoding="utf-8")
        value = json.loads(raw, object_pairs_hook=unique_object)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ContractError(f"{path.relative_to(ROOT)}: {error}") from error
    if not isinstance(value, dict):
        raise ContractError(f"{path.relative_to(ROOT)}: root must be an object")
    canonical = json.dumps(value, ensure_ascii=False, indent=2) + "\n"
    if raw != canonical:
        raise ContractError(f"{path.relative_to(ROOT)}: JSON is not canonical")
    return value


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def check() -> None:
    contract = load_canonical_json(CONTRACT_PATH)
    protocol = load_canonical_json(PROTOCOL_PATH)
    specification = SPEC_PATH.read_text(encoding="utf-8")
    requirements = REQUIREMENTS_PATH.read_text(encoding="utf-8")
    use_case = USE_CASE_PATH.read_text(encoding="utf-8")

    require(contract.get("contract_version") == "m29-v1", "wrong contract version")
    require(contract.get("status") == "accepted", "contract is not accepted")
    require(contract.get("milestone") == "M29", "contract milestone is not M29")
    require(contract.get("use_case") == "UC-009", "contract use case is not UC-009")
    require(contract.get("requirements") == EXPECTED_REQUIREMENTS, "requirement list drift")
    require(contract.get("rules") == EXPECTED_RULES, "rule list drift")
    require(contract.get("diagnostics") == EXPECTED_DIAGNOSTICS, "diagnostic list drift")
    require(
        contract.get("language_max_list_length") == 4_294_967_295,
        "language maximum drift",
    )
    require(contract.get("application_max_list_length") == 32, "application bound drift")

    documented_rules = re.findall(r"^### (M29-[A-Z]+-\d{3}) — ", specification, re.MULTILINE)
    require(documented_rules == EXPECTED_RULES, "specification rule headings drift")
    for diagnostic in EXPECTED_DIAGNOSTICS:
        require(diagnostic in specification, f"specification omits {diagnostic}")
    for requirement in EXPECTED_REQUIREMENTS:
        pattern = rf"^## {re.escape(requirement)} — .+?^Status: \*\*Accepted\*\*"
        require(
            re.search(pattern, requirements, re.MULTILINE | re.DOTALL) is not None,
            f"{requirement} is not accepted",
        )
        require(requirement in specification, f"specification omits {requirement}")
    require("Status: **Accepted 2026-08-08**" in use_case, "UC-009 is not accepted")

    sources = contract.get("canonical_sources")
    require(isinstance(sources, list) and len(sources) == 3, "canonical source list drift")
    source_paths: list[str] = []
    for source in sources:
        require(isinstance(source, dict), "canonical source entry must be an object")
        path_text = source.get("path")
        expected_hash = source.get("sha256")
        require(isinstance(path_text, str), "canonical source path must be text")
        require(isinstance(expected_hash, str), "canonical source digest must be text")
        path = ROOT / path_text
        data = path.read_bytes()
        require(data.endswith(b"\n"), f"{path_text} has no final newline")
        require(hashlib.sha256(data).hexdigest() == expected_hash, f"{path_text} digest drift")
        source_paths.append(path_text)
    require(source_paths == sorted(source_paths), "canonical source paths are not sorted")

    test_path_text = contract.get("rust_test")
    require(isinstance(test_path_text, str), "rust_test must be text")
    test_source = (ROOT / test_path_text).read_text(encoding="utf-8")
    tests = re.findall(r"#\[test\]\s+fn ([a-z0-9_]+)\(", test_source)
    require(tests == EXPECTED_TESTS, "M29 executable test matrix drift")
    for diagnostic in EXPECTED_DIAGNOSTICS:
        require(diagnostic in test_source, f"M29 tests omit {diagnostic}")

    require(protocol.get("protocol_version") == "m29-v1", "wrong protocol version")
    shapes = protocol.get("shapes")
    require(isinstance(shapes, list), "protocol shapes must be an array")
    observed_shapes = {
        shape.get("name"): shape.get("fields")
        for shape in shapes
        if isinstance(shape, dict)
    }
    require(observed_shapes == EXPECTED_SHAPES, "protocol shape drift")
    require(
        list(observed_shapes) == list(EXPECTED_SHAPES),
        "protocol shape order drift",
    )
    require(
        list(protocol.get("ordering", {}))
        == ["list_items", "map_evaluation", "dependencies", "capability_calls"],
        "protocol ordering facts drift",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["check"])
    parser.parse_args()
    try:
        check()
    except ContractError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print("M29 bounded-list contract check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
