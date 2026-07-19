#!/usr/bin/env python3
"""Deliberately controllable implementation runner for harness self-tests."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import sys
import time
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[3]


def load(path: str) -> dict[str, Any]:
    value = json.loads((ROOT / path).read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError("expected JSON object")
    return value


def single(case: dict[str, Any]) -> dict[str, Any]:
    return {
        "result_format": 1,
        "case_id": case["case_id"],
        "operation": case["operation"],
        "actual": copy.deepcopy(case["expected"]),
    }


def mutate(result: dict[str, Any], mode: str) -> None:
    first = result if "actual" in result else result["results"][0]
    actual = first["actual"]
    if mode == "response_mismatch":
        actual["response"]["result"] = {"kind": "persistence_unavailable"}
    elif mode == "final_state_mismatch":
        actual["final_jobs"] = []
    elif mode == "store_calls_mismatch":
        actual["store_calls"] = []
    elif mode == "result_schema_invalid":
        first.pop("actual")


def main() -> int:
    marker = os.environ.get("AIL_FAKE_MARKER")
    if marker:
        Path(marker).write_text("started\n", encoding="utf-8")
    mode = os.environ.get("AIL_FAKE_MODE", "pass")
    if mode == "timeout":
        time.sleep(2)
    if mode == "malformed_result":
        print("{")
        return 0
    if mode == "nonzero_exit":
        print("deliberate failure", file=sys.stderr)
        return 17

    parser = argparse.ArgumentParser()
    selected = parser.add_mutually_exclusive_group(required=True)
    selected.add_argument("--case")
    selected.add_argument("--corpus")
    args = parser.parse_args()

    if args.case:
        result = single(load(args.case))
    else:
        manifest_path = ROOT / args.corpus
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        result = {
            "result_format": 1,
            "fixture_manifest_sha256": hashlib.sha256(
                manifest_path.read_bytes()
            ).hexdigest(),
            "results": [single(load(entry["path"])) for entry in manifest["fixtures"]],
        }
        if mode == "missing_case":
            result["results"].pop()
        elif mode == "unexpected_case":
            extra = copy.deepcopy(result["results"][0])
            extra["case_id"] = "unexpected-case"
            result["results"].append(extra)
        elif mode == "manifest_mismatch":
            result["fixture_manifest_sha256"] = "0" * 64

    mutate(result, mode)
    print(json.dumps(result, ensure_ascii=False, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
