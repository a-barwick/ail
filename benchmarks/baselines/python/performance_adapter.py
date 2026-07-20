#!/usr/bin/env python3
"""Persistent M8 performance adapter for the frozen Python V2 boundary."""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Any

from v2.fixture import run_case


def _emit(value: dict[str, Any]) -> None:
    sys.stdout.write(
        json.dumps(value, ensure_ascii=False, separators=(",", ":")) + "\n"
    )
    sys.stdout.flush()


def _load_cases(manifest_path: Path) -> list[object]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    return [
        json.loads(
            (manifest_path.parents[2] / entry["path"]).read_text(encoding="utf-8")
        )
        for entry in manifest["fixtures"]
    ]


def _run(cases: list[object]) -> list[dict[str, Any]]:
    return [run_case(case) for case in cases]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args()
    cases = _load_cases(args.manifest.resolve())
    _emit({"type": "ready", "case_count": len(cases)})

    for line in sys.stdin:
        command = json.loads(line)
        kind = command["command"]
        if kind == "verify":
            _emit({"type": "verified", "results": _run(cases)})
        elif kind == "warmup":
            iterations = int(command["iterations"])
            checksum = 0
            for _ in range(iterations):
                checksum ^= sum(len(result["case_id"]) for result in _run(cases))
            _emit(
                {
                    "type": "warmed",
                    "iterations": iterations,
                    "request_count": iterations * len(cases),
                    "checksum": checksum,
                }
            )
        elif kind == "measure":
            duration_ns = int(command["duration_ns"])
            sample_stride = int(command["sample_stride"])
            started = time.perf_counter_ns()
            samples: list[int] = []
            request_count = 0
            checksum = 0
            while request_count == 0 or time.perf_counter_ns() - started < duration_ns:
                for case in cases:
                    before = time.perf_counter_ns()
                    result = run_case(case)
                    elapsed = time.perf_counter_ns() - before
                    if request_count % sample_stride == 0:
                        samples.append(elapsed)
                    request_count += 1
                    checksum ^= len(result["case_id"])
            elapsed_ns = time.perf_counter_ns() - started
            _emit(
                {
                    "type": "measured",
                    "clock": "time.perf_counter_ns",
                    "elapsed_ns": elapsed_ns,
                    "request_count": request_count,
                    "sample_stride": sample_stride,
                    "samples_ns": samples,
                    "checksum": checksum,
                }
            )
        elif kind == "shutdown":
            _emit({"type": "stopped"})
            return 0
        else:
            raise ValueError(f"unsupported command {kind!r}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
