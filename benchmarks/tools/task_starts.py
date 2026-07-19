#!/usr/bin/env python3
"""Build and verify the deterministic answer-free M7 task-start packages."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, NoReturn


ROOT = Path(__file__).resolve().parents[2]
TASK_START_ROOT = ROOT / "benchmarks" / "task-starts"
LOCK_PATH = TASK_START_ROOT / "task-starts.json"
LANGUAGES = ("rust", "go", "python", "typescript")
TASKS = ("UC-001", "UC-003")
TREE_DIGEST_ALGORITHM = (
    "sha256 of concatenated '<file-sha256>  <workspace-relative-path>\\n' "
    "records in lexical path order"
)
FORBIDDEN_WORKSPACE_NAMES = {
    "checkpoints.json",
    "contract-lock.json",
    "hidden-contract.json",
    "m7-freeze.json",
    "m7-parity-report.json",
    "runner.json",
    "seed-locations.json",
    "task-start.json",
    "task-starts.json",
    "verification-manifest.json",
    "verification-manifest.lock.json",
}
FORBIDDEN_CONTENT_MARKERS = (
    '"hidden_package_sha256"',
    '"m7_freeze_format"',
    '"parity_report_format"',
    '"seed_location_format"',
    '"source_tree_sha256"',
    '"verification-manifest',
    "SEED.",
)
TASK_PATHS = {
    "UC-001": "benchmarks/tasks/uc001-implement-create-job.md",
    "UC-003": "benchmarks/tasks/uc003-add-priority.md",
}
BASELINE_ROOTS = {
    language: f"benchmarks/baselines/{language}" for language in LANGUAGES
}
UC001_SOURCE_PATHS = {
    "rust": "benchmarks/baselines/rust/v1/src/lib.rs",
    "go": "benchmarks/baselines/go/v1/jobservice.go",
    "python": "benchmarks/baselines/python/v1/job_service.py",
    "typescript": "benchmarks/baselines/typescript/v1/job-service.ts",
}
UC001_HOLE_MARKERS = {
    "rust": 'todo!("TODO(UC-001): implement create_job and validation")',
    "go": 'panic("TODO(UC-001): implement CreateJob and validation")',
    "python": 'raise NotImplementedError("TODO(UC-001): implement create_job")',
    "typescript": 'throw new Error("TODO(UC-001): implement createJob");',
}
UC001_CONTRACT_MARKERS = {
    "rust": (
        "pub struct CreateJobRequest",
        "pub enum CreateJobResult",
        "pub trait JobStore",
    ),
    "go": (
        "type CreateJobRequest struct",
        "type CreateJobResult interface",
        "type JobStore interface",
    ),
    "python": (
        "class CreateJobRequest:",
        "type CreateJobResult =",
        "class JobStore(Protocol):",
    ),
    "typescript": (
        "export type CreateJobRequest",
        "export type CreateJobResult",
        "export type JobStore",
    ),
}
UC003_ACCEPTED_V1_FILES = {
    "rust": (
        "benchmarks/baselines/rust/v1/Cargo.toml",
        "benchmarks/baselines/rust/v1/src/lib.rs",
    ),
    "go": (
        "benchmarks/baselines/go/v1/jobservice.go",
        "benchmarks/baselines/go/v1/jobservice_test.go",
    ),
    "python": (
        "benchmarks/baselines/python/v1/__init__.py",
        "benchmarks/baselines/python/v1/job_service.py",
        "benchmarks/baselines/python/tests/test_v1.py",
    ),
    "typescript": (
        "benchmarks/baselines/typescript/v1/job-service.ts",
        "benchmarks/baselines/typescript/tests/v1.test.ts",
    ),
}
V2_REFERENCE_SOURCE_FILES = (
    "benchmarks/baselines/rust/v2/src/codec.rs",
    "benchmarks/baselines/rust/v2/src/domain.rs",
    "benchmarks/baselines/rust/v2/src/lib.rs",
    "benchmarks/baselines/rust/v2/src/main.rs",
    "benchmarks/baselines/rust/v2/src/service.rs",
    "benchmarks/baselines/rust/v2/src/store.rs",
    "benchmarks/baselines/go/v2/cmd/runner/main.go",
    "benchmarks/baselines/go/v2/domain/domain.go",
    "benchmarks/baselines/go/v2/fixture/codec.go",
    "benchmarks/baselines/go/v2/service/service.go",
    "benchmarks/baselines/go/v2/store/store.go",
    "benchmarks/baselines/python/v2/__init__.py",
    "benchmarks/baselines/python/v2/domain.py",
    "benchmarks/baselines/python/v2/fixture.py",
    "benchmarks/baselines/python/v2/runner.py",
    "benchmarks/baselines/python/v2/service.py",
    "benchmarks/baselines/python/v2/store.py",
    "benchmarks/baselines/typescript/v2/domain.ts",
    "benchmarks/baselines/typescript/v2/fixture.ts",
    "benchmarks/baselines/typescript/v2/runner.ts",
    "benchmarks/baselines/typescript/v2/service.ts",
    "benchmarks/baselines/typescript/v2/store.ts",
)
TASK_START_DEFINITION_ARTIFACTS = (
    "benchmarks/task-starts/README.md",
    "benchmarks/task-starts/overlays/python/uc001/pyproject.toml",
    "benchmarks/task-starts/overlays/rust/uc001/Cargo.lock",
    "benchmarks/task-starts/overlays/rust/uc001/Cargo.toml",
    "benchmarks/task-starts/overlays/rust/uc003/v2/Cargo.toml",
    "benchmarks/task-starts/overlays/rust/uc003/v2/tests/task_checks.rs",
    "benchmarks/task-starts/overlays/typescript/common/.prettierignore",
    "benchmarks/task-starts/overlays/typescript/uc001/tsconfig.json",
    "benchmarks/task-starts/task-starts.json",
    "benchmarks/tests/test_task_starts.py",
    "benchmarks/tools/task_starts.py",
)


class TaskStartError(Exception):
    """A stable task-start package validation failure."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message


