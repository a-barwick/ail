#!/usr/bin/env python3
"""Two-arm convergence harness for one broken AIL workspace.

Both arms get the same broken workspace, the same frozen task specification,
and the same success gate: `ailc publish` must succeed while the specification
still holds. The arms differ in one way only.

- Arm `ail`: every `check` and `publish` returns the compiler's own output.
- Arm `control`: every `check` and `publish` returns `PASS` or `FAIL`. No
  diagnostic code, no `expected.type`, no architecture rule, no count of
  findings, no stage of failure.

The harness is the arm's only access to the workspace. The authoritative source
lives in `runs/<arm>/state.json` and is materialized into a private temporary
directory only while `ailc` runs, so no on-disk workspace exists for an arm to
compile behind the harness's back.

The harness never repairs source and never edits policy. It measures.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from statistics import median

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
FIXTURE = HERE / "fixture" / "broken"
CONTRACT = HERE / "fixture" / "contract.json"
RUNS = HERE / "runs"
POLICIES = ("ail", "control")
ARM_NAME = re.compile(r"^(?P<policy>ail|control)(?:-t(?P<trial>\d+))?$")
ATTEMPT_LIMIT = 40

# Reference material both arms may read through the harness. Identical for the
# two arms, so it cannot explain a difference between them.
REFERENCE_READS = (
    "docs/language.md",
    "docs/STATUS.md",
    "specs/core.md",
    "specs/architecture.md",
    "compiler/examples/architecture-denied/architecture.json",
    "compiler/examples/architecture-denied/transport.ail",
    "compiler/examples/architecture-denied/domain.ail",
    "compiler/examples/architecture-denied/contracts.ail",
    "compiler/examples/composed-service/domain.ail",
    "compiler/examples/composed-service/service.ail",
    "compiler/examples/composed-service/validation.ail",
    "compiler/examples/batch-cancellation/domain.ail",
    "compiler/examples/batch-cancellation/service.ail",
    "compiler/examples/batch-cancellation/single.ail",
)

# Approximation of the cl100k_base pre-tokenizer. Counting its matches is a
# deterministic, offline, model-free lower bound on BPE token count. Raw
# character counts are reported beside it so any reader can recompute with a
# real tokenizer.
PRETOKEN = re.compile(
    r"'(?:[sdmt]|ll|ve|re)|[^\r\n\w]?\w+|\d{1,3}| ?[^\s\w]+[\r\n]*|\s*[\r\n]+|\s+(?!\S)|\s+"
)

SOURCE_DIAGNOSTIC = re.compile(
    r"^(?P<code>AIL\.[A-Z0-9_.]+):(?P<path>[^:]+):(?P<start>\d+):(?P<end>\d+):(?P<details>.*)$"
)
ARCH_DIAGNOSTIC = re.compile(
    r"^(?P<code>AIL\.ARCH\.[A-Z0-9_]+):(?P<scope>.+?):(?P<rule>M\d+-[A-Z0-9-]+):(?P<facts>.*)$"
)
ARCH_INCOMPLETE = re.compile(r"^(?P<code>AIL\.ARCH\.ANALYSIS_INCOMPLETE)\s*(?P<reason>.*)$")
BARE_DIAGNOSTIC = re.compile(r"^(?P<code>AIL\.[A-Z0-9_.]+)\b(?P<rest>.*)$")

# Lines `ailc` prints that are summaries or success output, not findings.
TRAILER_LINES = (
    "ok",
    "published",
    "source contains check diagnostics",
    "source contains architecture diagnostics",
)


def tokens(text: str) -> int:
    return len(PRETOKEN.findall(text))


def digest(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def strip_comments(text: str) -> str:
    """Remove AIL line and block comments, leaving text literals intact.

    Without this, a requirement stated as source text could be satisfied by
    commenting the source text out, which is not an implementation.
    """
    out: list[str] = []
    index = 0
    end = len(text)
    while index < end:
        char = text[index]
        if char == '"':
            out.append(char)
            index += 1
            while index < end:
                out.append(text[index])
                if text[index] == "\\" and index + 1 < end:
                    out.append(text[index + 1])
                    index += 2
                    continue
                if text[index] == '"':
                    index += 1
                    break
                index += 1
            continue
        if text.startswith("//", index):
            while index < end and text[index] != "\n":
                index += 1
            continue
        if text.startswith("/*", index):
            closed = text.find("*/", index + 2)
            index = end if closed == -1 else closed + 2
            out.append(" ")
            continue
        out.append(char)
        index += 1
    return "".join(out)


def normalize(text: str) -> str:
    """Collapse whitespace, drop comments, drop the `contracts.` qualifier.

    The task specification is matched against this form so a requirement does
    not dictate which module a declaration lives in or how it is indented.
    """
    stripped = strip_comments(text).replace("contracts.", "")
    return re.sub(r"\s+", " ", stripped).strip()


def policy_of(arm_name: str) -> str:
    matched = ARM_NAME.match(arm_name)
    if not matched:
        fail(
            f"unknown arm {arm_name}; expected ail, control, or a trial such as "
            "ail-t2 or control-t3"
        )
    return matched.group("policy")


@dataclass
class Arm:
    name: str
    path: Path

    @property
    def policy(self) -> str:
        return policy_of(self.name)

    @property
    def state_path(self) -> Path:
        return self.path / "state.json"

    def load(self) -> dict:
        if not self.state_path.is_file():
            fail(f"arm {self.name} has not started; run: harness.py start --arm {self.name}")
        return json.loads(self.state_path.read_text(encoding="utf-8"))

    def save(self, state: dict) -> None:
        self.path.mkdir(parents=True, exist_ok=True)
        self.state_path.write_text(json.dumps(state, indent=2) + "\n", encoding="utf-8")


def fail(message: str) -> None:
    print(f"harness error: {message}", file=sys.stderr)
    raise SystemExit(2)


def arm(name: str) -> Arm:
    policy_of(name)
    return Arm(name=name, path=RUNS / name)


def contract() -> dict:
    return json.loads(CONTRACT.read_text(encoding="utf-8"))


def ailc_binary() -> Path:
    candidates = [
        REPO / "target" / "release" / "ailc",
        REPO / "target" / "debug" / "ailc",
    ]
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    fail("ailc binary not found; run: cargo +1.87.0 build --release -p ail-compiler --bin ailc")
    raise AssertionError("unreachable")


def emit(state: dict, text: str) -> None:
    """Print to the arm and charge the bytes to that arm's token budget."""
    state["tokens_out"] += tokens(text)
    state["chars_out"] += len(text)
    sys.stdout.write(text)
    if not text.endswith("\n"):
        sys.stdout.write("\n")


