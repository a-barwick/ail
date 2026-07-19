#!/usr/bin/env python3
"""Check local Markdown links and roadmap/status milestone consistency."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote


ROOT = Path(__file__).resolve().parents[1]
MARKDOWN_LINK = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")
EXTERNAL_SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")
MILESTONE_ROW = re.compile(
    r"^\|\s*(M\d+)\s*\|.*\|\s*(Complete|Active|Planned|Deferred)\s*\|",
    re.MULTILINE,
)
MILESTONE_HEADING = re.compile(r"^###\s+(M\d+)\s+.+$", re.MULTILINE)
MILESTONE_STATUS = re.compile(
    r"^\*\*Status:\*\*\s+(Complete|Active|Planned|Deferred)\s*$",
    re.MULTILINE,
)


def markdown_files() -> list[Path]:
    return sorted(
        path
        for path in ROOT.rglob("*.md")
        if ".git" not in path.parts
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


def milestone_errors() -> list[str]:
    errors: list[str] = []
    roadmap_path = ROOT / "docs" / "roadmap.md"
    status_path = ROOT / "docs" / "STATUS.md"
    roadmap = roadmap_path.read_text(encoding="utf-8")
    status = status_path.read_text(encoding="utf-8")

    rows = dict(MILESTONE_ROW.findall(roadmap))
    active = [milestone for milestone, state in rows.items() if state == "Active"]
    if len(active) != 1:
        errors.append(
            "docs/roadmap.md: dependency map must contain exactly one Active "
            f"milestone, found {active}"
        )

    status_match = re.search(
        r"^## Active milestone\s*$\s*^(M\d+)\s+.+$",
        status,
        re.MULTILINE,
    )
    if not status_match:
        errors.append("docs/STATUS.md: cannot find the active milestone")
    elif active and status_match.group(1) != active[0]:
        errors.append(
            "docs/STATUS.md: active milestone "
            f"{status_match.group(1)} does not match roadmap {active[0]}"
        )

    headings = MILESTONE_HEADING.findall(roadmap)
    status_blocks = MILESTONE_STATUS.findall(roadmap)
    if len(headings) != len(status_blocks):
        errors.append(
            "docs/roadmap.md: each milestone heading must have one status "
            f"({len(headings)} headings, {len(status_blocks)} statuses)"
        )
    elif headings != list(rows):
        errors.append(
            "docs/roadmap.md: detailed milestone order does not match the "
            "dependency map"
        )
    else:
        for milestone, detailed_status in zip(headings, status_blocks):
            if rows[milestone] != detailed_status:
                errors.append(
                    f"docs/roadmap.md: {milestone} status differs between "
                    f"dependency map ({rows[milestone]}) and detail "
                    f"({detailed_status})"
                )

    forbidden_headings = ("## Current discovery gate", "## Phase ")
    for heading in forbidden_headings:
        if heading in roadmap:
            errors.append(
                "docs/roadmap.md: retired operational heading remains: "
                f"{heading}"
            )
    return errors


def main() -> int:
    errors = local_link_errors() + milestone_errors()
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print(
        f"Documentation check passed: {len(markdown_files())} Markdown files, "
        "local links valid, milestone status aligned."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
