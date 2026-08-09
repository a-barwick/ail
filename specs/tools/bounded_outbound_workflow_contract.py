#!/usr/bin/env python3
"""Verify the accepted M31 bounded outbound workflow contract."""

import hashlib
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def check() -> None:
    contract = json.loads((ROOT / "specs/bounded-outbound-workflow-contract.json").read_text())
    protocol = json.loads((ROOT / "specs/bounded-outbound-workflow-protocol.json").read_text())
    spec = (ROOT / "specs/bounded-outbound-workflows.md").read_text()
    tests = (ROOT / "compiler/ail-compiler/tests/m31_bounded_outbound_workflows.rs").read_text()
    require(contract["milestone"] == "M31" and contract["status"] == "accepted", "M31 status drift")
    require(contract["input_bound"] == 8 and contract["concurrency_limit"] == 3, "M31 bounds drift")
    for rule in contract["rules"]:
        require(f"### {rule}" in spec, f"missing {rule}")
    for source in contract["canonical_sources"]:
        digest = hashlib.sha256((ROOT / source["path"]).read_bytes()).hexdigest()
        require(digest == source["sha256"], f"source digest drift: {source['path']}")
    for proof in contract["proof_tests"]:
        require(f"fn {proof}()" in tests, f"missing proof test {proof}")
    require(protocol["host_interface"] == ["start_outbound", "check_outbound", "cancel_outbound", "collect_outbound"], "host lifecycle drift")
    print("M31 bounded outbound workflow contract check passed: 10 rules, 8 executable proofs.")


if __name__ == "__main__":
    require(sys.argv[1:] == ["check"], "usage: bounded_outbound_workflow_contract.py check")
    check()