def log(state: dict, entry: dict) -> None:
    entry["at"] = round(time.time(), 3)
    state["commands"].append(entry)


# --------------------------------------------------------------------------
# specification check (harness text matching, identical for both arms)
# --------------------------------------------------------------------------


def specification_violations(files: dict[str, str]) -> list[dict]:
    spec = contract()
    violations: list[dict] = []
    for required in spec["required_files"]:
        if required not in files:
            violations.append(
                {"id": f"file.{required}", "requirement": f"file {required} exists"}
            )
    blob = normalize("\n".join(files[name] for name in sorted(files)))
    for rule in spec["required_declarations"]:
        if not re.search(rule["pattern"], blob):
            violations.append({"id": rule["id"], "requirement": rule["requirement"]})
    return violations


# --------------------------------------------------------------------------
# gate
# --------------------------------------------------------------------------


def parse_diagnostics(output: str) -> list[dict]:
    """Parse ailc output into stable keys the ledger can compare over time.

    Spans move whenever a file is edited, so a diagnostic's identity is its
    code plus its detail or fact payload, not its offsets.
    """
    parsed: list[dict] = []
    for line in output.splitlines():
        line = line.strip()
        if not line:
            continue
        match = ARCH_DIAGNOSTIC.match(line)
        if match:
            parsed.append(
                {
                    "code": match.group("code"),
                    "class": "ARCH",
                    "scope": match.group("scope"),
                    "rule": match.group("rule"),
                    "facts": match.group("facts").strip(),
                    "key": f"{match.group('code')}|{match.group('rule')}|{match.group('scope')}",
                }
            )
            continue
        match = SOURCE_DIAGNOSTIC.match(line)
        if match:
            code = match.group("code")
            details = " ".join(sorted(match.group("details").split()))
            parsed.append(
                {
                    "code": code,
                    "class": code.split(".")[1],
                    "path": match.group("path"),
                    "details": details,
                    "key": f"{code}|{details}",
                }
            )
            continue
        match = ARCH_INCOMPLETE.match(line)
        if match:
            reason = match.group("reason").strip()
            parsed.append(
                {
                    "code": match.group("code"),
                    "class": "ARCH",
                    "scope": "workspace",
                    "rule": "M23-ANALYSIS-COMPLETE",
                    "facts": reason,
                    "key": f"{match.group('code')}|{reason}",
                }
            )
            continue
        if line.endswith("has parse diagnostics"):
            path = line.split(" ", 1)[0]
            parsed.append(
                {
                    "code": "AIL.PARSE.DIAGNOSTICS",
                    "class": "PARSE",
                    "path": path,
                    "details": "",
                    "key": f"AIL.PARSE.DIAGNOSTICS|{path}",
                }
            )
            continue
        if line in TRAILER_LINES or line.startswith(("revision_id=", "source_set_digest=")):
            continue
        match = BARE_DIAGNOSTIC.match(line)
        if match:
            rest = match.group("rest").strip()
            parsed.append(
                {
                    "code": match.group("code"),
                    "class": match.group("code").split(".")[1],
                    "details": rest,
                    "key": f"{match.group('code')}|{rest}",
                }
            )
            continue
        # Nothing recognized. Record it so the ledger never silently loses a
        # failure the compiler reported.
        parsed.append(
            {
                "code": "HARNESS.UNPARSED_OUTPUT",
                "class": "UNPARSED",
                "details": line,
                "key": f"HARNESS.UNPARSED_OUTPUT|{line}",
            }
        )
    return parsed


