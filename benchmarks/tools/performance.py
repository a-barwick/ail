#!/usr/bin/env python3
"""Measure M8 warm handlers and cold processes with one cross-language policy."""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import math
import os
import platform
import selectors
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, NoReturn

import fixtures as fixture_tool


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "benchmarks" / "fixtures" / "manifest.json"
EXPERIMENT_CONTRACT = ROOT / "benchmarks" / "calibration" / "experiment-contract.json"
WARM_SCHEMA = (
    ROOT / "benchmarks" / "schemas" / "calibration-warm-measurement.schema.json"
)
COLD_SCHEMA = (
    ROOT / "benchmarks" / "schemas" / "calibration-cold-measurement.schema.json"
)
BUILD_ROOT = Path(tempfile.gettempdir()) / "ail-m8-performance"
PILOT_ROOT = ROOT / "benchmarks" / "calibration" / "pilots" / "m8e"
LANGUAGES = ("rust", "go", "python", "typescript")
CAMPAIGN_ID = "m8e-non-official-pilot"
CONFIGURATION_ID = "m8-agent-experiment-v1"
NETWORK_POLICY = "(version 1)(allow default)(deny network*)"

ADAPTER_FILES = {
    "rust": "benchmarks/baselines/rust/v2/examples/performance_adapter.rs",
    "go": "benchmarks/baselines/go/v2/cmd/performance-adapter/main.go",
    "python": "benchmarks/baselines/python/performance_adapter.py",
    "typescript": "benchmarks/baselines/typescript/v2/performance-adapter.ts",
}
PERFORMANCE_SUPPORT_FILES = {
    "rust": [],
    "go": [],
    "python": [],
    "typescript": ["benchmarks/performance/typescript-tsconfig.json"],
}
DEPENDENCY_LOCKS = {
    "rust": "benchmarks/baselines/rust/Cargo.lock",
    "go": "benchmarks/baselines/go/go.mod",
    "python": "benchmarks/baselines/python/uv.lock",
    "typescript": "benchmarks/baselines/typescript/package-lock.json",
}


@dataclass(frozen=True)
class PerformanceError(Exception):
    code: str
    message: str

    def __str__(self) -> str:
        return f"{self.code}: {self.message}"


@dataclass(frozen=True)
class ProcessObservation:
    process: subprocess.Popen[str]
    stderr_path: Path
    process_creation_ns: int
    readiness_ns: int
    idle_rss_bytes: int
    peak_rss_bytes: int
    ready: dict[str, Any]
    monitor: str


def _raise(code: str, message: str) -> NoReturn:
    raise PerformanceError(code, message)


