#!/usr/bin/env python3
"""Check local Markdown links and decision-record filenames."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote


ROOT = Path(__file__).resolve().parents[1]
MARKDOWN_LINK = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")
EXTERNAL_SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")
IGNORED_DIRECTORIES = {
    ".git",
    ".venv",
    "coverage",
    "dist",
    "node_modules",
    "target",
}
ADR_FILENAME = re.compile(r"^(\d{4})-[a-z0-9-]+\.md$")
ADR_TITLE = re.compile(r"^# ADR (\d{4}): .+$", re.MULTILINE)


def markdown_files() -> list[Path]:
    return sorted(
        path
        for path in ROOT.rglob("*.md")
        if not IGNORED_DIRECTORIES.intersection(path.parts)
    )


def local_link_errors() -> list[str]:
    errors: list[str] = []
    for source in markdown_files():
        text = source.read_text(encoding="utf-8")
        for raw_target in MARKDOWN_LINK.findall(text):
            target = raw_target.strip()
            if target.startswith("<") and target.endswith(">"):
                target = target[1:-1]
            if not target or target.startswith("#") or EXTERNAL_SCHEME.match(target):
                continue

            path_text = unquote(target.split("#", 1)[0])
            destination = (source.parent / path_text).resolve()
            try:
                destination.relative_to(ROOT)
            except ValueError:
                errors.append(
                    f"{source.relative_to(ROOT)}: local link leaves repository: "
                    f"{raw_target}"
                )
                continue
            if not destination.exists():
                errors.append(
                    f"{source.relative_to(ROOT)}: missing local link target: "
                    f"{raw_target}"
                )
    return errors


def decision_errors() -> list[str]:
    errors: list[str] = []
    decisions = ROOT / "docs" / "decisions"
    seen_ids: dict[str, Path] = {}

    for path in sorted(decisions.glob("*.md")):
        filename_match = ADR_FILENAME.fullmatch(path.name)
        if not filename_match:
            errors.append(
                f"{path.relative_to(ROOT)}: decision filename must use "
                "NNNN-lowercase-kebab-case.md"
            )
            continue

        decision_id = filename_match.group(1)
        if decision_id in seen_ids:
            errors.append(
                f"{path.relative_to(ROOT)}: duplicate ADR {decision_id}; first "
                f"used by {seen_ids[decision_id].relative_to(ROOT)}"
            )
        else:
            seen_ids[decision_id] = path

        text = path.read_text(encoding="utf-8")
        title_match = ADR_TITLE.search(text)
        if not title_match:
            errors.append(
                f"{path.relative_to(ROOT)}: missing '# ADR {decision_id}: ...' "
                "title"
            )
        elif title_match.group(1) != decision_id:
            errors.append(
                f"{path.relative_to(ROOT)}: title ADR {title_match.group(1)} "
                f"does not match filename {decision_id}"
            )
        if "## Decision" not in text:
            errors.append(
                f"{path.relative_to(ROOT)}: missing '## Decision' section"
            )

    return errors


def main() -> int:
    errors = local_link_errors() + decision_errors()
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print(
        f"Documentation check passed: {len(markdown_files())} Markdown files, "
        "local links valid, decision filenames unique."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