def run_gate(state: dict, command: str) -> dict:
    """Materialize the workspace, run one real `ailc` command, tear it down."""
    files = state["files"]
    workspace = Path(tempfile.mkdtemp(prefix=f"ail-gate-{state['arm']}-"))
    try:
        for name, text in files.items():
            (workspace / name).write_text(text, encoding="utf-8")
        completed = subprocess.run(
            [str(ailc_binary()), command, str(workspace)],
            capture_output=True,
            text=True,
            check=False,
        )
        store_written = (workspace / ".ail").exists()
        published_sources = {}
        if store_written:
            source_dir = workspace / ".ail" / "revisions" / "published" / "sources"
            if source_dir.is_dir():
                published_sources = {
                    item.name: item.read_text(encoding="utf-8")
                    for item in sorted(source_dir.iterdir())
                }
    finally:
        shutil.rmtree(workspace, ignore_errors=True)

    output = (completed.stdout + completed.stderr).strip()
    return {
        "command": command,
        "exit_code": completed.returncode,
        "output": output,
        "diagnostics": parse_diagnostics(output),
        "store_written": store_written,
        "published_sources": published_sources,
    }


def gate(arm_name: str, command: str) -> None:
    active = arm(arm_name)
    state = active.load()
    if state.get("passed_at_attempt") is not None:
        emit(state, f"PASS (already passed at attempt {state['passed_at_attempt']})")
        active.save(state)
        return
    if len(state["attempts"]) >= ATTEMPT_LIMIT:
        emit(state, f"attempt limit {ATTEMPT_LIMIT} reached; this run did not converge")
        active.save(state)
        return

    violations = specification_violations(state["files"])
    result = run_gate(state, command)
    index = len(state["attempts"]) + 1
    # The invoked command's own outcome. Both arms are told this bit, because
    # both arms must be able to tell whether the command they ran succeeded.
    # Only the compiler's diagnostics are withheld from the control arm.
    command_passed = result["exit_code"] == 0
    accepted = (
        command == "publish" and command_passed and result["store_written"] and not violations
    )

    attempt = {
        "index": index,
        "command": command,
        "workspace_digest": workspace_digest(state["files"]),
        "exit_code": result["exit_code"],
        "command_passed": command_passed,
        "compiler_output": result["output"],
        "diagnostics": result["diagnostics"],
        "diagnostic_keys": sorted({item["key"] for item in result["diagnostics"]}),
        "specification_violations": [item["id"] for item in violations],
        "store_written": result["store_written"],
        "accepted": accepted,
    }
    if command == "check" and result["store_written"]:
        attempt["invariant_breach"] = "check wrote a revision store"
    if command == "publish" and result["exit_code"] != 0 and result["store_written"]:
        attempt["invariant_breach"] = "failed publish wrote a revision store"
    state["attempts"].append(attempt)

    if accepted:
        state["passed_at_attempt"] = index
        state["published_sources"] = result["published_sources"]

    remaining = ATTEMPT_LIMIT - len(state["attempts"])
    lines = [f"attempt {index} of at most {ATTEMPT_LIMIT} ({remaining} left)"]
    if command == "check":
        lines.append(f"ailc check: {'PASS' if command_passed else 'FAIL'}")
        lines.append("check is read-only and never satisfies the gate; publish does")
    else:
        lines.append("PASS" if accepted else "FAIL")
    if state.get("policy", policy_of(state["arm"])) == "ail" and result["output"]:
        lines.append(result["output"])
    if violations:
        lines.append("task specification not satisfied:")
        lines.extend(f"  - {item['id']}: {item['requirement']}" for item in violations)
    if accepted:
        lines.append("gate satisfied: ailc publish wrote a revision and the specification holds")
    emit(state, "\n".join(lines))
    log(state, {"command": command, "attempt": index, "accepted": accepted})
    active.save(state)