def _canonical(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def _canonical_bytes(value: Any) -> bytes:
    return _canonical(value).encode("utf-8")


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _write(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(_canonical(value), encoding="utf-8")


def _load(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        _raise("performance_input_invalid", f"{path}: expected object")
    return value


def _schema_check(value: dict[str, Any], schema_path: Path) -> None:
    schema = fixture_tool._load_json(schema_path)
    errors = fixture_tool._schema_errors(value, schema, schema, "$")
    if errors:
        _raise("performance_record_invalid", "; ".join(map(str, errors[:8])))


def _percentile(samples: list[int], percentile: int) -> int:
    if not samples:
        _raise("performance_samples_invalid", "latency samples are empty")
    ordered = sorted(samples)
    index = max(0, math.ceil(percentile / 100 * len(ordered)) - 1)
    return ordered[index]


def _variance(samples: list[int]) -> int:
    if not samples:
        _raise("performance_samples_invalid", "latency samples are empty")
    mean = sum(samples) // len(samples)
    return sum((sample - mean) ** 2 for sample in samples) // len(samples)


def _expected_results() -> list[dict[str, Any]]:
    manifest = _load(MANIFEST)
    results = []
    for entry in manifest["fixtures"]:
        fixture = _load(ROOT / entry["path"])
        results.append(
            {
                "result_format": 1,
                "case_id": fixture["case_id"],
                "operation": fixture["operation"],
                "actual": fixture["expected"],
            }
        )
    return results


def _verify_results(value: dict[str, Any]) -> None:
    if value.get("type") != "verified" or value.get("results") != _expected_results():
        _raise(
            "performance_correctness_failed",
            "adapter result or ordered trace differs from the frozen corpus",
        )


def _package_files(language: str) -> list[str]:
    checkpoints = _load(
        ROOT / "benchmarks" / "baselines" / language / "checkpoints.json"
    )
    files = list(checkpoints["checkpoints"][1]["files"])
    files.append(ADAPTER_FILES[language])
    files.extend(PERFORMANCE_SUPPORT_FILES[language])
    return sorted(set(files))


def _package_manifest(language: str) -> dict[str, Any]:
    files = [
        {"path": path, "sha256": _sha256(ROOT / path)}
        for path in _package_files(language)
    ]
    return {
        "package_manifest_format": 1,
        "language": language,
        "dependency_lock": {
            "path": DEPENDENCY_LOCKS[language],
            "sha256": _sha256(ROOT / DEPENDENCY_LOCKS[language]),
        },
        "files": files,
    }


def _environment() -> dict[str, Any]:
    contract = _load(EXPERIMENT_CONTRACT)
    return {
        "reference": contract["environment"],
        "observed": {
            "system": platform.system(),
            "release": platform.release(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
    }


def _affinity() -> dict[str, Any]:
    return {
        "policy": "single-sequential-unbound",
        "binding_supported": "no" if sys.platform == "darwin" else "recorded-only",
        "logical_cpu_count": os.cpu_count() or 0,
    }


def _load_state() -> dict[str, Any]:
    try:
        values = os.getloadavg()
        return {
            "available": "yes",
            "one_minute_milli": round(values[0] * 1000),
            "five_minute_milli": round(values[1] * 1000),
            "fifteen_minute_milli": round(values[2] * 1000),
        }
    except OSError:
        return {"available": "no"}


def _prepare(language: str) -> list[str]:
    BUILD_ROOT.mkdir(parents=True, exist_ok=True)
    manifest = str(MANIFEST.resolve())
    if language == "rust":
        target = BUILD_ROOT / "rust-target"
        subprocess.run(
            [
                "rustup",
                "run",
                "1.88.0",
                "cargo",
                "build",
                "--quiet",
                "--offline",
                "--locked",
                "--manifest-path",
                str(ROOT / "benchmarks/baselines/rust/Cargo.toml"),
                "--package",
                "ail-job-service-v2",
                "--example",
                "performance_adapter",
                "--target-dir",
                str(target),
            ],
            cwd=ROOT,
            check=True,
        )
        return [
            str(target / "debug" / "examples" / "performance_adapter"),
            "--manifest",
            manifest,
        ]
    if language == "go":
        binary = BUILD_ROOT / "go-performance-adapter"
        subprocess.run(
            [
                "go",
                "-C",
                str(ROOT / "benchmarks/baselines/go"),
                "build",
                "-o",
                str(binary),
                "./v2/cmd/performance-adapter",
            ],
            cwd=ROOT,
            check=True,
            env={
                **os.environ,
                "GOTOOLCHAIN": "local",
                "GOCACHE": "/tmp/ail-go-build-cache",
            },
        )
        return [str(binary), "--manifest", manifest]
    if language == "python":
        return [
            sys.executable,
            str(ROOT / ADAPTER_FILES[language]),
            "--manifest",
            manifest,
        ]
    output = BUILD_ROOT / "typescript"
    subprocess.run(
        [
            str(ROOT / "benchmarks/baselines/typescript/node_modules/.bin/tsc"),
            "--project",
            str(ROOT / "benchmarks/performance/typescript-tsconfig.json"),
            "--outDir",
            str(output),
        ],
        cwd=ROOT,
        check=True,
    )
    (output / "package.json").write_text('{"type":"module"}\n', encoding="utf-8")
    return [
        "node",
        str(output / "v2" / "performance-adapter.js"),
        "--manifest",
        manifest,
    ]


def _rss_bytes(pid: int) -> tuple[int, str]:
    proc_status = Path(f"/proc/{pid}/status")
    if proc_status.is_file():
        for line in proc_status.read_text(encoding="utf-8").splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1]) * 1024, "proc-status"
    if sys.platform == "darwin":

        class ProcTaskInfo(ctypes.Structure):
            _fields_ = [
                ("virtual_size", ctypes.c_uint64),
                ("resident_size", ctypes.c_uint64),
                ("total_user", ctypes.c_uint64),
                ("total_system", ctypes.c_uint64),
                ("threads_user", ctypes.c_uint64),
                ("threads_system", ctypes.c_uint64),
                ("policy", ctypes.c_int32),
                ("faults", ctypes.c_int32),
                ("pageins", ctypes.c_int32),
                ("cow_faults", ctypes.c_int32),
                ("messages_sent", ctypes.c_int32),
                ("messages_received", ctypes.c_int32),
                ("syscalls_mach", ctypes.c_int32),
                ("syscalls_unix", ctypes.c_int32),
                ("csw", ctypes.c_int32),
                ("threadnum", ctypes.c_int32),
                ("numrunning", ctypes.c_int32),
                ("priority", ctypes.c_int32),
            ]

        info = ProcTaskInfo()
        library = ctypes.CDLL("/usr/lib/libproc.dylib")
        size = library.proc_pidinfo(pid, 4, 0, ctypes.byref(info), ctypes.sizeof(info))
        if size == ctypes.sizeof(info):
            return int(info.resident_size), "proc-pidtaskinfo"
    _raise("rss_monitor_unavailable", f"cannot observe RSS for pid {pid}")


def _read_line(
    process: subprocess.Popen[str], timeout: float, peak: int = 0
) -> tuple[dict[str, Any], int]:
    if process.stdout is None:
        _raise("performance_protocol_failed", "adapter stdout is unavailable")
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    deadline = time.monotonic() + timeout
    observed_peak = peak
    while time.monotonic() < deadline:
        if process.poll() is not None:
            _raise(
                "performance_process_failed",
                f"adapter exited {process.returncode} before a protocol result",
            )
        try:
            rss, _ = _rss_bytes(process.pid)
            observed_peak = max(observed_peak, rss)
        except PerformanceError:
            pass
        if selector.select(timeout=0.005):
            line = process.stdout.readline()
            try:
                value = json.loads(line)
            except json.JSONDecodeError as error:
                _raise("performance_protocol_failed", str(error))
            if not isinstance(value, dict):
                _raise("performance_protocol_failed", "adapter emitted a non-object")
            return value, observed_peak
    _raise("performance_timeout", "adapter protocol response exceeded its limit")


def _send(
    observation: ProcessObservation, value: dict[str, Any], timeout: float
) -> tuple[dict[str, Any], int]:
    if observation.process.stdin is None:
        _raise("performance_protocol_failed", "adapter stdin is unavailable")
    observation.process.stdin.write(json.dumps(value, separators=(",", ":")) + "\n")
    observation.process.stdin.flush()
    return _read_line(observation.process, timeout, observation.peak_rss_bytes)


def _spawn(command: list[str], *, enforce_network: bool = True) -> ProcessObservation:
    stderr_file = tempfile.NamedTemporaryFile(
        prefix="ail-m8e-stderr-", suffix=".log", delete=False
    )
    stderr_path = Path(stderr_file.name)
    stderr_file.close()
    effective = command
    monitor = "outer-sandbox"
    if enforce_network and sys.platform == "darwin":
        effective = ["/usr/bin/sandbox-exec", "-p", NETWORK_POLICY, *command]
        monitor = "sandbox-exec-deny-report"
    environment = {
        **os.environ,
        "PYTHONHASHSEED": "0",
        "PYTHONDONTWRITEBYTECODE": "1",
        "NO_COLOR": "1",
    }
    if command[0] == sys.executable:
        environment["PYTHONPATH"] = str(ROOT / "benchmarks/baselines/python")
    started = time.perf_counter_ns()
    process_started = time.perf_counter_ns()
    with stderr_path.open("w", encoding="utf-8") as stderr:
        process = subprocess.Popen(
            effective,
            cwd=ROOT,
            env=environment,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=stderr,
            text=True,
            bufsize=1,
            start_new_session=True,
        )
    process_creation_ns = time.perf_counter_ns() - process_started
    ready, peak = _read_line(process, 2.0)
    readiness_ns = time.perf_counter_ns() - started
    if ready.get("type") != "ready" or ready.get("case_count") != len(
        _expected_results()
    ):
        process.kill()
        _raise("readiness_failed", "adapter readiness record is invalid")
    idle, rss_monitor = _rss_bytes(process.pid)
    return ProcessObservation(
        process=process,
        stderr_path=stderr_path,
        process_creation_ns=process_creation_ns,
        readiness_ns=readiness_ns,
        idle_rss_bytes=idle,
        peak_rss_bytes=max(peak, idle),
        ready=ready,
        monitor=f"{monitor}+{rss_monitor}",
    )


def _external_attempts(observation: ProcessObservation) -> int:
    try:
        text = observation.stderr_path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return 0
    return sum(
        1
        for line in text.splitlines()
        if "deny" in line.lower() and "network" in line.lower()
    )


def _stop(observation: ProcessObservation) -> tuple[int, int]:
    stopped, peak = _send(observation, {"command": "shutdown"}, 2.0)
    if stopped.get("type") != "stopped":
        _raise("performance_protocol_failed", "adapter did not acknowledge shutdown")
    try:
        status = observation.process.wait(timeout=2.0)
        if observation.process.stdin is not None:
            observation.process.stdin.close()
        if observation.process.stdout is not None:
            observation.process.stdout.close()
        return status, peak
    except subprocess.TimeoutExpired:
        observation.process.kill()
        _raise("performance_timeout", "adapter did not stop")


def _base_record(language: str, kind: str, round_number: int) -> dict[str, Any]:
    environment_sha = hashlib.sha256(_canonical_bytes(_environment())).hexdigest()
    return {
        "measurement_id": f"m8e.pilot.{kind}.{language}.{round_number:02d}",
        "schedule_id": f"pilot.{kind}.{language}.{round_number:02d}",
        "campaign_id": CAMPAIGN_ID,
        "configuration_id": CONFIGURATION_ID,
        "language": language,
        "official": "no",
        "round": round_number,
        "status": "included",
        "exclusion_reason": "",
        "environment_sha256": environment_sha,
    }


def measure_warm(
    language: str,
    output: Path,
    *,
    duration_ns: int,
    sample_stride: int,
    command: list[str] | None = None,
    enforce_network: bool = True,
) -> dict[str, Any]:
    command = command or _prepare(language)
    before_load = _load_state()
    observation = _spawn(command, enforce_network=enforce_network)
    verified, peak = _send(observation, {"command": "verify"}, 5.0)
    _verify_results(verified)
    warmed, peak = _send(observation, {"command": "warmup", "iterations": 3}, 5.0)
    if warmed.get("type") != "warmed":
        _raise("warmup_failed", "adapter warmup response is invalid")
    measured, peak = _send(
        observation,
        {
            "command": "measure",
            "duration_ns": duration_ns,
            "sample_stride": sample_stride,
        },
        max(5.0, duration_ns / 1_000_000_000 + 2.0),
    )
    exit_status, peak = _stop(observation)
    attempts = _external_attempts(observation)
    observation.stderr_path.unlink(missing_ok=True)
    if exit_status != 0 or attempts:
        _raise("warm_safety_failed", "adapter exit or external-access policy failed")
    samples = measured.get("samples_ns")
    if (
        measured.get("type") != "measured"
        or not isinstance(samples, list)
        or not samples
        or any(not isinstance(value, int) or value < 0 for value in samples)
    ):
        _raise("performance_samples_invalid", "adapter measurement is invalid")
    samples_path = output / "artifacts" / f"warm-{language}-samples.txt"
    samples_path.parent.mkdir(parents=True, exist_ok=True)
    samples_path.write_text(
        "".join(f"{value}\n" for value in samples), encoding="utf-8"
    )
    package = _package_manifest(language)
    elapsed_ns = int(measured["elapsed_ns"])
    request_count = int(measured["request_count"])
    record = {
        "warm_measurement_format": 1,
        **_base_record(language, "warm", 0),
        "package_sha256": hashlib.sha256(_canonical_bytes(package)).hexdigest(),
        "dependency_lock_sha256": package["dependency_lock"]["sha256"],
        "readiness": {
            "observed": "yes",
            "elapsed_ns": observation.readiness_ns,
            "signal": "jsonl-ready-v1",
        },
        "warmup": {
            "iterations": 3,
            "request_count": warmed["request_count"],
            "checksum": warmed["checksum"],
        },
        "clock": {
            "source": measured["clock"],
            "unit": "nanosecond",
            "monotonic": "yes",
        },
        "affinity": _affinity(),
        "load": {"before": before_load, "after": _load_state()},
        "corpus": {
            "manifest_sha256": _sha256(MANIFEST),
            "case_count": len(_expected_results()),
            "request_count": request_count,
            "elapsed_ns": elapsed_ns,
            "sample_stride": measured["sample_stride"],
            "status": "passed",
            "trace": "passed",
            "external_access_monitor": observation.monitor,
            "peak_rss_bytes": peak,
        },
        "latency": {
            "sample_count": len(samples),
            "samples_path": samples_path.relative_to(output).as_posix(),
            "samples_sha256": _sha256(samples_path),
            "p50_ns": _percentile(samples, 50),
            "p95_ns": _percentile(samples, 95),
            "p99_ns": _percentile(samples, 99),
            "variance_ns_squared": _variance(samples),
        },
        "throughput_milli_requests_per_second": (
            request_count * 1_000_000_000_000 // elapsed_ns
        ),
    }
    _schema_check(record, WARM_SCHEMA)
    return record


def measure_cold(
    language: str,
    output: Path,
    *,
    command: list[str] | None = None,
    enforce_network: bool = True,
) -> dict[str, Any]:
    command = command or _prepare(language)
    before_load = _load_state()
    package = _package_manifest(language)
    package_path = output / "artifacts" / f"cold-{language}-package.json"
    _write(package_path, package)
    observation = _spawn(command, enforce_network=enforce_network)
    corpus_started = time.perf_counter_ns()
    verified, peak = _send(observation, {"command": "verify"}, 5.0)
    corpus_elapsed = time.perf_counter_ns() - corpus_started
    _verify_results(verified)
    measured, peak = _send(
        observation,
        {"command": "measure", "duration_ns": 50_000_000, "sample_stride": 1000},
        5.0,
    )
    if measured.get("type") != "measured":
        _raise("performance_protocol_failed", "cold workload response is invalid")
    exit_status, peak = _stop(observation)
    attempts = _external_attempts(observation)
    observation.stderr_path.unlink(missing_ok=True)
    record = {
        "cold_measurement_format": 1,
        **_base_record(language, "cold", 0),
        "package_manifest_path": package_path.relative_to(output).as_posix(),
        "package_manifest_sha256": _sha256(package_path),
        "dependency_lock_sha256": package["dependency_lock"]["sha256"],
        "clock": {
            "source": "time.perf_counter_ns",
            "unit": "nanosecond",
            "monotonic": "yes",
        },
        "affinity": _affinity(),
        "load": {"before": before_load, "after": _load_state()},
        "readiness_signal": "jsonl-ready-v1",
        "process_creation_ns": observation.process_creation_ns,
        "readiness_ns": observation.readiness_ns,
        "idle_rss_bytes": observation.idle_rss_bytes,
        "peak_rss_bytes": peak,
        "exit_status": exit_status,
        "external_access_attempts": attempts,
        "corpus": {
            "manifest_sha256": _sha256(MANIFEST),
            "case_count": len(_expected_results()),
            "elapsed_ns": corpus_elapsed,
            "status": "passed",
            "trace": "passed",
            "external_access_monitor": observation.monitor,
        },
    }
    limits = _load(EXPERIMENT_CONTRACT)["limits"]
    reasons = []
    if observation.readiness_ns > limits["cold_start_milliseconds"] * 1_000_000:
        reasons.append("cold_start_limit")
    if peak > limits["peak_rss_bytes"]:
        reasons.append("peak_rss_limit")
    if exit_status != 0:
        reasons.append("nonzero_exit")
    if attempts:
        reasons.append("external_access_attempt")
    if reasons:
        record["status"] = "excluded"
        record["exclusion_reason"] = ",".join(reasons)
    _schema_check(record, COLD_SCHEMA)
    return record


def run_pilots(
    output: Path, languages: tuple[str, ...], duration_ns: int
) -> dict[str, Any]:
    output.mkdir(parents=True, exist_ok=True)
    records = []
    for language in languages:
        command = _prepare(language)
        for kind in ("warm", "cold"):
            try:
                if kind == "warm":
                    record = measure_warm(
                        language,
                        output,
                        duration_ns=duration_ns,
                        sample_stride=10,
                        command=command,
                    )
                else:
                    record = measure_cold(
                        language,
                        output,
                        command=command,
                    )
            except PerformanceError as error:
                record = {
                    "measurement_id": f"m8e.pilot.{kind}.{language}.00",
                    "language": language,
                    "kind": kind,
                    "official": "no",
                    "status": "excluded",
                    "exclusion_reason": error.code,
                    "detail": error.message,
                }
            path = output / "records" / f"{kind}-{language}.json"
            _write(path, record)
            records.append(
                {
                    "language": language,
                    "kind": kind,
                    "path": path.relative_to(output).as_posix(),
                    "sha256": _sha256(path),
                    "status": record["status"],
                    "exclusion_reason": record["exclusion_reason"],
                }
            )
    summary = {
        "m8e_pilot_summary_format": 1,
        "official": "no",
        "campaign_id": CAMPAIGN_ID,
        "configuration_id": CONFIGURATION_ID,
        "records": records,
    }
    _write(output / "pilot-summary.json", summary)
    return summary


def verify_fake_measurements() -> tuple[str, str]:
    """Run deterministic warm and cold protocol tests without a real baseline."""
    command = [
        sys.executable,
        str(ROOT / "benchmarks/tests/support/fake_performance_adapter.py"),
        "--manifest",
        str(MANIFEST),
    ]
    with tempfile.TemporaryDirectory(prefix="ail-m8e-fake-") as temporary:
        output = Path(temporary)
        warm = measure_warm(
            "python",
            output,
            duration_ns=1,
            sample_stride=1,
            command=command,
            enforce_network=False,
        )
        cold = measure_cold("python", output, command=command, enforce_network=False)
    if warm["latency"]["p50_ns"] != 30 or cold["corpus"]["trace"] != "passed":
        _raise("performance_fake_failed", "deterministic fake result changed")
    return ("warm:included", "cold:included")


def verify_retained_pilots() -> tuple[str, ...]:
    """Verify the retained M8e matrix without treating it as campaign evidence."""
    summary_path = PILOT_ROOT / "pilot-summary.json"
    summary = _load(summary_path)
    if summary_path.read_text(encoding="utf-8") != _canonical(summary):
        _raise("performance_pilot_invalid", "pilot summary is not canonical")
    if (
        summary.get("m8e_pilot_summary_format") != 1
        or summary.get("official") != "no"
        or summary.get("campaign_id") != CAMPAIGN_ID
        or summary.get("configuration_id") != CONFIGURATION_ID
    ):
        _raise("performance_pilot_invalid", "pilot summary identity differs")
    entries = summary.get("records")
    if not isinstance(entries, list):
        _raise("performance_pilot_invalid", "pilot records are missing")
    expected = [(language, kind) for language in LANGUAGES for kind in ("warm", "cold")]
    actual = [(entry.get("language"), entry.get("kind")) for entry in entries]
    if actual != expected:
        _raise("performance_pilot_invalid", "pilot matrix or order differs")
    records: dict[tuple[str, str], dict[str, Any]] = {}
    outcomes = []
    for entry in entries:
        raw_path = entry.get("path")
        if not isinstance(raw_path, str):
            _raise("performance_pilot_invalid", "pilot path is invalid")
        path = (PILOT_ROOT / raw_path).resolve()
        try:
            path.relative_to(PILOT_ROOT)
        except ValueError:
            _raise("performance_pilot_invalid", "pilot path leaves its root")
        record = _load(path)
        if (
            path.read_text(encoding="utf-8") != _canonical(record)
            or _sha256(path) != entry.get("sha256")
            or record.get("official") != "no"
            or record.get("status") != "included"
            or record.get("exclusion_reason") != ""
            or record.get("language") != entry["language"]
        ):
            _raise(
                "performance_pilot_invalid",
                f"{entry['language']} {entry['kind']}: identity or status differs",
            )
        kind = entry["kind"]
        _schema_check(record, WARM_SCHEMA if kind == "warm" else COLD_SCHEMA)
        if (
            record["corpus"].get("status") != "passed"
            or record["corpus"].get("trace") != "passed"
        ):
            _raise(
                "performance_pilot_invalid",
                f"{entry['language']} {kind}: correctness differs",
            )
        if kind == "warm":
            samples_path = PILOT_ROOT / record["latency"]["samples_path"]
            if _sha256(samples_path) != record["latency"]["samples_sha256"]:
                _raise("performance_pilot_invalid", "sample digest differs")
            samples = [
                int(line)
                for line in samples_path.read_text(encoding="utf-8").splitlines()
            ]
            latency = record["latency"]
            if (
                len(samples) != latency["sample_count"]
                or _percentile(samples, 50) != latency["p50_ns"]
                or _percentile(samples, 95) != latency["p95_ns"]
                or _percentile(samples, 99) != latency["p99_ns"]
                or _variance(samples) != latency["variance_ns_squared"]
                or record["throughput_milli_requests_per_second"] <= 0
            ):
                _raise(
                    "performance_pilot_invalid",
                    f"{entry['language']}: warm statistics differ",
                )
        else:
            package_path = PILOT_ROOT / record["package_manifest_path"]
            if (
                _sha256(package_path) != record["package_manifest_sha256"]
                or record["external_access_attempts"] != 0
                or record["readiness_ns"] > 2_000_000_000
                or record["peak_rss_bytes"] > 536_870_912
                or record["exit_status"] != 0
            ):
                _raise(
                    "performance_pilot_invalid",
                    f"{entry['language']}: cold safety or package differs",
                )
        records[(entry["language"], kind)] = record
        outcomes.append(f"{entry['language']}:{kind}:included")
    for language in LANGUAGES:
        if (
            records[(language, "warm")]["package_sha256"]
            != records[(language, "cold")]["package_manifest_sha256"]
        ):
            _raise(
                "performance_pilot_invalid",
                f"{language}: warm and cold package identities differ",
            )
    return tuple(outcomes)


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    pilot = subparsers.add_parser("pilot")
    pilot.add_argument("--language", choices=(*LANGUAGES, "all"), default="all")
    pilot.add_argument(
        "--output",
        type=Path,
        default=PILOT_ROOT,
    )
    pilot.add_argument("--duration-ms", type=int, default=250)
    return parser.parse_args()


def main() -> int:
    args = _parse_args()
    try:
        languages = LANGUAGES if args.language == "all" else (args.language,)
        summary = run_pilots(
            args.output.resolve(), languages, args.duration_ms * 1_000_000
        )
    except (OSError, subprocess.SubprocessError, PerformanceError) as error:
        print(error, file=sys.stderr)
        return 1
    for record in summary["records"]:
        print(
            f"{record['language']} {record['kind']}: {record['status']}"
            + (f" ({record['exclusion_reason']})" if record["exclusion_reason"] else "")
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