def _raise(code: str, message: str) -> NoReturn:
    raise TaskStartError(code, message)


@dataclass(frozen=True)
class FileSpec:
    """One explicit file copied or derived into an agent-visible workspace."""

    origin: str
    path: str
    role: str = "protected"
    derivation: str = "copy"


def _canonical(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _load_object(path: Path, code: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        _raise(code, f"{path}: {error}")
    if not isinstance(value, dict):
        _raise(code, f"{path}: must contain a JSON object")
    return value


def _repository_path(raw_path: str) -> Path:
    path = (ROOT / raw_path).resolve()
    try:
        path.relative_to(ROOT)
    except ValueError:
        _raise("task_start_recipe_invalid", f"path leaves repository: {raw_path!r}")
    if not path.is_file():
        _raise("task_start_recipe_invalid", f"missing source file {raw_path}")
    return path


def _copy(
    origin: str,
    *,
    path: str | None = None,
    role: str = "protected",
) -> FileSpec:
    return FileSpec(origin=origin, path=origin if path is None else path, role=role)


def _overlay(origin: str, path: str) -> FileSpec:
    return FileSpec(origin=origin, path=path, role="protected")


def _derived(origin: str, path: str) -> FileSpec:
    return FileSpec(
        origin=origin,
        path=path,
        role="editable",
        derivation="uc001_implementation_hole",
    )


def _public_fixture_specs() -> list[FileSpec]:
    manifest_path = "benchmarks/fixtures/manifest.json"
    manifest = _load_object(ROOT / manifest_path, "task_start_recipe_invalid")
    entries = manifest.get("fixtures")
    if not isinstance(entries, list) or not entries:
        _raise("task_start_recipe_invalid", "public fixture manifest is empty")
    paths: list[str] = []
    for entry in entries:
        if not isinstance(entry, dict) or not isinstance(entry.get("path"), str):
            _raise("task_start_recipe_invalid", "public fixture path is invalid")
        path = entry["path"]
        if not path.startswith("benchmarks/fixtures/public/"):
            _raise("task_start_recipe_invalid", f"non-public fixture path {path}")
        paths.append(path)
    if len(paths) != len(set(paths)):
        _raise("task_start_recipe_invalid", "public fixture paths are not unique")
    return [_copy(manifest_path), *(_copy(path) for path in paths)]


def _rust_specs(task: str) -> list[FileSpec]:
    prefix = BASELINE_ROOTS["rust"]
    if task == "UC-001":
        return [
            _overlay(
                "benchmarks/task-starts/overlays/rust/uc001/Cargo.toml",
                f"{prefix}/Cargo.toml",
            ),
            _overlay(
                "benchmarks/task-starts/overlays/rust/uc001/Cargo.lock",
                f"{prefix}/Cargo.lock",
            ),
            _copy(f"{prefix}/rust-toolchain.toml"),
            _copy(f"{prefix}/v1/Cargo.toml"),
            _derived(f"{prefix}/v1/src/lib.rs", f"{prefix}/v1/src/lib.rs"),
        ]
    return [
        _copy(f"{prefix}/Cargo.toml"),
        _copy(f"{prefix}/Cargo.lock"),
        _copy(f"{prefix}/rust-toolchain.toml"),
        _copy(f"{prefix}/v1/Cargo.toml"),
        _copy(f"{prefix}/v1/src/lib.rs", role="editable"),
        _overlay(
            "benchmarks/task-starts/overlays/rust/uc003/v2/Cargo.toml",
            f"{prefix}/v2/Cargo.toml",
        ),
        _overlay(
            "benchmarks/task-starts/overlays/rust/uc003/v2/tests/task_checks.rs",
            f"{prefix}/v2/tests/task_checks.rs",
        ),
        _copy(f"{prefix}/v2/tests/runner_cli.rs"),
    ]


def _go_specs(task: str) -> list[FileSpec]:
    prefix = BASELINE_ROOTS["go"]
    if task == "UC-001":
        return [
            _copy(f"{prefix}/go.mod"),
            _derived(f"{prefix}/v1/jobservice.go", f"{prefix}/v1/jobservice.go"),
            _copy(f"{prefix}/v1/jobservice_test.go"),
        ]
    return [
        _copy(f"{prefix}/go.mod"),
        _copy(f"{prefix}/v1/jobservice.go", role="editable"),
        _copy(f"{prefix}/v1/jobservice_test.go"),
        _copy(f"{prefix}/v2/cmd/runner/main_test.go"),
        _copy(f"{prefix}/v2/domain/domain_test.go"),
        _copy(f"{prefix}/v2/fixture/codec_test.go"),
        _copy(f"{prefix}/v2/service/service_test.go"),
        _copy(f"{prefix}/v2/store/store_test.go"),
    ]


def _python_specs(task: str) -> list[FileSpec]:
    prefix = BASELINE_ROOTS["python"]
    if task == "UC-001":
        return [
            _overlay(
                "benchmarks/task-starts/overlays/python/uc001/pyproject.toml",
                f"{prefix}/pyproject.toml",
            ),
            _copy(f"{prefix}/uv.lock"),
            _copy(f"{prefix}/v1/__init__.py"),
            _derived(
                f"{prefix}/v1/job_service.py",
                f"{prefix}/v1/job_service.py",
            ),
            _copy(f"{prefix}/tests/__init__.py"),
            _copy(f"{prefix}/tests/test_v1.py"),
        ]
    return [
        _copy(f"{prefix}/pyproject.toml"),
        _copy(f"{prefix}/uv.lock"),
        _copy(f"{prefix}/v1/__init__.py"),
        _copy(f"{prefix}/v1/job_service.py", role="editable"),
        _copy(f"{prefix}/tests/__init__.py"),
        _copy(f"{prefix}/tests/conftest.py"),
        _copy(f"{prefix}/tests/test_fixture.py"),
        _copy(f"{prefix}/tests/test_runner.py"),
        _copy(f"{prefix}/tests/test_service.py"),
        _copy(f"{prefix}/tests/test_store.py"),
        _copy(f"{prefix}/tests/test_v1.py"),
    ]


def _typescript_specs(task: str) -> list[FileSpec]:
    prefix = BASELINE_ROOTS["typescript"]
    common = [
        _overlay(
            "benchmarks/task-starts/overlays/typescript/common/.prettierignore",
            f"{prefix}/.prettierignore",
        ),
        _copy(f"{prefix}/eslint.config.mjs"),
        _copy(f"{prefix}/package-lock.json"),
        _copy(f"{prefix}/package.json"),
    ]
    if task == "UC-001":
        return [
            *common,
            _overlay(
                "benchmarks/task-starts/overlays/typescript/uc001/tsconfig.json",
                f"{prefix}/tsconfig.json",
            ),
            _derived(
                f"{prefix}/v1/job-service.ts",
                f"{prefix}/v1/job-service.ts",
            ),
            _copy(f"{prefix}/tests/v1.test.ts"),
        ]
    return [
        *common,
        _copy(f"{prefix}/tsconfig.json"),
        _copy(f"{prefix}/v1/job-service.ts", role="editable"),
        _copy(f"{prefix}/tests/fixture.test.ts"),
        _copy(f"{prefix}/tests/runner.test.ts"),
        _copy(f"{prefix}/tests/service-store.test.ts"),
        _copy(f"{prefix}/tests/support.ts"),
        _copy(f"{prefix}/tests/v1.test.ts"),
    ]


def _file_specs(language: str, task: str) -> list[FileSpec]:
    if language not in LANGUAGES or task not in TASKS:
        _raise("task_start_configuration_invalid", f"unsupported {language}/{task}")
    language_specs = {
        "rust": _rust_specs,
        "go": _go_specs,
        "python": _python_specs,
        "typescript": _typescript_specs,
    }[language](task)
    specs = [
        _copy(TASK_PATHS[task], path="TASK.md"),
        *language_specs,
        *(_public_fixture_specs() if task == "UC-003" else ()),
    ]
    ordered = sorted(specs, key=lambda spec: spec.path)
    paths = [spec.path for spec in ordered]
    if len(paths) != len(set(paths)):
        _raise(
            "task_start_recipe_invalid",
            f"{language}/{task}: duplicate output path",
        )
    if any(spec.role not in {"editable", "protected"} for spec in ordered):
        _raise("task_start_recipe_invalid", "unknown file protection role")
    return ordered


def _uc001_hole(language: str, source: str) -> str:
    if language == "rust":
        start = source.index("pub fn create_job")
        end = source.index("#[cfg(test)]", start)
        replacement = """pub fn create_job(
    _request: CreateJobRequest,
    _store: &mut impl JobStore,
) -> CreateJobResult {
    todo!("TODO(UC-001): implement create_job and validation")
}

fn validate(_request: &CreateJobRequest) -> Vec<ValidationIssue> {
    todo!("TODO(UC-001): implement validation")
}

fn valid_job_id(_job_id: &str) -> bool {
    todo!("TODO(UC-001): implement validation")
}

"""
        return source[:start] + replacement + source[end:]
    if language == "go":
        source = source.replace(
            'import (\n\t"unicode"\n\t"unicode/utf8"\n)\n\n',
            "",
        )
        start = source.index("// CreateJob validates")
        replacement = """// CreateJob is the explicit UC-001 implementation hole.
func CreateJob(_ CreateJobRequest, _ JobStore) CreateJobResult {
\tpanic("TODO(UC-001): implement CreateJob and validation")
}

func validate(_ CreateJobRequest) []ValidationIssue {
\tpanic("TODO(UC-001): implement validation")
}

func validJobID(_ string) bool {
\tpanic("TODO(UC-001): implement validation")
}
"""
        return source[:start] + replacement
    if language == "python":
        source = source.replace("import re\nimport unicodedata\n", "")
        source = source.replace(
            "from typing import Protocol, assert_never",
            "from typing import Protocol",
        )
        source = source.replace(
            '_JOB_ID_PATTERN = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]{0,63}\\Z")\n\n',
            "",
        )
        start = source.index("def create_job")
        replacement = '''def create_job(
    request: CreateJobRequest,
    store: JobStore,
) -> CreateJobResult:
    """Implement the supplied UC-001 handler contract."""
    del request, store
    raise NotImplementedError("TODO(UC-001): implement create_job")


def _validate(request: CreateJobRequest) -> tuple[ValidationIssue, ...]:
    del request
    raise NotImplementedError("TODO(UC-001): implement validation")
'''
        return source[:start] + replacement
    start = source.index("const jobIdPattern")
    replacement = """export function createJob(
  request: CreateJobRequest,
  store: JobStore,
): CreateJobResult {
  void validate(request);
  void store;
  throw new Error("TODO(UC-001): implement createJob");
}

function validate(
  request: CreateJobRequest,
): readonly ValidationIssue[] {
  void request;
  throw new Error("TODO(UC-001): implement validation");
}
"""
    return source[:start] + replacement


def _render(spec: FileSpec, language: str) -> bytes:
    contents = _repository_path(spec.origin).read_bytes()
    if spec.derivation == "copy":
        return contents
    if spec.derivation != "uc001_implementation_hole":
        _raise("task_start_recipe_invalid", f"unknown derivation {spec.derivation}")
    try:
        source = contents.decode("utf-8")
        return _uc001_hole(language, source).encode("utf-8")
    except (UnicodeDecodeError, ValueError) as error:
        _raise("task_start_recipe_invalid", f"{spec.origin}: {error}")


def build_task_start(
    language: str,
    task: str,
    destination: Path,
) -> None:
    """Materialize one exact agent-visible workspace without deleting content."""

    destination = destination.resolve()
    if destination.exists() and any(destination.iterdir()):
        _raise(
            "task_start_destination_not_empty",
            f"refusing to replace non-empty {destination}",
        )
    destination.mkdir(parents=True, exist_ok=True)
    for spec in _file_specs(language, task):
        output = (destination / spec.path).resolve()
        try:
            output.relative_to(destination)
        except ValueError:
            _raise("task_start_recipe_invalid", f"output leaves workspace: {spec.path}")
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(_render(spec, language))
    validate_workspace(language, task, destination)


def _tree_records(root: Path) -> list[dict[str, str]]:
    records: list[dict[str, str]] = []
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            _raise(
                "task_start_symlink_exposed",
                f"{path.relative_to(root).as_posix()} is a symlink",
            )
        if path.is_file():
            records.append(
                {
                    "path": path.relative_to(root).as_posix(),
                    "sha256": _sha256(path),
                }
            )
    return records


def _tree_digest(records: list[dict[str, str]]) -> str:
    digest = hashlib.sha256()
    for record in records:
        digest.update(f"{record['sha256']}  {record['path']}\n".encode("utf-8"))
    return digest.hexdigest()


def _reference_source_digests(language: str, task: str) -> set[str]:
    digests = {
        _sha256(ROOT / path)
        for path in V2_REFERENCE_SOURCE_FILES
        if f"/{language}/" in f"/{path}/"
    }
    if task == "UC-001":
        digests.add(_sha256(ROOT / UC001_SOURCE_PATHS[language]))
    return digests


def _check_no_reference_source(language: str, task: str, workspace: Path) -> None:
    forbidden = _reference_source_digests(language, task)
    for record in _tree_records(workspace):
        if record["sha256"] in forbidden:
            _raise(
                "task_start_answer_leak",
                f"{language}/{task}: completed reference source at {record['path']}",
            )


def _check_uc001(language: str, workspace: Path) -> None:
    source_path = workspace / UC001_SOURCE_PATHS[language]
    if not source_path.is_file():
        _raise("task_start_incomplete", f"{language}/UC-001: source hole missing")
    source = source_path.read_text(encoding="utf-8")
    if UC001_HOLE_MARKERS[language] not in source:
        _raise(
            "task_start_answer_leak",
            f"{language}/UC-001: explicit implementation hole missing",
        )
    for marker in UC001_CONTRACT_MARKERS[language]:
        if marker not in source:
            _raise(
                "task_start_incomplete",
                f"{language}/UC-001: public contract marker {marker!r} missing",
            )
    prefix = f"{BASELINE_ROOTS[language]}/v2/"
    if any(record["path"].startswith(prefix) for record in _tree_records(workspace)):
        _raise("task_start_answer_leak", f"{language}/UC-001: V2 content exposed")


def _check_uc003(language: str, workspace: Path) -> None:
    for raw_path in UC003_ACCEPTED_V1_FILES[language]:
        output = workspace / raw_path
        if not output.is_file() or _sha256(output) != _sha256(ROOT / raw_path):
            _raise(
                "task_start_v1_changed",
                f"{language}/UC-003: accepted V1 file differs: {raw_path}",
            )

    prefix = f"{BASELINE_ROOTS[language]}/"
    records = _tree_records(workspace)
    if language == "rust":
        leaked = [
            record["path"]
            for record in records
            if record["path"].startswith(f"{prefix}v2/src/")
            or (
                record["path"].startswith(f"{prefix}v2/")
                and record["path"].endswith(".rs")
                and "/tests/" not in record["path"]
            )
        ]
        visible_tests = [
            record["path"]
            for record in records
            if record["path"].startswith(f"{prefix}v2/tests/")
            and record["path"].endswith(".rs")
        ]
    elif language == "go":
        leaked = [
            record["path"]
            for record in records
            if record["path"].startswith(f"{prefix}v2/")
            and record["path"].endswith(".go")
            and not record["path"].endswith("_test.go")
        ]
        visible_tests = [
            record["path"]
            for record in records
            if record["path"].startswith(f"{prefix}v2/")
            and record["path"].endswith("_test.go")
        ]
    else:
        leaked = [
            record["path"]
            for record in records
            if record["path"].startswith(f"{prefix}v2/")
        ]
        suffix = ".py" if language == "python" else ".ts"
        visible_tests = [
            record["path"]
            for record in records
            if "/tests/" in record["path"]
            and record["path"].endswith(suffix)
            and not record["path"].endswith(
                "__init__.py" if language == "python" else "v1.test.ts"
            )
        ]
    if leaked:
        _raise(
            "task_start_answer_leak",
            f"{language}/UC-003: V2 implementation source exposed at {leaked[0]}",
        )
    if not visible_tests:
        _raise(
            "task_start_incomplete",
            f"{language}/UC-003: no ordinary V2 task checks",
        )


def validate_workspace(language: str, task: str, workspace: Path) -> None:
    """Enforce the agent-visible package boundary independently of its lock."""

    records = _tree_records(workspace)
    if not records:
        _raise("task_start_incomplete", f"{language}/{task}: workspace is empty")
    baseline_root = workspace / "benchmarks" / "baselines"
    visible_languages = (
        sorted(path.name for path in baseline_root.iterdir() if path.is_dir())
        if baseline_root.is_dir()
        else []
    )
    if visible_languages != [language]:
        _raise(
            "task_start_other_language_exposed",
            f"{language}/{task}: visible baselines are {visible_languages}",
        )
    if any(
        Path(record["path"]).name in FORBIDDEN_WORKSPACE_NAMES for record in records
    ):
        _raise(
            "task_start_freeze_metadata_exposed",
            f"{language}/{task}: freeze-only file is visible",
        )
    for record in records:
        path = workspace / record["path"]
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        for marker in FORBIDDEN_CONTENT_MARKERS:
            if marker in text:
                _raise(
                    "task_start_freeze_metadata_exposed",
                    f"{language}/{task}: {marker!r} appears in {record['path']}",
                )
    fixture_paths = [
        record["path"]
        for record in records
        if record["path"].startswith("benchmarks/fixtures/")
        and record["path"] != "benchmarks/fixtures/manifest.json"
    ]
    if any(
        not path.startswith("benchmarks/fixtures/public/") for path in fixture_paths
    ):
        _raise(
            "task_start_private_fixture_exposed",
            f"{language}/{task}: non-public fixture is visible",
        )
    if task == "UC-001" and fixture_paths:
        _raise(
            "task_start_incomplete",
            f"{language}/UC-001: unrelated fixture corpus is visible",
        )
    if task == "UC-003":
        manifest = _load_object(
            workspace / "benchmarks" / "fixtures" / "manifest.json",
            "task_start_incomplete",
        )
        paths = [
            entry.get("path")
            for entry in manifest.get("fixtures", ())
            if isinstance(entry, dict)
        ]
        if (
            len(paths) != 37
            or any(
                not isinstance(path, str)
                or not path.startswith("benchmarks/fixtures/public/")
                for path in paths
            )
            or any(not (workspace / path).is_file() for path in paths)
        ):
            _raise(
                "task_start_private_fixture_exposed",
                f"{language}/UC-003: public fixture package is incomplete",
            )
    _check_no_reference_source(language, task, workspace)
    if task == "UC-001":
        _check_uc001(language, workspace)
    else:
        _check_uc003(language, workspace)


def _starting_state_checks(language: str, task: str) -> list[dict[str, Any]]:
    prefix = BASELINE_ROOTS[language]
    checks: dict[tuple[str, str], list[dict[str, Any]]] = {
        (
            "rust",
            "UC-001",
        ): [
            {
                "id": "public-task-tests",
                "working_directory": ".",
                "command": [
                    "rustup",
                    "run",
                    "1.88.0",
                    "cargo",
                    "test",
                    "--offline",
                    "--locked",
                    "--manifest-path",
                    f"{prefix}/Cargo.toml",
                    "-p",
                    "ail-job-service-v1",
                ],
                "expected_exit": "nonzero",
                "output_contains": "TODO(UC-001)",
            }
        ],
        (
            "rust",
            "UC-003",
        ): [
            {
                "id": "accepted-v1-tests",
                "working_directory": ".",
                "command": [
                    "rustup",
                    "run",
                    "1.88.0",
                    "cargo",
                    "test",
                    "--offline",
                    "--locked",
                    "--manifest-path",
                    f"{prefix}/Cargo.toml",
                    "-p",
                    "ail-job-service-v1",
                ],
                "expected_exit": "zero",
            },
            {
                "id": "v2-task-incomplete",
                "working_directory": ".",
                "command": [
                    "rustup",
                    "run",
                    "1.88.0",
                    "cargo",
                    "test",
                    "--offline",
                    "--locked",
                    "--manifest-path",
                    f"{prefix}/Cargo.toml",
                    "--workspace",
                ],
                "expected_exit": "nonzero",
                "output_contains": "v2/src/lib.rs",
            },
        ],
        (
            "go",
            "UC-001",
        ): [
            {
                "id": "public-task-tests",
                "working_directory": ".",
                "command": ["go", "-C", prefix, "test", "./v1"],
                "expected_exit": "nonzero",
                "output_contains": "TODO(UC-001)",
            }
        ],
        (
            "go",
            "UC-003",
        ): [
            {
                "id": "accepted-v1-tests",
                "working_directory": ".",
                "command": ["go", "-C", prefix, "test", "./v1"],
                "expected_exit": "zero",
            },
            {
                "id": "v2-task-incomplete",
                "working_directory": ".",
                "command": ["go", "-C", prefix, "test", "./v2/..."],
                "expected_exit": "nonzero",
            },
        ],
        (
            "python",
            "UC-001",
        ): [
            {
                "id": "public-task-tests",
                "working_directory": prefix,
                "command": [
                    "python3",
                    "-m",
                    "pytest",
                    "-q",
                    "-p",
                    "no:cacheprovider",
                    "tests/test_v1.py",
                ],
                "expected_exit": "nonzero",
                "output_contains": "TODO(UC-001)",
            }
        ],
        (
            "python",
            "UC-003",
        ): [
            {
                "id": "accepted-v1-tests",
                "working_directory": prefix,
                "command": [
                    "python3",
                    "-m",
                    "pytest",
                    "-q",
                    "-p",
                    "no:cacheprovider",
                    "tests/test_v1.py",
                ],
                "expected_exit": "zero",
            },
            {
                "id": "v2-task-incomplete",
                "working_directory": prefix,
                "command": [
                    "python3",
                    "-m",
                    "pytest",
                    "-q",
                    "-p",
                    "no:cacheprovider",
                ],
                "expected_exit": "nonzero",
                "output_contains": "No module named 'v2'",
            },
        ],
        (
            "typescript",
            "UC-001",
        ): [
            {
                "id": "public-task-tests",
                "working_directory": prefix,
                "command": [
                    "node_modules/.bin/tsx",
                    "--test",
                    "tests/v1.test.ts",
                ],
                "expected_exit": "nonzero",
                "output_contains": "TODO(UC-001)",
            }
        ],
        (
            "typescript",
            "UC-003",
        ): [
            {
                "id": "accepted-v1-tests",
                "working_directory": prefix,
                "command": [
                    "node_modules/.bin/tsx",
                    "--test",
                    "tests/v1.test.ts",
                ],
                "expected_exit": "zero",
            },
            {
                "id": "v2-task-incomplete",
                "working_directory": prefix,
                "command": [
                    "node_modules/.bin/tsx",
                    "--test",
                    "tests/fixture.test.ts",
                    "tests/runner.test.ts",
                    "tests/service-store.test.ts",
                ],
                "expected_exit": "nonzero",
                "output_contains": "ERR_MODULE_NOT_FOUND",
            },
        ],
    }
    return checks[(language, task)]


def _configuration_value(
    language: str,
    task: str,
    workspace: Path,
) -> dict[str, Any]:
    specs = _file_specs(language, task)
    records = _tree_records(workspace)
    records_by_path = {record["path"]: record["sha256"] for record in records}
    files = [
        {
            "path": spec.path,
            "sha256": records_by_path[spec.path],
            "role": spec.role,
            "origin": spec.origin,
            "derivation": spec.derivation,
        }
        for spec in specs
    ]
    return {
        "id": f"{language}-{task.lower().replace('-', '')}",
        "language": language,
        "task": task,
        "task_path": "TASK.md",
        "tree_sha256": _tree_digest(records),
        "files": files,
        "starting_state_checks": _starting_state_checks(language, task),
    }


def task_start_lock_value() -> dict[str, Any]:
    configurations: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="ail-m7-task-start-lock-") as raw:
        temporary = Path(raw)
        for language in LANGUAGES:
            for task in TASKS:
                workspace = temporary / language / task.lower()
                build_task_start(language, task, workspace)
                configurations.append(_configuration_value(language, task, workspace))
    return {
        "task_start_lock_format": 1,
        "tree_digest_algorithm": TREE_DIGEST_ALGORITHM,
        "generator": {
            "path": "benchmarks/tools/task_starts.py",
            "sha256": _sha256(Path(__file__)),
        },
        "configurations": configurations,
    }


def write_task_start_lock() -> None:
    LOCK_PATH.write_text(_canonical(task_start_lock_value()), encoding="utf-8")
    print(
        f"Wrote {LOCK_PATH.relative_to(ROOT)} with "
        f"{len(LANGUAGES) * len(TASKS)} configurations."
    )


def _configuration_map(lock: dict[str, Any]) -> dict[tuple[str, str], dict[str, Any]]:
    configurations = lock.get("configurations")
    if not isinstance(configurations, list):
        _raise("task_start_lock_invalid", "configurations must be an array")
    result: dict[tuple[str, str], dict[str, Any]] = {}
    for configuration in configurations:
        if not isinstance(configuration, dict):
            _raise("task_start_lock_invalid", "configuration must be an object")
        key = (configuration.get("language"), configuration.get("task"))
        if not all(isinstance(value, str) for value in key) or key in result:
            _raise("task_start_lock_invalid", "configuration identity is invalid")
        result[key] = configuration
    return result


def verify_locked_workspace(
    workspace: Path,
    configuration: dict[str, Any],
) -> None:
    files = configuration.get("files")
    if not isinstance(files, list) or not files:
        _raise("task_start_lock_invalid", "explicit file manifest is empty")
    paths = [entry.get("path") for entry in files if isinstance(entry, dict)]
    if paths != sorted(paths) or len(paths) != len(set(paths)):
        _raise(
            "task_start_lock_invalid",
            "explicit file manifest paths must be unique lexical order",
        )
    for entry in files:
        if (
            not isinstance(entry, dict)
            or list(entry) != ["path", "sha256", "role", "origin", "derivation"]
            or entry["role"] not in {"editable", "protected"}
        ):
            _raise("task_start_lock_invalid", "file manifest entry is invalid")
        path = workspace / entry["path"]
        if not path.is_file() or path.is_symlink():
            _raise(
                "task_start_protected_artifact_changed",
                f"missing locked file {entry['path']}",
            )
        if _sha256(path) != entry["sha256"]:
            _raise(
                "task_start_protected_artifact_changed",
                f"digest differs for {entry['path']}",
            )
    records = _tree_records(workspace)
    if [record["path"] for record in records] != paths:
        _raise(
            "task_start_completeness_changed",
            "workspace files differ from explicit manifest",
        )
    if _tree_digest(records) != configuration.get("tree_sha256"):
        _raise("task_start_tree_changed", "independent tree digest differs")


def _check_lock_shape(lock: dict[str, Any]) -> None:
    if list(lock) != [
        "task_start_lock_format",
        "tree_digest_algorithm",
        "generator",
        "configurations",
    ]:
        _raise("task_start_lock_invalid", "unexpected top-level lock shape")
    if (
        lock["task_start_lock_format"] != 1
        or lock["tree_digest_algorithm"] != TREE_DIGEST_ALGORITHM
    ):
        _raise("task_start_lock_invalid", "lock format or digest algorithm differs")
    expected_pairs = [(language, task) for language in LANGUAGES for task in TASKS]
    actual_pairs = [
        (entry.get("language"), entry.get("task"))
        for entry in lock["configurations"]
        if isinstance(entry, dict)
    ]
    if actual_pairs != expected_pairs:
        _raise("task_start_lock_invalid", "eight-configuration matrix is incomplete")
    digests = [
        entry.get("tree_sha256")
        for entry in lock["configurations"]
        if isinstance(entry, dict)
    ]
    if len(digests) != len(set(digests)):
        _raise("task_start_lock_invalid", "tree digests are not independent")


def check_task_start_lock(*, run_starting_state: bool = False) -> None:
    lock = _load_object(LOCK_PATH, "task_start_lock_missing")
    if LOCK_PATH.read_text(encoding="utf-8") != _canonical(lock):
        _raise("task_start_lock_invalid", "lock must be canonical two-space JSON")
    _check_lock_shape(lock)
    expected = task_start_lock_value()
    if lock != expected:
        _raise("task_start_lock_changed", "task-start package lock differs")
    configurations = _configuration_map(lock)
    with tempfile.TemporaryDirectory(prefix="ail-m7-task-start-check-") as raw:
        temporary = Path(raw)
        for language in LANGUAGES:
            for task in TASKS:
                first = temporary / "first" / language / task.lower()
                second = temporary / "second" / language / task.lower()
                build_task_start(language, task, first)
                build_task_start(language, task, second)
                first_records = _tree_records(first)
                second_records = _tree_records(second)
                if first_records != second_records:
                    _raise(
                        "task_start_nondeterministic",
                        f"{language}/{task}: independent rebuilds differ",
                    )
                configuration = configurations[(language, task)]
                verify_locked_workspace(first, configuration)
                verify_locked_workspace(second, configuration)
                if run_starting_state:
                    _run_starting_state(language, task, first, configuration)
    detail = " and starting states" if run_starting_state else ""
    print(f"M7 task starts passed: 8 deterministic answer-free packages{detail}.")


def _exact_python() -> Path:
    path = ROOT / "benchmarks" / "baselines" / "python" / ".venv" / "bin" / "python"
    if not path.is_file():
        _raise(
            "task_start_toolchain_missing",
            f"frozen Python test environment is unavailable at {path}",
        )
    completed = subprocess.run(
        [
            str(path),
            "-c",
            "import pytest,sys; print(sys.version.split()[0], pytest.__version__)",
        ],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if completed.returncode != 0 or completed.stdout.strip() != "3.13.5 8.4.1":
        _raise(
            "task_start_toolchain_changed",
            f"expected CPython 3.13.5 and pytest 8.4.1, got {completed.stdout.strip()!r}",
        )
    return path


def _exact_tsx() -> Path:
    baseline = ROOT / "benchmarks" / "baselines" / "typescript"
    path = baseline / "node_modules" / ".bin" / "tsx"
    package = baseline / "node_modules" / "tsx" / "package.json"
    if not path.is_file() or not package.is_file():
        _raise(
            "task_start_toolchain_missing",
            f"frozen TypeScript test environment is unavailable at {baseline / 'node_modules'}",
        )
    value = _load_object(package, "task_start_toolchain_changed")
    if value.get("version") != "4.20.3":
        _raise(
            "task_start_toolchain_changed",
            f"expected tsx 4.20.3, got {value.get('version')!r}",
        )
    completed = subprocess.run(
        ["node", "--version"],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if completed.returncode != 0 or completed.stdout.strip() != "v23.10.0":
        _raise(
            "task_start_toolchain_changed",
            f"expected Node.js v23.10.0, got {completed.stdout.strip()!r}",
        )
    return path


def _resolve_check_command(command: list[str]) -> list[str]:
    if command[0] == "python3":
        return [str(_exact_python()), *command[1:]]
    if command[0] == "node_modules/.bin/tsx":
        return [str(_exact_tsx()), *command[1:]]
    return command


def _run_starting_state(
    language: str,
    task: str,
    workspace: Path,
    configuration: dict[str, Any],
) -> None:
    workspace = workspace.resolve()
    subprocess.run(
        ["git", "init", "--quiet"],
        cwd=workspace,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    environment = os.environ.copy()
    environment.update(
        {
            "CARGO_NET_OFFLINE": "true",
            "CARGO_TERM_COLOR": "never",
            "GOCACHE": "/tmp/ail-go-build-cache",
            "GOTOOLCHAIN": "local",
            "LANG": "C",
            "LC_ALL": "C",
            "NO_COLOR": "1",
            "PYTHONDONTWRITEBYTECODE": "1",
            "PYTHONHASHSEED": "0",
            "TMPDIR": "/tmp",
        }
    )
    checks = configuration.get("starting_state_checks")
    if not isinstance(checks, list) or not checks:
        _raise("task_start_lock_invalid", "starting-state checks are missing")
    for check in checks:
        command = _resolve_check_command(check["command"])
        working_directory = (workspace / check["working_directory"]).resolve()
        try:
            working_directory.relative_to(workspace)
        except ValueError:
            _raise(
                "task_start_lock_invalid", "check working directory escapes workspace"
            )
        try:
            completed = subprocess.run(
                command,
                cwd=working_directory,
                env=environment,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                encoding="utf-8",
                timeout=180,
                check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            _raise(
                "task_start_check_failed",
                f"{language}/{task}/{check['id']}: {error}",
            )
        expected_zero = check["expected_exit"] == "zero"
        if (completed.returncode == 0) != expected_zero:
            _raise(
                "task_start_check_failed",
                f"{language}/{task}/{check['id']}: exit {completed.returncode}, "
                f"expected {check['expected_exit']}",
            )
        output = completed.stdout + completed.stderr
        marker = check.get("output_contains")
        if marker is not None and marker not in output:
            _raise(
                "task_start_check_failed",
                f"{language}/{task}/{check['id']}: output omitted {marker!r}",
            )
    for entry in configuration["files"]:
        if entry["role"] != "protected":
            continue
        path = workspace / entry["path"]
        if not path.is_file() or _sha256(path) != entry["sha256"]:
            _raise(
                "task_start_protected_artifact_changed",
                f"{language}/{task}: starting checks changed {entry['path']}",
            )
    print(f"PASS {language}/{task} starting state")


def freeze_artifacts() -> tuple[str, ...]:
    """Return the explicit M7 artifacts that define and lock task packages."""

    for path in TASK_START_DEFINITION_ARTIFACTS:
        _repository_path(path)
    return TASK_START_DEFINITION_ARTIFACTS


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    build = subparsers.add_parser("build", help="materialize one locked workspace")
    build.add_argument("--language", required=True, choices=LANGUAGES)
    build.add_argument("--task", required=True, choices=TASKS)
    build.add_argument("--output", required=True, type=Path)

    check = subparsers.add_parser("check", help="verify every task-start package")
    check.add_argument("--run-starting-state", action="store_true")

    lock = subparsers.add_parser("lock", help="manage the task-start digest lock")
    lock_mode = lock.add_mutually_exclusive_group(required=True)
    lock_mode.add_argument("--write", action="store_true")
    lock_mode.add_argument("--check", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        if args.command == "build":
            build_task_start(args.language, args.task, args.output)
            records = _tree_records(args.output.resolve())
            print(
                f"Built {args.language}/{args.task}: {len(records)} files, "
                f"{_tree_digest(records)}."
            )
        elif args.command == "check":
            check_task_start_lock(run_starting_state=args.run_starting_state)
        elif args.write:
            write_task_start_lock()
        else:
            check_task_start_lock()
    except TaskStartError as error:
        print(f"ERROR [{error.code}]: {error.message}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