def workspace_digest(files: dict[str, str]) -> str:
    blob = "\n".join(f"{name}:{digest(text)}" for name, text in sorted(files.items()))
    return digest(blob)


# --------------------------------------------------------------------------
# commands
# --------------------------------------------------------------------------


def brief_text(arm_name: str) -> str:
    spec = contract()
    requirements = "\n".join(
        f"  - {rule['id']}: {rule['requirement']}" for rule in spec["required_declarations"]
    )
    if policy_of(arm_name) == "ail":
        feedback = (
            "FEEDBACK POLICY (arm ail): `check` and `publish` return the AIL compiler's\n"
            "own output verbatim: diagnostic codes, spans, `expected.*` / `actual.*`\n"
            "details, and architecture rule findings with the numbers the checker\n"
            "measured. Treat that output as the only source of truth about what is\n"
            "wrong. Do not guess at causes the compiler did not report."
        )
    else:
        feedback = (
            "FEEDBACK POLICY (arm control): `check` and `publish` return only `PASS` or\n"
            "`FAIL`. You will not be told which file, which rule, which type, or even\n"
            "which stage failed. Decide what to change from the source and the\n"
            "reference material alone."
        )
    return f"""TASK
A five-module AIL workspace is broken. Make `ailc publish` succeed on it while
the task specification below still holds. The workspace and its project policy
already exist; you repair AIL source.

RULES
1. The harness is your only access to the workspace. Use `files`, `read`,
   `write`, `check`, `publish`. Do not look for the workspace on disk, do not
   run `ailc` yourself, and do not read the harness state file. There is no
   on-disk copy of the workspace between gate calls.
2. `architecture.json` is project policy and is immutable. The harness rejects
   any write to it. Weakening policy is not a solution.
3. The capability environment is empty and stays empty. There is no capability
   configuration file to add.
4. You may `read` these reference paths, which are identical for both arms:
{chr(10).join('     ' + path for path in REFERENCE_READS)}
5. At most {ATTEMPT_LIMIT} gate calls (`check` plus `publish`) in total.

TASK SPECIFICATION (checked by the harness, not by the compiler)
{requirements}
Immutable files: {', '.join(spec['immutable_files'])}

{feedback}

SUCCESS
`harness.py publish --arm {arm_name}` reports PASS.
"""


def command_start(args: argparse.Namespace) -> None:
    active = arm(args.arm)
    if active.state_path.exists() and not args.force:
        fail(f"arm {args.arm} already started; pass --force to reset it")
    files = {
        item.name: item.read_text(encoding="utf-8")
        for item in sorted(FIXTURE.iterdir())
        if item.is_file()
    }
    state = {
        "arm": args.arm,
        "policy": policy_of(args.arm),
        "started_at": round(time.time(), 3),
        "fixture_digest": workspace_digest(files),
        "attempt_limit": ATTEMPT_LIMIT,
        "files": files,
        "attempts": [],
        "commands": [],
        "edits": [],
        "tokens_out": 0,
        "chars_out": 0,
        "tokens_in": 0,
        "chars_in": 0,
        "passed_at_attempt": None,
        "published_sources": {},
    }
    active.save(state)
    emit(state, f"arm {args.arm} started from the broken fixture ({len(files)} files)")
    active.save(state)


