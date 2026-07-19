"""Language-neutral one-case and corpus runner."""

from __future__ import annotations

import hashlib
import json
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import TextIO, cast

from v2.fixture import FixtureError, JsonObject, run_case


class RunnerError(RuntimeError):
    """A runner input or process-contract error."""


def repository_root(start: Path | None = None) -> Path:
    current = (start or Path.cwd()).resolve()
    for candidate in (current, *current.parents):
        if (candidate / ".git").is_dir():
            return candidate
    raise RunnerError("runner working directory is not inside the repository")


def run(arguments: Sequence[str]) -> JsonObject:
    if len(arguments) != 2 or arguments[0] not in {"--case", "--corpus"}:
        raise RunnerError("expected exactly --case <fixture> or --corpus <manifest>")
    path = Path(arguments[1])
    root = repository_root()
    resolved = path if path.is_absolute() else root / path
    if arguments[0] == "--case":
        return _run_case_file(resolved)
    return _run_corpus(resolved, root)


def _load_json(path: Path) -> object:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise RunnerError(f"could not read or parse {path}: {error}") from error


def _run_case_file(path: Path) -> JsonObject:
    try:
        return run_case(_load_json(path))
    except FixtureError as error:
        raise RunnerError(f"could not run {path}: {error}") from error


def _run_corpus(path: Path, root: Path) -> JsonObject:
    raw_bytes = path.read_bytes()
    manifest = _load_json(path)
    if not isinstance(manifest, dict) or not isinstance(manifest.get("fixtures"), list):
        raise RunnerError(f"could not parse {path}: fixtures must be an array")
    entries = cast("list[object]", manifest["fixtures"])
    results: list[JsonObject] = []
    for entry in entries:
        if not isinstance(entry, dict) or not isinstance(entry.get("path"), str):
            raise RunnerError("fixture manifest entry must contain a string path")
        fixture_path = cast("str", entry["path"])
        results.append(_run_case_file(root / fixture_path))
    return {
        "result_format": 1,
        "fixture_manifest_sha256": hashlib.sha256(raw_bytes).hexdigest(),
        "results": results,
    }


def run_cli(
    arguments: Sequence[str],
    stdout: TextIO = sys.stdout,
    stderr: TextIO = sys.stderr,
) -> int:
    try:
        result = run(arguments)
        json.dump(result, stdout, ensure_ascii=False, separators=(",", ":"))
        stdout.write("\n")
    except (OSError, RunnerError) as error:
        print(f"python baseline runner: {error}", file=stderr)
        return 1
    return 0


def main() -> int:
    return run_cli(sys.argv[1:])


if __name__ == "__main__":
    raise SystemExit(main())
