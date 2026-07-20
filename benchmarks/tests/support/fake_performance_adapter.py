#!/usr/bin/env python3
"""Deterministic line-protocol adapter for M8 performance tests."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def emit(value: object) -> None:
    print(json.dumps(value, separators=(",", ":")), flush=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args()
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    root = args.manifest.parents[2]
    fixtures = [
        json.loads((root / entry["path"]).read_text(encoding="utf-8"))
        for entry in manifest["fixtures"]
    ]
    results = [
        {
            "result_format": 1,
            "case_id": fixture["case_id"],
            "operation": fixture["operation"],
            "actual": fixture["expected"],
        }
        for fixture in fixtures
    ]
    emit({"type": "ready", "case_count": len(fixtures)})
    for line in sys.stdin:
        command = json.loads(line)
        if command["command"] == "verify":
            emit({"type": "verified", "results": results})
        elif command["command"] == "warmup":
            emit(
                {
                    "type": "warmed",
                    "iterations": command["iterations"],
                    "request_count": command["iterations"] * len(fixtures),
                    "checksum": 0,
                }
            )
        elif command["command"] == "measure":
            emit(
                {
                    "type": "measured",
                    "clock": "fake-monotonic",
                    "elapsed_ns": 1_000_000,
                    "request_count": 100,
                    "sample_stride": command["sample_stride"],
                    "samples_ns": [10, 20, 30, 40, 50],
                    "checksum": 0,
                }
            )
        elif command["command"] == "shutdown":
            emit({"type": "stopped"})
            return 0
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