def command_brief(args: argparse.Namespace) -> None:
    active = arm(args.arm)
    state = active.load()
    emit(state, brief_text(args.arm))
    log(state, {"command": "brief"})
    active.save(state)


def command_files(args: argparse.Namespace) -> None:
    active = arm(args.arm)
    state = active.load()
    listing = "\n".join(
        f"{name}  {len(text)} chars" for name, text in sorted(state["files"].items())
    )
    emit(state, listing)
    log(state, {"command": "files"})
    active.save(state)


def command_read(args: argparse.Namespace) -> None:
    active = arm(args.arm)
    state = active.load()
    target = args.path
    if target in state["files"]:
        text = state["files"][target]
    elif target in REFERENCE_READS:
        text = (REPO / target).read_text(encoding="utf-8")
    else:
        fail(
            f"{target} is not a workspace file or an allowed reference path; "
            "run `files` for the workspace and see the brief for reference paths"
        )
        raise AssertionError("unreachable")
    emit(state, text)
    log(state, {"command": "read", "path": target, "tokens": tokens(text)})
    active.save(state)


def command_write(args: argparse.Namespace) -> None:
    active = arm(args.arm)
    state = active.load()
    spec = contract()
    if args.path in spec["immutable_files"]:
        fail(f"{args.path} is immutable project policy; the harness rejects this write")
    if not re.fullmatch(r"[a-z_][a-z0-9_]*\.ail", args.path):
        fail(f"{args.path} is not a writable workspace source name (expected <module>.ail)")
    text = sys.stdin.read()
    if not text.strip():
        fail("refusing to write empty source; pass the full file on stdin")
    if not text.endswith("\n"):
        text += "\n"
    previous = state["files"].get(args.path)
    state["files"][args.path] = text
    state["tokens_in"] += tokens(text)
    state["chars_in"] += len(text)
    state["edits"].append(
        {
            "path": args.path,
            "created": previous is None,
            "tokens": tokens(text),
            "chars": len(text),
            "digest": digest(text),
        }
    )
    emit(state, f"wrote {args.path} ({len(text)} chars)")
    log(state, {"command": "write", "path": args.path, "tokens": tokens(text)})
    active.save(state)


def command_status(args: argparse.Namespace) -> None:
    active = arm(args.arm)
    state = active.load()
    used = len(state["attempts"])
    lines = [
        f"arm {state['arm']}",
        f"gate calls used {used} of {ATTEMPT_LIMIT}",
        f"passed at attempt {state['passed_at_attempt']}"
        if state["passed_at_attempt"]
        else "not passed yet",
    ]
    emit(state, "\n".join(lines))
    log(state, {"command": "status"})
    active.save(state)


def command_check(args: argparse.Namespace) -> None:
    gate(args.arm, "check")


def command_publish(args: argparse.Namespace) -> None:
    gate(args.arm, "publish")


# --------------------------------------------------------------------------
# measures
# --------------------------------------------------------------------------

GOD_METHOD_RULES = {
    "M23-POL-DISPATCH-NO-GROWTH",
    "M23-POL-NEW-UNIT",
    "M23-POL-TRANSPORT-CAPABILITY",
    "M23-POL-TRANSPORT-STATE",
}


