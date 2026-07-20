from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "benchmarks" / "tools"))

import performance as performance_tool  # noqa: E402


class PerformanceTests(unittest.TestCase):
    def command(self) -> list[str]:
        return [
            sys.executable,
            str(ROOT / "benchmarks/tests/support/fake_performance_adapter.py"),
            "--manifest",
            str(performance_tool.MANIFEST),
        ]

    def test_percentiles_use_nearest_rank_and_variance_is_integer(self) -> None:
        samples = [50, 10, 40, 20, 30]
        self.assertEqual(performance_tool._percentile(samples, 50), 30)
        self.assertEqual(performance_tool._percentile(samples, 95), 50)
        self.assertEqual(performance_tool._variance(samples), 200)

    def test_fake_warm_and_cold_measurements_are_schema_valid(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            warm = performance_tool.measure_warm(
                "python",
                output,
                duration_ns=1,
                sample_stride=1,
                command=self.command(),
                enforce_network=False,
            )
            cold = performance_tool.measure_cold(
                "python",
                output,
                command=self.command(),
                enforce_network=False,
            )
        self.assertEqual(warm["status"], "included")
        self.assertEqual(warm["latency"]["p99_ns"], 50)
        self.assertEqual(warm["latency"]["variance_ns_squared"], 200)
        self.assertEqual(cold["status"], "included")
        self.assertEqual(cold["external_access_attempts"], 0)
        self.assertEqual(cold["corpus"]["trace"], "passed")

    def test_pilot_summary_is_explicitly_non_official(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            warm = performance_tool.measure_warm(
                "python",
                output,
                duration_ns=1,
                sample_stride=1,
                command=self.command(),
                enforce_network=False,
            )
            record = output / "records" / "warm-python.json"
            performance_tool._write(record, warm)
            summary = {
                "m8e_pilot_summary_format": 1,
                "official": "no",
                "records": [
                    {
                        "path": record.relative_to(output).as_posix(),
                        "sha256": performance_tool._sha256(record),
                    }
                ],
            }
            performance_tool._write(output / "pilot-summary.json", summary)
            loaded = json.loads(
                (output / "pilot-summary.json").read_text(encoding="utf-8")
            )
        self.assertEqual(loaded["official"], "no")


if __name__ == "__main__":
    unittest.main()
