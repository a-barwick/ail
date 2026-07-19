from __future__ import annotations

import hashlib
import io
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, cast

import pytest

from v2.runner import RunnerError, repository_root, run, run_cli

HIGH_PRIORITY_FIXTURE = (
    "benchmarks/fixtures/public/create_job/uc003-v2-created-priority-high.json"
)


@pytest.mark.parametrize(
    "arguments",
    [
        (),
        ("--case",),
        ("--case", "x", "y"),
        ("--unknown", "x"),
    ],
)
def test_argument_contract_rejects_invalid_shapes(
    arguments: tuple[str, ...],
) -> None:
    with pytest.raises(RunnerError, match="expected exactly"):
        run(arguments)


def test_one_case_accepts_relative_and_absolute_paths(
    repository_root: Path,
) -> None:
    relative = run(("--case", HIGH_PRIORITY_FIXTURE))
    absolute = run(("--case", str(repository_root / HIGH_PRIORITY_FIXTURE)))
    assert relative == absolute
    assert relative["case_id"] == "uc003-v2-created-priority-high"


def test_corpus_preserves_manifest_digest_count_and_order(
    repository_root: Path,
) -> None:
    result = run(("--corpus", "benchmarks/fixtures/manifest.json"))
    manifest_bytes = (
        repository_root / "benchmarks/fixtures/manifest.json"
    ).read_bytes()
    assert (
        result["fixture_manifest_sha256"] == hashlib.sha256(manifest_bytes).hexdigest()
    )
    cases = cast("list[dict[str, Any]]", result["results"])
    assert len(cases) == 37
    assert cases[0]["case_id"] == "uc001-v1-created-empty-payload"
    assert cases[-1]["case_id"] == "uc003-v1-stored-job-adapted"


def test_cli_writes_exactly_one_json_value_and_separates_errors() -> None:
    stdout = io.StringIO()
    stderr = io.StringIO()
    assert run_cli(("--case", HIGH_PRIORITY_FIXTURE), stdout, stderr) == 0
    assert stderr.getvalue() == ""
    assert json.loads(stdout.getvalue())["case_id"].endswith("priority-high")

    stdout = io.StringIO()
    stderr = io.StringIO()
    assert run_cli(("--unknown",), stdout, stderr) == 1
    assert stdout.getvalue() == ""
    assert "expected exactly" in stderr.getvalue()


def test_runner_module_process_writes_only_normalized_json(
    repository_root: Path,
) -> None:
    baseline = repository_root / "benchmarks/baselines/python"
    completed = subprocess.run(
        [sys.executable, "-m", "v2.runner", "--case", HIGH_PRIORITY_FIXTURE],
        cwd=baseline,
        check=True,
        capture_output=True,
        text=True,
    )
    result = json.loads(completed.stdout)
    assert result["case_id"] == "uc003-v2-created-priority-high"
    assert completed.stderr == ""


def test_missing_and_malformed_inputs_report_errors(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    with pytest.raises(RunnerError, match="could not read"):
        run(("--case", "benchmarks/fixtures/public/missing.json"))

    monkeypatch.chdir(repository_root())
    malformed = tmp_path / "manifest.json"
    malformed.write_text("{", encoding="utf-8")
    with pytest.raises(RunnerError, match="could not read or parse"):
        run(("--corpus", str(malformed)))


@pytest.mark.parametrize(
    "manifest",
    [
        {},
        {"fixtures": [{}]},
        {"fixtures": [{"path": 3}]},
    ],
)
def test_malformed_manifest_shape_is_rejected(
    manifest: dict[str, object],
    tmp_path: Path,
) -> None:
    path = tmp_path / "manifest.json"
    path.write_text(json.dumps(manifest), encoding="utf-8")
    with pytest.raises(RunnerError):
        run(("--corpus", str(path)))


class FailingWriter(io.StringIO):
    def write(self, _value: str) -> int:
        raise OSError("write denied")


def test_cli_reports_json_encoding_failure() -> None:
    stderr = io.StringIO()
    assert run_cli(("--case", HIGH_PRIORITY_FIXTURE), FailingWriter(), stderr) == 1
    assert "write denied" in stderr.getvalue()


def test_repository_root_outside_checkout_raises(tmp_path: Path) -> None:
    with pytest.raises(RunnerError, match="not inside"):
        repository_root(tmp_path)
