from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any, cast


def tree_digest(root: Path, files: list[str]) -> str:
    records = "".join(
        f"{hashlib.sha256((root / path).read_bytes()).hexdigest()}  {path}\n"
        for path in files
    )
    return hashlib.sha256(records.encode()).hexdigest()


def load(path: Path) -> dict[str, Any]:
    return cast(
        "dict[str, Any]",
        json.loads(path.read_text(encoding="utf-8")),
    )


def test_checkpoint_source_trees_match_frozen_digests(
    repository_root: Path,
) -> None:
    manifest = load(repository_root / "benchmarks/baselines/python/checkpoints.json")
    checkpoints = cast("list[dict[str, Any]]", manifest["checkpoints"])
    assert len(checkpoints) == 2
    for checkpoint in checkpoints:
        files = cast("list[str]", checkpoint["files"])
        assert tree_digest(repository_root, files) == checkpoint["source_tree_sha256"]


def test_seed_locations_cover_every_frozen_category_once(
    repository_root: Path,
) -> None:
    hidden = load(repository_root / "benchmarks/contracts/hidden-contract.json")
    locations = load(
        repository_root / "benchmarks/baselines/python/seed-locations.json"
    )
    assert locations["language"] == "python"
    categories = cast("list[dict[str, Any]]", hidden["seed_categories"])
    seeds = cast("list[dict[str, Any]]", locations["locations"])
    assert [seed["seed_id"] for seed in seeds] == [
        category["id"] for category in categories
    ]
    assert all(seed["semantic_locations"] for seed in seeds)
