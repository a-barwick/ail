#!/usr/bin/env python3
"""Check local links, decision records, and milestone consistency."""

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
MILESTONE_ROW = re.compile(
    r"^\|\s*(M\d+)\s*\|.*\|\s*(Complete|Active|Planned|Deferred)\s*\|",
    re.MULTILINE,
)
MILESTONE_HEADING = re.compile(r"^###\s+(M\d+)\s+.+$", re.MULTILINE)
MILESTONE_STATUS = re.compile(
    r"^\*\*Status:\*\*\s+(Complete|Active|Planned|Deferred)\s*$",
    re.MULTILINE,
)
ADR_FILENAME = re.compile(r"^(\d{4})-[a-z0-9-]+\.md$")
ADR_TITLE = re.compile(r"^# ADR (\d{4}): .+$", re.MULTILINE)
ADR_METADATA = (
    "- Status:",
    "- Date:",
    "- Owners:",
    "- Documentation layer and scope:",
)
ADR_SECTIONS = (
    "## Context",
    "## Decision",
    "## Consequences",
    "## Alternatives considered",
    "## Validation",
)


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

        for metadata in ADR_METADATA:
            pattern = (
                rf"^{re.escape(metadata)}(?:\s+.*)?$"
                if decision_id == "0000"
                else rf"^{re.escape(metadata)}\s+.+$"
            )
            if not re.search(pattern, text, re.MULTILINE):
                errors.append(
                    f"{path.relative_to(ROOT)}: missing decision metadata "
                    f"'{metadata}'"
                )

        section_positions = [text.find(section) for section in ADR_SECTIONS]
        for section, position in zip(ADR_SECTIONS, section_positions):
            if position == -1:
                errors.append(
                    f"{path.relative_to(ROOT)}: missing decision section "
                    f"'{section}'"
                )
        present_positions = [
            position for position in section_positions if position != -1
        ]
        if present_positions != sorted(present_positions):
            errors.append(
                f"{path.relative_to(ROOT)}: decision sections are out of order"
            )

    return errors


def main() -> int:
    errors = local_link_errors() + milestone_errors() + decision_errors()
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print(
        f"Documentation check passed: {len(markdown_files())} Markdown files, "
        "local links valid, decision records structured, milestone status "
        "aligned."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
