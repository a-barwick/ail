#!/usr/bin/env python3
"""Verify the standalone accepted M30 outbound-request contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = ROOT / "specs/outbound-request-contract.json"
PROTOCOL_PATH = ROOT / "specs/outbound-request-protocol.json"
SPEC_PATH = ROOT / "specs/outbound-requests.md"
README_PATH = ROOT / "specs/README.md"
WORKLOAD_PATH = ROOT / "docs/workloads/outbound-dependency-request.md"
DECISION_PATH = ROOT / "docs/decisions/0009-cooperative-outbound-requests.md"

RULES = [
    *(f"M30-LANG-{number:03}" for number in range(1, 5)),
    *(f"M30-RUNTIME-{number:03}" for number in range(1, 5)),
    *(f"M30-PROTO-{number:03}" for number in range(1, 4)),
]
DIAGNOSTICS = [
    "AIL.CAPABILITY.OUTBOUND_TIMEOUT_CONTRACT",
    "AIL.CAPABILITY.OUTBOUND_CANCELLATION_CONTRACT",
    "AIL.CAPABILITY.OUTBOUND_RESULT_CONTRACT",
    "AIL.RUNTIME.OUTBOUND_TIMEOUT_ARGUMENT",
    "AIL.RUNTIME.OUTBOUND_UNSUPPORTED",
    "AIL.RUNTIME.ARGUMENT_TYPE",
    "AIL.RUNTIME.CAPABILITY_RESULT",
    "AIL.RUNTIME.CAPABILITY_CONTRACT",
]
REJECTIONS = [
    "invalid-timeout-index", "invalid-cancellation-index", "overlapping-indices",
    "wrong-timeout-type", "wrong-cancellation-type", "invalid-timeout-maximum",
    "non-variant-result", "missing-timeout-case", "missing-cancelled-case",
    "same-completion-case", "payload-bearing-timeout-case",
    "payload-bearing-cancelled-case", "missing-capability-permission",
    "missing-effect-permission", "timeout-zero", "timeout-over-maximum",
    "malformed-cancellation-value", "unsupported-outbound-provider",
    "unknown-returned-case", "malformed-returned-value",
    "provider-contract-fault",
]
COMPLETIONS = [
    "returned-found", "returned-not-found", "returned-unavailable",
    "timed-out-synthesized", "cancelled-synthesized", "ordinary-call-compatible",
    "inspection-deterministic", "retained-revision-binding",
]
SHAPES = {
    "OperationMetadata.Outbound": ("fields", ["timeout_argument_index", "cancellation_argument_index", "maximum_timeout_ms", "timed_out_case_identity", "cancelled_case_identity"]),
    "CapabilityEnvironment": ("fields", ["interfaces", "stable_digest"]),
    "OutboundOperationInspection": ("fields", ["revision_id", "capability_environment_digest", "receiver", "operation", "effect", "operation_kind", "timeout_argument_index", "timeout_parameter_type", "maximum_timeout_ms", "cancellation_argument_index", "cancellation_parameter_type", "result_variant_identity", "timed_out_case_identity", "cancelled_case_identity"]),
    "OutboundProviderOutcome": ("cases", ["Returned(value)", "TimedOut", "Cancelled"]),
    "ObservedOutboundCall": ("fields", ["receiver", "operation", "effect", "arguments", "timeout_ms", "cancellation_token_identity", "outcome", "result"]),
}


class ContractError(Exception):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    raw = path.read_text(encoding="utf-8")
    try:
        value = json.loads(raw, object_pairs_hook=unique_object)
    except json.JSONDecodeError as error:
        raise ContractError(f"{path.relative_to(ROOT)}: {error}") from error
    require(isinstance(value, dict), f"{path.relative_to(ROOT)} root is not an object")
    require(raw == json.dumps(value, ensure_ascii=False, indent=2) + "\n", f"{path.relative_to(ROOT)} is not canonical JSON")
    return value


def check() -> None:
    contract = load_json(CONTRACT_PATH)
    protocol = load_json(PROTOCOL_PATH)
    spec = SPEC_PATH.read_text(encoding="utf-8")
    require(contract.get("contract_version") == "m30-v1", "wrong contract version")
    require(contract.get("status") == "accepted", "contract is not accepted")
    require(contract.get("milestone") == "M30", "wrong milestone")
    require(contract.get("rules") == RULES, "numbered rule list drift")
    require(re.findall(r"^### (M30-[A-Z]+-\d{3}) — ", spec, re.MULTILINE) == RULES, "numbered specification headings drift")
    require(contract.get("diagnostics") == DIAGNOSTICS, "diagnostic list drift")
    require(contract.get("maximum_timeout") == 2**64 - 1, "maximum timeout drift")
    require(contract.get("rejection_cases") == REJECTIONS, "rejection matrix drift")
    require(contract.get("completion_cases") == COMPLETIONS, "completion matrix drift")
    for text in [*DIAGNOSTICS, "synchronous", "never falls back", "stable digest", "external-only `Cancellation`"]:
        require(text in spec, f"specification omits {text!r}")

    sources = contract.get("canonical_sources")
    require(isinstance(sources, list) and len(sources) == 2, "canonical source list drift")
    combined = ""
    paths: list[str] = []
    for source in sources:
        require(isinstance(source, dict), "source entry is not an object")
        path_text, digest = source.get("path"), source.get("sha256")
        require(isinstance(path_text, str) and isinstance(digest, str), "invalid source entry")
        data = (ROOT / path_text).read_bytes()
        require(data.endswith(b"\n"), f"{path_text} has no final newline")
        require(hashlib.sha256(data).hexdigest() == digest, f"{path_text} digest drift")
        combined += data.decode("utf-8")
        paths.append(path_text)
    require(paths == sorted(paths), "canonical source paths are not sorted")
    snippets = contract.get("required_source_snippets")
    require(isinstance(snippets, list) and all(item in combined for item in snippets), "canonical source snippet missing")
    require(combined.count("dependency.fetch(key, timeout, cancellation)") == 1, "canonical source must make one direct request")

    require(protocol.get("protocol_version") == "m30-v1", "wrong protocol version")
    shapes = protocol.get("shapes")
    require(isinstance(shapes, list), "protocol shapes are not an array")
    observed: dict[str, tuple[str, Any]] = {}
    for shape in shapes:
        require(isinstance(shape, dict) and isinstance(shape.get("name"), str), "invalid protocol shape")
        keys = [key for key in ("fields", "cases") if key in shape]
        require(len(keys) == 1, f"shape {shape.get('name')} needs exactly one body")
        observed[shape["name"]] = (keys[0], shape[keys[0]])
    require(observed == SHAPES and list(observed) == list(SHAPES), "protocol shape drift")
    require(list(protocol.get("ordering", {})) == ["external_validation", "environment_digest", "observed_calls", "retained_revision"], "protocol ordering drift")

    require("[outbound request contract](outbound-requests.md)" in README_PATH.read_text(encoding="utf-8"), "spec README omits contract link")
    require("[protocol shapes](outbound-request-protocol.json)" in README_PATH.read_text(encoding="utf-8"), "spec README omits protocol link")
    require("../../specs/outbound-requests.md" in WORKLOAD_PATH.read_text(encoding="utf-8"), "workload omits contract link")
    require("../../specs/tools/outbound_request_contract.py" in DECISION_PATH.read_text(encoding="utf-8"), "decision omits checker link")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["check"])
    parser.parse_args()
    try:
        check()
    except (ContractError, OSError, UnicodeError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print("M30 outbound-request contract check passed: 11 rules, 21 rejections, 8 completions, 5 protocol shapes.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