def measure(state: dict) -> dict:
    attempts = state["attempts"]
    seen: set[str] = set()
    fixed: set[str] = set()
    new_breakage_events: list[dict] = []
    rebreak_events: list[dict] = []
    previous: set[str] = set()
    god_method_attempts: list[int] = []
    dispatch_cfc: list[int] = []

    for attempt in attempts:
        current = set(attempt["diagnostic_keys"])
        introduced = sorted(current - previous)
        if attempt["index"] > 1 and introduced:
            new_breakage_events.append({"attempt": attempt["index"], "keys": introduced})
        returned = sorted(key for key in introduced if key in fixed)
        if returned:
            rebreak_events.append({"attempt": attempt["index"], "keys": returned})
        fixed |= previous - current
        seen |= current
        previous = current

        for item in attempt["diagnostics"]:
            if item.get("rule") in GOD_METHOD_RULES:
                god_method_attempts.append(attempt["index"])
            for fact in item.get("facts", "").split():
                if fact.startswith("candidate_cfc="):
                    dispatch_cfc.append(int(fact.split("=", 1)[1]))

    reads = [item for item in state["commands"] if item["command"] == "read"]
    return {
        "arm": state["arm"],
        "policy": state.get("policy", policy_of(state["arm"])),
        "converged": state["passed_at_attempt"] is not None,
        "passed_at_attempt": state["passed_at_attempt"],
        # The win condition: failed gate calls before the passing publish.
        "retries_to_pass": (
            state["passed_at_attempt"] - 1 if state["passed_at_attempt"] else None
        ),
        "gate_calls": len(attempts),
        "gate_calls_check": sum(1 for item in attempts if item["command"] == "check"),
        "gate_calls_publish": sum(1 for item in attempts if item["command"] == "publish"),
        "first_clean_check": next(
            (
                item["index"]
                for item in attempts
                if item["command"] == "check" and item["exit_code"] == 0
            ),
            None,
        ),
        "tokens_harness_to_agent": state["tokens_out"],
        "tokens_agent_to_harness": state["tokens_in"],
        "tokens_total": state["tokens_out"] + state["tokens_in"],
        "chars_harness_to_agent": state["chars_out"],
        "chars_agent_to_harness": state["chars_in"],
        "reads": len(reads),
        "reference_reads": sum(1 for item in reads if item["path"] in REFERENCE_READS),
        "edits": len(state["edits"]),
        "distinct_diagnostics_seen": len(seen),
        "diagnostics_seen": sorted(seen),
        "fix_cycles": len(new_breakage_events),
        "new_breakage_events": new_breakage_events,
        "rebreaks": len(rebreak_events),
        "rebreak_events": rebreak_events,
        "god_method_rejections": len(set(god_method_attempts)),
        "god_method_attempts": sorted(set(god_method_attempts)),
        "max_dispatch_control_flow_complexity_seen": max(dispatch_cfc, default=None),
        "specification_violation_attempts": sum(
            1 for item in attempts if item["specification_violations"]
        ),
        "invariant_breaches": [
            {"attempt": item["index"], "breach": item["invariant_breach"]}
            for item in attempts
            if "invariant_breach" in item
        ],
        "check_never_published": all(
            not item["store_written"] for item in attempts if item["command"] == "check"
        ),
        "failed_publish_never_published": all(
            not item["store_written"]
            for item in attempts
            if item["command"] == "publish" and item["exit_code"] != 0
        ),
    }


def command_report(args: argparse.Namespace) -> None:
    if not args.operator:
        fail(
            "report is an operator command: it shows both arms, including the "
            "diagnostics withheld from the control arm. Pass --operator."
        )
    results = {}
    for state_path in sorted(RUNS.glob("*/state.json")):
        name = state_path.parent.name
        if ARM_NAME.match(name):
            results[name] = measure(json.loads(state_path.read_text(encoding="utf-8")))
    if not results:
        fail("no arm has run yet")
    order = sorted(results, key=lambda name: (policy_of(name) != "ail", name))
    results = {name: results[name] for name in order}

    out_dir = Path(args.out) if args.out else HERE / "report"
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "measures.json").write_text(
        json.dumps(results, indent=2) + "\n", encoding="utf-8"
    )

    # Ordered by what decides the outcome. Retries to a passing publish is the
    # win condition. Rebreak is whether the change was safe. Tokens are extra
    # and never decide anything on their own.
    rows = [
        ("1 RETRIES to passing publish", "retries_to_pass"),
        ("  converged", "converged"),
        ("  gate calls to pass", "passed_at_attempt"),
        ("2 REBREAKS (fixed, then broken again)", "rebreaks"),
        ("  fix cycles (new breakage after an edit)", "fix_cycles"),
        ("  attempts failing the task specification", "specification_violation_attempts"),
        ("3 TOKENS total (protocol, extra)", "tokens_total"),
        ("  tokens harness to agent", "tokens_harness_to_agent"),
        ("  tokens agent to harness", "tokens_agent_to_harness"),
        ("- source edits", "edits"),
        ("- reads", "reads"),
        ("- distinct diagnostics reached", "distinct_diagnostics_seen"),
        ("- god-method rejections", "god_method_rejections"),
        ("- worst dispatch control-flow complexity", "max_dispatch_control_flow_complexity_seen"),
        ("- first check that passed", "first_clean_check"),
    ]
    width = max(len(label) for label, _ in rows) + 2
    lines = ["per trial", ""]
    lines.append("measure".ljust(width) + "".join(name.rjust(12) for name in results))
    for label, key in rows:
        cells = "".join(str(results[name][key]).rjust(12) for name in results)
        lines.append(label.ljust(width) + cells)

    policies = {
        policy: [name for name in results if policy_of(name) == policy]
        for policy in POLICIES
    }
    policies = {policy: names for policy, names in policies.items() if names}
    if any(len(names) > 1 for names in policies.values()):
        lines += ["", f"median of {', '.join(f'{k}: {len(v)} trials' for k, v in policies.items())}", ""]
        lines.append("measure".ljust(width) + "".join(policy.rjust(12) for policy in policies))
        for label, key in rows:
            cells = ""
            for names in policies.values():
                values = [results[name][key] for name in names]
                numeric = [value for value in values if isinstance(value, (int, float))]
                cell = str(median(numeric)) if len(numeric) == len(values) else "-"
                cells += cell.rjust(12)
            lines.append(label.ljust(width) + cells)

    lines += ["", "win condition: retries to a passing publish", ""]
    spread = {}
    for policy, names in policies.items():
        values = sorted(
            results[name]["retries_to_pass"]
            for name in names
            if results[name]["retries_to_pass"] is not None
        )
        stalled = sum(1 for name in names if not results[name]["converged"])
        spread[policy] = values
        lines.append(
            f"  {policy:<8} n={len(names)}  retries {values}  "
            f"median {median(values) if values else '-'}  "
            f"worst {max(values) if values else '-'}  did not converge {stalled}"
        )
    if len(spread) == 2 and all(spread.values()):
        (first, low), (second, high) = spread.items()
        worse = sum(1 for a in low for b in high if b > a)
        better = sum(1 for a in low for b in high if b < a)
        tied = sum(1 for a in low for b in high if b == a)
        lines += [
            "",
            f"  paired comparisons ({len(low) * len(high)} of them, every {first} trial "
            f"against every {second} trial):",
            f"    {second} needed more retries in {worse}, fewer in {better}, equal in {tied}",
        ]
        if max(low) < min(high):
            verdict = f"{first} wins the win condition outright: every {first} trial beat every {second} trial."
        elif max(high) < min(low):
            verdict = f"{second} wins the win condition outright: every {second} trial beat every {first} trial."
        elif median(low) == median(high):
            verdict = "no win on the win condition: the medians are equal and the distributions overlap."
        else:
            better = first if median(low) < median(high) else second
            worse = second if better == first else first
            verdict = (
                f"{better} wins the median but not outright: the distributions overlap, so at "
                f"least one {worse} trial beat at least one {better} trial. Not a clean win."
            )
        lines += ["", f"  {verdict}"]

    table = "\n".join(lines)
    (out_dir / "measures.txt").write_text(table + "\n", encoding="utf-8")
    print(table)


# --------------------------------------------------------------------------
# self-test
# --------------------------------------------------------------------------


def strip_comments_target(source: str) -> str:
    """Comment out every line of one module body, keeping its module header.

    Used only by the self-test, to prove a commented-out requirement does not
    satisfy the specification.
    """
    lines = source.splitlines()
    return "\n".join(
        line if line.startswith("module ") else f"// {line}" for line in lines
    ) + "\n"


def command_self_test(args: argparse.Namespace) -> None:
    """Prove the fixture is solvable and the gate cannot be gamed.

    Runs no agent. Applies a reference repair directly, so it is labelled a
    self-test and never counted as an arm result.
    """
    reference = Path(args.reference)
    if not reference.is_dir():
        fail(f"{reference} is not a directory of reference sources")
    checks: list[tuple[str, bool, str]] = []

    broken = {
        item.name: item.read_text(encoding="utf-8")
        for item in sorted(FIXTURE.iterdir())
        if item.is_file()
    }
    state = {"arm": "self-test", "files": dict(broken)}

    result = run_gate(state, "check")
    checks.append(
        (
            "broken fixture fails check",
            result["exit_code"] != 0 and bool(result["diagnostics"]),
            result["output"].splitlines()[0] if result["output"] else "",
        )
    )
    checks.append(
        ("failing check writes no revision store", not result["store_written"], "")
    )

    result = run_gate(state, "publish")
    checks.append(("broken fixture fails publish", result["exit_code"] != 0, ""))
    checks.append(
        ("failing publish writes no revision store", not result["store_written"], "")
    )

    violations = specification_violations(broken)
    checks.append(
        (
            "broken fixture violates the task specification",
            bool(violations),
            ",".join(item["id"] for item in violations),
        )
    )

    weakened = dict(broken)
    policy = json.loads(weakened["architecture.json"])
    policy["policy"]["allowed_group_dependencies"]["transport"] = ["contract", "domain"]
    weakened["architecture.json"] = json.dumps(policy, indent=2) + "\n"
    weakened_state = {"arm": "self-test", "files": weakened}
    weakened_result = run_gate(weakened_state, "publish")
    checks.append(
        (
            "weakening policy is not accepted by the harness write rule",
            "architecture.json" in contract()["immutable_files"],
            "harness.py write rejects architecture.json",
        )
    )
    checks.append(
        (
            "weakened policy alone still does not publish the broken fixture",
            weakened_result["exit_code"] != 0,
            "",
        )
    )

    repaired = dict(broken)
    for item in sorted(reference.iterdir()):
        if item.is_file():
            repaired[item.name] = item.read_text(encoding="utf-8")
    repaired["architecture.json"] = broken["architecture.json"]
    repaired_state = {"arm": "self-test", "files": repaired}
    result = run_gate(repaired_state, "publish")
    checks.append(("reference repair publishes", result["exit_code"] == 0, result["output"]))
    checks.append(("reference repair writes a revision store", result["store_written"], ""))
    checks.append(
        (
            "reference repair satisfies the task specification",
            not specification_violations(repaired),
            "",
        )
    )
    checks.append(
        (
            "reference repair keeps policy byte-identical",
            repaired["architecture.json"] == broken["architecture.json"],
            "",
        )
    )

    commented = dict(repaired)
    commented["domain.ail"] = strip_comments_target(commented["domain.ail"])
    checks.append(
        (
            "a requirement commented out does not satisfy the specification",
            bool(specification_violations(commented)),
            ",".join(item["id"] for item in specification_violations(commented)),
        )
    )

    failed = 0
    for label, ok, detail in checks:
        status = "ok  " if ok else "FAIL"
        suffix = f"  [{detail}]" if detail else ""
        print(f"{status} {label}{suffix}")
        failed += 0 if ok else 1
    print(f"\n{len(checks) - failed}/{len(checks)} self-test checks passed")
    raise SystemExit(0 if failed == 0 else 1)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    def with_arm(name: str, handler, **kwargs):
        item = sub.add_parser(name, **kwargs)
        item.add_argument("--arm", required=True)
        item.set_defaults(handler=handler)
        return item

    start = with_arm("start", command_start, help="load the broken fixture for one arm")
    start.add_argument("--force", action="store_true", help="reset an existing arm")
    with_arm("brief", command_brief, help="print the task brief for one arm")
    with_arm("files", command_files, help="list workspace files")
    with_arm("read", command_read, help="read a workspace file or reference path").add_argument(
        "path"
    )
    with_arm("write", command_write, help="replace one workspace file from stdin").add_argument(
        "path"
    )
    with_arm("check", command_check, help="run ailc check through the gate")
    with_arm("publish", command_publish, help="run ailc publish through the gate")
    with_arm("status", command_status, help="show remaining gate calls")

    report = sub.add_parser("report", help="write the two-arm measure table")
    report.add_argument("--operator", action="store_true")
    report.add_argument("--out")
    report.set_defaults(handler=command_report)

    self_test = sub.add_parser("self-test", help="prove the fixture is solvable and gated")
    self_test.add_argument("--reference", required=True)
    self_test.set_defaults(handler=command_self_test)

    args = parser.parse_args()
    args.handler(args)


if __name__ == "__main__":
    main()
