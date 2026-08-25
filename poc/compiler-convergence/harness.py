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

The arms are staggered. Every `control` arm of a broken workspace finishes
before the first `ail` arm of that workspace starts, so a blind trial cannot
read compiler findings out of a compiler arm's ledger: no such ledger exists
while it works. The harness enforces both halves of that order and records it,
and `self-test` proves it by driving the real command line.

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
from dataclasses import dataclass, replace
from functools import lru_cache
from pathlib import Path
from statistics import median

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
POLICIES = ("ail", "control")
ARM_NAME = re.compile(r"^(?P<policy>ail|control)(?:-t(?P<trial>\d+))?$")
ATTEMPT_LIMIT = 40

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


@dataclass(frozen=True)
class Fixture:
    """One broken workspace, its task specification, and its run directories.

    A fixture owns everything that varies between experiments: the broken
    source, the specification the harness matches, the reference material both
    arms may read, and where its runs and report are written. The protocol, the
    gate, and the measures are shared, so two fixtures are comparable as
    experiments even when their faults are unrelated.
    """

    name: str
    directory: str
    # Where this fixture's run directories live. Defaults to the harness
    # directory; `--runs-root` moves one batch elsewhere, which the self-test
    # uses so its own trial never lands in the committed runs.
    runs_root: Path | None = None

    @property
    def root(self) -> Path:
        return HERE / self.directory

    @property
    def base(self) -> Path:
        return self.runs_root or HERE

    @property
    def broken(self) -> Path:
        return self.root / "broken"

    @property
    def contract_path(self) -> Path:
        return self.root / "contract.json"

    @property
    def reference(self) -> Path:
        return self.root / "reference-solution"

    @property
    def negative_controls(self) -> Path:
        return self.root / "negative-controls"

    @property
    def runs(self) -> Path:
        return (
            self.base / f"runs-{self.name}"
            if self.directory != "fixture"
            else self.base / "runs"
        )

    @property
    def report(self) -> Path:
        return (
            self.base / f"report-{self.name}"
            if self.directory != "fixture"
            else self.base / "report"
        )


FIXTURES = {
    "cancel-dispatch": Fixture(name="cancel-dispatch", directory="fixture"),
    "label-batch": Fixture(name="label-batch", directory="fixture-label-batch"),
    # Same broken workspace and same brief as `label-batch`, separate runs. Used
    # for a second condition in which the operator adds one instruction, worded
    # identically for both arms, so its trials never pool with the first
    # condition's.
    "label-batch-frugal": Fixture(name="label-batch-frugal", directory="fixture-label-batch"),
}
DEFAULT_FIXTURE = "cancel-dispatch"


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


def normalize(text: str, qualifiers: tuple[str, ...]) -> str:
    """Collapse whitespace, drop comments, drop the fixture's type qualifiers.

    The task specification is matched against this form so a requirement does
    not dictate which module a declaration lives in or how it is indented.
    """
    stripped = strip_comments(text)
    for qualifier in qualifiers:
        stripped = stripped.replace(qualifier, "")
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
    fixture: Fixture

    @property
    def policy(self) -> str:
        return policy_of(self.name)

    @property
    def state_path(self) -> Path:
        return self.path / "state.json"

    def load(self) -> dict:
        if not self.state_path.is_file():
            fail(
                f"arm {self.name} has not started; run: harness.py start "
                f"--fixture {self.fixture.name} --arm {self.name}"
            )
        return json.loads(self.state_path.read_text(encoding="utf-8"))

    def save(self, state: dict) -> None:
        self.path.mkdir(parents=True, exist_ok=True)
        self.state_path.write_text(json.dumps(state, indent=2) + "\n", encoding="utf-8")


def fail(message: str) -> None:
    print(f"harness error: {message}", file=sys.stderr)
    raise SystemExit(2)


def fixture_named(name: str, runs_root: str | None = None) -> Fixture:
    if name not in FIXTURES:
        fail(f"unknown fixture {name}; expected one of {', '.join(sorted(FIXTURES))}")
    fixture = FIXTURES[name]
    if runs_root:
        fixture = replace(fixture, runs_root=Path(runs_root).resolve())
    return fixture


def arm(name: str, fixture: Fixture, action: str) -> Arm:
    """Resolve one arm, refusing any command the arm order forbids.

    Every arm command goes through here, so the order is enforced once: a blind
    arm may act only while no compiler-arm ledger exists, and a compiler arm may
    start only after every blind arm of this workspace has finished.
    """
    if policy_of(name) == "control":
        require_no_compiler_ledger(fixture, name, action)
    elif action == "start":
        require_blind_arms_finished(fixture, name)
    return Arm(name=name, path=fixture.runs / name, fixture=fixture)


def contract(fixture: Fixture) -> dict:
    return json.loads(fixture.contract_path.read_text(encoding="utf-8"))


# --------------------------------------------------------------------------
# arm order: every blind arm finishes before the first compiler arm starts
# --------------------------------------------------------------------------

STAGGER_PROTOCOL = "control-then-ail"


def broken_files(fixture: Fixture) -> dict[str, str]:
    return {
        item.name: item.read_text(encoding="utf-8")
        for item in sorted(fixture.broken.iterdir())
        if item.is_file()
    }


@lru_cache(maxsize=None)
def fixture_digest(fixture: Fixture) -> str:
    return workspace_digest(broken_files(fixture))


@dataclass(frozen=True)
class Ledger:
    """One arm's state file, named by the run directory that holds it.

    Two run directories of one broken workspace can hold arms of the same name,
    so `label` is what a message or an audit check names.
    """

    arm: str
    label: str
    path: Path
    state: dict

    @property
    def policy(self) -> str:
        return policy_of(self.arm)


def ledgers(fixture: Fixture) -> list[Ledger]:
    """Every arm ledger under this runs root started from the same broken workspace.

    Relevance is the broken workspace's digest, not the run directory's name.
    Two fixtures can share one broken workspace, as `label-batch` and
    `label-batch-frugal` do, and an archived run directory holds the same
    compiler findings as a live one.
    """
    wanted = fixture_digest(fixture)
    found: list[Ledger] = []
    for path in sorted(fixture.base.glob("runs*/*/state.json")):
        if not ARM_NAME.match(path.parent.name):
            continue
        try:
            state = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        if state.get("fixture_digest") == wanted:
            found.append(
                Ledger(
                    arm=path.parent.name,
                    label=str(path.parent.relative_to(fixture.base)),
                    path=path,
                    state=state,
                )
            )
    return found


def compiler_ledgers(fixture: Fixture) -> list[Ledger]:
    """Every `ail` ledger on disk for this broken workspace.

    These files carry the compiler output the blind arm must not see, including
    the findings the harness withheld from it.
    """
    return [item for item in ledgers(fixture) if item.policy == "ail"]


def finished_at(state: dict) -> float | None:
    """When this arm stopped being able to change its ledger, or `None` in flight.

    Runs recorded before arm-order enforcement carry no `finished_at`, so it is
    derived: an arm that passed or exhausted its gate calls finished at its last
    recorded command.
    """
    if state.get("finished_at") is not None:
        return state["finished_at"]
    attempts = state.get("attempts") or []
    limit = state.get("attempt_limit", ATTEMPT_LIMIT)
    if state.get("passed_at_attempt") is None and len(attempts) < limit:
        return None
    commands = state.get("commands") or []
    return commands[-1]["at"] if commands else state.get("started_at")


def blind_arms(fixture: Fixture) -> list[Ledger]:
    return [item for item in ledgers(fixture) if item.policy == "control"]


def close_if_done(state: dict) -> None:
    """Stamp the moment this arm can no longer change its ledger."""
    if state.get("finished_at") is not None:
        return
    if state["passed_at_attempt"] is not None or len(state["attempts"]) >= ATTEMPT_LIMIT:
        state["finished_at"] = round(time.time(), 3)


def require_no_compiler_ledger(fixture: Fixture, arm_name: str, action: str) -> None:
    present = compiler_ledgers(fixture)
    if not present:
        return
    names = ", ".join(item.label for item in present)
    fail(
        f"arm order violation: refusing to {action} blind arm {arm_name} because a "
        f"compiler-arm ledger for this broken workspace already exists: {names}. A "
        "blind trial runs before any compiler output for its workspace exists on "
        "disk. Run the blind arms first, or run this batch under a fresh --runs-root."
    )


def require_blind_arms_finished(fixture: Fixture, arm_name: str) -> dict[str, float]:
    arms = blind_arms(fixture)
    unfinished = sorted(item.label for item in arms if finished_at(item.state) is None)
    if unfinished:
        fail(
            f"arm order violation: refusing to start compiler arm {arm_name} while "
            f"blind arms are still running: {', '.join(unfinished)}. Every control "
            "arm finishes first, so no compiler-arm ledger exists on disk while a "
            "blind arm works. Close an abandoned arm with: harness.py close --arm "
            "<name> --operator --reason <why>"
        )
    if not arms:
        fail(
            f"arm order violation: refusing to start compiler arm {arm_name} before "
            "any blind arm of this workspace has run. The blind arms run first."
        )
    return {item.label: finished_at(item.state) for item in arms}


def qualifiers(spec: dict) -> tuple[str, ...]:
    return tuple(spec.get("strip_qualifiers", []))


def reference_reads(spec: dict) -> tuple[str, ...]:
    """Reference material both arms may read, identical for the two arms."""
    return tuple(spec["reference_reads"])


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


def specification_violations(files: dict[str, str], fixture: Fixture) -> list[dict]:
    spec = contract(fixture)
    violations: list[dict] = []
    for required in spec["required_files"]:
        if required not in files:
            violations.append(
                {"id": f"file.{required}", "requirement": f"file {required} exists"}
            )
    blob = normalize("\n".join(files[name] for name in sorted(files)), qualifiers(spec))
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


def json_findings(workspace: Path) -> list[dict] | None:
    """Read the ledger's diagnostics from `ailc check --json`, if supported.

    `check` is read-only, so this probe is safe before either command and gives
    the ledger a stable machine-readable shape instead of a line format that
    changes when diagnostic rendering changes. Returns `None` when the compiler
    has no `--json` view, and the caller falls back to parsing text.
    """
    completed = subprocess.run(
        [str(ailc_binary()), "check", "--json", str(workspace)],
        capture_output=True,
        text=True,
        check=False,
    )
    try:
        document = json.loads(completed.stdout)
    except json.JSONDecodeError:
        return None
    if not isinstance(document, dict) or "findings" not in document:
        return None
    parsed = []
    for finding in document["findings"]:
        facts = dict(finding.get("facts") or {})
        for key, value in (finding.get("expected") or {}).items():
            facts[f"expected.{key}"] = value
        for key, value in (finding.get("actual") or {}).items():
            facts[f"actual.{key}"] = value
        # Keep the in-ledger shape the text parser produced, so every measure
        # reads one format regardless of which compiler emitted it.
        flat = " ".join(f"{key}={value}" for key, value in sorted(facts.items()))
        code = finding["code"]
        parsed.append(
            {
                "code": code,
                "class": finding.get("category") or code.split(".")[1],
                "path": (finding.get("location") or {}).get("path", "<source-set>"),
                "rule": facts.get("rule"),
                "scope": facts.get("scope"),
                "facts": flat,
                "requirement": finding.get("requirement"),
                "key": f"{code}|{flat}",
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
        findings = json_findings(workspace)
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
        "diagnostics": findings if findings is not None else parse_diagnostics(output),
        "diagnostic_source": "json" if findings is not None else "text",
        "store_written": store_written,
        "published_sources": published_sources,
    }


def gate(arm_name: str, command: str, fixture: Fixture) -> None:
    active = arm(arm_name, fixture, command)
    state = active.load()
    if state.get("passed_at_attempt") is not None:
        emit(state, f"PASS (already passed at attempt {state['passed_at_attempt']})")
        close_if_done(state)
        active.save(state)
        return
    if len(state["attempts"]) >= ATTEMPT_LIMIT:
        emit(state, f"attempt limit {ATTEMPT_LIMIT} reached; this run did not converge")
        close_if_done(state)
        active.save(state)
        return

    violations = specification_violations(state["files"], fixture)
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
        "diagnostic_source": result["diagnostic_source"],
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
    close_if_done(state)
    active.save(state)


def workspace_digest(files: dict[str, str]) -> str:
    blob = "\n".join(f"{name}:{digest(text)}" for name, text in sorted(files.items()))
    return digest(blob)


# --------------------------------------------------------------------------
# commands
# --------------------------------------------------------------------------


def fixture_flag(fixture: Fixture) -> str:
    """The `--fixture` argument an arm must pass, empty for the default."""
    return "" if fixture.name == DEFAULT_FIXTURE else f" --fixture {fixture.name}"


def brief_text(arm_name: str, fixture: Fixture) -> str:
    spec = contract(fixture)
    requirements = "\n".join(
        f"  - {rule['id']}: {rule['requirement']}" for rule in spec["required_declarations"]
    )
    reads = reference_reads(spec)
    rules = [
        "The harness is your only access to the workspace. Use `files`, `read`,\n"
        "   `write`, `check`, `publish`. Do not look for the workspace on disk, do not\n"
        "   run `ailc` yourself, and do not read the harness state file. There is no\n"
        "   on-disk copy of the workspace between gate calls.",
        *spec["brief"]["rules"],
        "You may `read` these reference paths, which are identical for both arms:\n"
        + chr(10).join("     " + path for path in reads),
        f"At most {ATTEMPT_LIMIT} gate calls (`check` plus `publish`) in total.",
    ]
    numbered = "\n".join(f"{index}. {rule}" for index, rule in enumerate(rules, start=1))
    if policy_of(arm_name) == "ail":
        feedback = spec["brief"]["feedback_ail"]
    else:
        feedback = (
            "FEEDBACK POLICY (arm control): `check` and `publish` return only `PASS` or\n"
            "`FAIL`. You will not be told which file, which rule, which type, or even\n"
            "which stage failed. Decide what to change from the source and the\n"
            "reference material alone."
        )
    return f"""TASK
{spec["brief"]["task"]}

RULES
{numbered}

TASK SPECIFICATION (checked by the harness, not by the compiler)
{requirements}
Immutable files: {', '.join(spec['immutable_files'])}

{feedback}

SUCCESS
`harness.py publish{fixture_flag(fixture)} --arm {arm_name}` reports PASS.
"""


def command_start(args: argparse.Namespace) -> None:
    fixture = fixture_named(args.fixture, args.runs_root)
    active = arm(args.arm, fixture, "start")
    if active.state_path.exists() and not args.force:
        fail(f"arm {args.arm} already started; pass --force to reset it")
    files = broken_files(fixture)
    policy = policy_of(args.arm)
    # What the arm order looked like at the instant this arm started, recorded
    # so `audit` can check the claim later without trusting this run.
    stagger = {
        "protocol": STAGGER_PROTOCOL,
        "compiler_ledgers_present": [item.label for item in compiler_ledgers(fixture)],
        "blind_arms_finished": (
            require_blind_arms_finished(fixture, args.arm) if policy == "ail" else {}
        ),
    }
    state = {
        # A canary, not a control. Nothing prevents an arm from reading this
        # file; the brief forbids it. An arm that reads it and reports so makes
        # its own trial auditable, and one that does not is a trust failure the
        # run notes cannot detect.
        "_notice": (
            "operator canary: this is the harness state file. The brief forbids an arm "
            "from reading it. An arm that reads it must say so in its final report."
        ),
        "arm": args.arm,
        "policy": policy,
        "fixture": fixture.name,
        "started_at": round(time.time(), 3),
        "finished_at": None,
        "stagger": stagger,
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
    fixture = fixture_named(args.fixture, args.runs_root)
    active = arm(args.arm, fixture, "brief")
    state = active.load()
    emit(state, brief_text(args.arm, fixture))
    log(state, {"command": "brief"})
    active.save(state)


def command_files(args: argparse.Namespace) -> None:
    active = arm(args.arm, fixture_named(args.fixture, args.runs_root), "list files for")
    state = active.load()
    listing = "\n".join(
        f"{name}  {len(text)} chars" for name, text in sorted(state["files"].items())
    )
    emit(state, listing)
    log(state, {"command": "files"})
    active.save(state)


def command_read(args: argparse.Namespace) -> None:
    fixture = fixture_named(args.fixture, args.runs_root)
    active = arm(args.arm, fixture, "read for")
    state = active.load()
    target = args.path
    if target in state["files"]:
        text = state["files"][target]
    elif target in reference_reads(contract(fixture)):
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
    fixture = fixture_named(args.fixture, args.runs_root)
    active = arm(args.arm, fixture, "write for")
    state = active.load()
    spec = contract(fixture)
    rejection = write_rejection(args.path, spec)
    if rejection:
        fail(rejection)
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


def write_rejection(path: str, spec: dict) -> str | None:
    """Why the harness refuses to write `path`, or `None` when it accepts it.

    The immutable check runs first, so an immutable file with a source-shaped
    name is still rejected as immutable.
    """
    if path in spec["immutable_files"]:
        return f"{path} is {spec['immutable_reason']}; the harness rejects this write"
    if not re.fullmatch(r"[a-z_][a-z0-9_]*\.ail", path):
        return f"{path} is not a writable workspace source name (expected <module>.ail)"
    return None


def command_status(args: argparse.Namespace) -> None:
    active = arm(args.arm, fixture_named(args.fixture, args.runs_root), "report status for")
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
    gate(args.arm, "check", fixture_named(args.fixture, args.runs_root))


def command_publish(args: argparse.Namespace) -> None:
    gate(args.arm, "publish", fixture_named(args.fixture, args.runs_root))


def command_close(args: argparse.Namespace) -> None:
    """Close an abandoned arm so the next policy can start.

    An arm that neither passed nor spent its gate calls is in flight forever,
    and a blind arm in flight blocks every compiler arm. Closing one is an
    operator act, it records why, and it never changes a measure.
    """
    if not args.operator or not args.reason.strip():
        fail("close ends a trial the arm did not finish. Pass --operator and --reason.")
    fixture = fixture_named(args.fixture, args.runs_root)
    active = arm(args.arm, fixture, "close")
    state = active.load()
    if state.get("finished_at") is not None:
        fail(f"arm {args.arm} already finished at {state['finished_at']}")
    state["finished_at"] = round(time.time(), 3)
    state["closed"] = {"reason": args.reason, "at": state["finished_at"]}
    active.save(state)
    print(
        f"closed arm {args.arm} after {len(state['attempts'])} gate calls, "
        f"passed_at_attempt={state['passed_at_attempt']}: {args.reason}"
    )


# --------------------------------------------------------------------------
# measures
# --------------------------------------------------------------------------

GOD_METHOD_RULES = {
    "M23-POL-DISPATCH-NO-GROWTH",
    "M23-POL-NEW-UNIT",
    "M23-POL-TRANSPORT-CAPABILITY",
    "M23-POL-TRANSPORT-STATE",
}


def diagnostic_class(item: dict) -> str:
    """Uppercase failure class of one ledger diagnostic.

    `ailc check --json` reports a category (`type`, `capability`, `architecture`)
    and the text parser derives the class from the code, so both ledger formats
    reduce to one comparable label.
    """
    label = item.get("class") or item.get("category") or ""
    if not label:
        label = item.get("code", "").split(".")[1] if "." in item.get("code", "") else "UNKNOWN"
    return label.upper()


def measure(state: dict, fixture: Fixture) -> dict:
    attempts = state["attempts"]
    seen: set[str] = set()
    fixed: set[str] = set()
    new_breakage_events: list[dict] = []
    rebreak_events: list[dict] = []
    previous: set[str] = set()
    god_method_attempts: list[int] = []
    dispatch_cfc: list[int] = []
    keys_by_class: dict[str, set[str]] = {}

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
            keys_by_class.setdefault(diagnostic_class(item), set()).add(item["key"])
            if item.get("rule") in GOD_METHOD_RULES:
                god_method_attempts.append(attempt["index"])
            for fact in (item.get("facts") or "").split():
                name, _, value = fact.partition("=")
                if name.removeprefix("facts.") == "candidate_cfc" and value.isdigit():
                    dispatch_cfc.append(int(value))

    reads = [item for item in state["commands"] if item["command"] == "read"]
    allowed_reads = reference_reads(contract(fixture))
    return {
        "arm": state["arm"],
        "policy": state.get("policy", policy_of(state["arm"])),
        "fixture": state.get("fixture", fixture.name),
        "converged": state["passed_at_attempt"] is not None,
        "passed_at_attempt": state["passed_at_attempt"],
        # The win condition: gate calls spent before the publish that passed,
        # whether each one failed or passed.
        "retries_to_pass": (
            state["passed_at_attempt"] - 1 if state["passed_at_attempt"] else None
        ),
        # Of those gate calls, the ones the compiler or the specification denied.
        # `retries_to_pass` counts every earlier gate call, including a passing
        # `check`, so the two numbers differ for an arm that checks before it
        # publishes.
        "failed_gate_calls_before_pass": (
            sum(
                1
                for item in attempts
                if item["index"] < state["passed_at_attempt"] and not item["command_passed"]
            )
            if state["passed_at_attempt"]
            else None
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
        "ledger_from": ",".join(
            sorted({item.get("diagnostic_source", "text") for item in attempts})
        )
        or "-",
        "reads": len(reads),
        "reference_reads": sum(1 for item in reads if item["path"] in allowed_reads),
        "edits": len(state["edits"]),
        "distinct_diagnostics_seen": len(seen),
        "diagnostics_seen": sorted(seen),
        "diagnostic_classes": ",".join(sorted(keys_by_class)) or "-",
        "type_diagnostics_seen": len(keys_by_class.get("TYPE", ())),
        "capability_diagnostics_seen": len(keys_by_class.get("CAPABILITY", ())),
        "architecture_diagnostics_seen": len(keys_by_class.get("ARCHITECTURE", ()))
        + len(keys_by_class.get("ARCH", ())),
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
    fixture = fixture_named(args.fixture, args.runs_root)
    results = {}
    for state_path in sorted(fixture.runs.glob("*/state.json")):
        name = state_path.parent.name
        if ARM_NAME.match(name):
            state = json.loads(state_path.read_text(encoding="utf-8"))
            results[name] = measure(state, fixture)
    if not results:
        fail(f"no arm has run yet for fixture {fixture.name}")
    order = sorted(results, key=lambda name: (policy_of(name) != "ail", name))
    results = {name: results[name] for name in order}

    out_dir = Path(args.out) if args.out else fixture.report
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "measures.json").write_text(
        json.dumps(results, indent=2) + "\n", encoding="utf-8"
    )

    # Ordered by what decides the outcome. Retries to a passing publish is the
    # win condition. Rebreak is whether the change was safe. Tokens are extra
    # and never decide anything on their own.
    rows = [
        ("1 RETRIES to passing publish", "retries_to_pass"),
        ("  of those, gate calls that failed", "failed_gate_calls_before_pass"),
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
        ("- distinct type diagnostics reached", "type_diagnostics_seen"),
        ("- distinct capability diagnostics reached", "capability_diagnostics_seen"),
        ("- distinct architecture diagnostics reached", "architecture_diagnostics_seen"),
        ("- god-method rejections", "god_method_rejections"),
        ("- worst dispatch control-flow complexity", "max_dispatch_control_flow_complexity_seen"),
        ("- first check that passed", "first_clean_check"),
        ("- ledger built from", "ledger_from"),
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
        pairs = len(low) * len(high)
        second_worse = sum(1 for a in low for b in high if b > a)
        second_better = sum(1 for a in low for b in high if b < a)
        tied = pairs - second_worse - second_better
        lines += [
            "",
            f"  paired comparisons ({pairs} of them, every {first} trial "
            f"against every {second} trial):",
            f"    {second} needed more retries in {second_worse}, fewer in "
            f"{second_better}, equal in {tied}",
        ]
        # The verdict states only what the paired comparisons support. Fewer
        # retries is better, so a win means the other arm never needed fewer.
        if second_worse == pairs:
            verdict = (
                f"{first} wins the win condition outright: every {first} trial needed "
                f"fewer retries than every {second} trial."
            )
        elif second_better == pairs:
            verdict = (
                f"{second} wins the win condition outright: every {second} trial needed "
                f"fewer retries than every {first} trial."
            )
        elif second_better == 0 and second_worse > 0:
            verdict = (
                f"{first} wins the win condition, with ties: no {second} trial needed "
                f"fewer retries than any {first} trial, and {second_worse} of {pairs} "
                f"comparisons went to {first}."
            )
        elif second_worse == 0 and second_better > 0:
            verdict = (
                f"{second} wins the win condition, with ties: no {first} trial needed "
                f"fewer retries than any {second} trial, and {second_better} of {pairs} "
                f"comparisons went to {second}."
            )
        elif median(low) == median(high):
            verdict = "no win on the win condition: the medians are equal and the distributions overlap."
        else:
            ahead = first if median(low) < median(high) else second
            behind = second if ahead == first else first
            verdict = (
                f"{ahead} wins the median but not outright: the distributions overlap, so at "
                f"least one {behind} trial beat at least one {ahead} trial. Not a clean win."
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


def overlay(base: dict[str, str], directory: Path) -> dict[str, str]:
    """Return `base` with every file in `directory` replacing its namesake."""
    merged = dict(base)
    for item in sorted(directory.iterdir()):
        if item.is_file():
            merged[item.name] = item.read_text(encoding="utf-8")
    return merged


def harness_cli(
    root: Path, fixture: Fixture, *argv: str, stdin: str | None = None
) -> subprocess.CompletedProcess:
    """Run one real harness command against a temporary runs root."""
    return subprocess.run(
        [
            sys.executable,
            str(Path(__file__).resolve()),
            *argv,
            "--fixture",
            fixture.name,
            "--runs-root",
            str(root),
        ],
        capture_output=True,
        text=True,
        input=stdin,
        check=False,
    )


def first_line(text: str) -> str:
    return text.strip().splitlines()[0] if text.strip() else ""


def arm_order_checks(fixture: Fixture, reference: Path, spec: dict) -> list[tuple[str, bool, str]]:
    """Prove a blind trial runs to its end before any compiler ledger exists.

    Drives the real command line in a temporary runs root, so the committed runs
    are untouched and every refusal here is the one an operator would hit. The
    blind trial is a real one: it writes the stored reference repair through the
    harness and publishes, with `PASS` or `FAIL` as its only feedback.
    """
    checks: list[tuple[str, bool, str]] = []
    root = Path(tempfile.mkdtemp(prefix="ail-arm-order-"))

    def ledger_arms() -> list[str]:
        return sorted(path.parent.name for path in root.glob("runs*/*/state.json"))

    try:
        started = harness_cli(root, fixture, "start", "--arm", "control")
        checks.append(
            (
                "blind arm starts when no compiler-arm ledger exists",
                started.returncode == 0,
                first_line(started.stderr),
            )
        )
        early = harness_cli(root, fixture, "start", "--arm", "ail")
        checks.append(
            (
                "compiler arm is refused while a blind arm is still running",
                early.returncode != 0 and "arm order violation" in early.stderr,
                first_line(early.stderr),
            )
        )

        repair = {
            item.name: item.read_text(encoding="utf-8")
            for item in sorted(reference.iterdir())
            if item.is_file() and item.name not in spec["immutable_files"]
        }
        for name, text in repair.items():
            wrote = harness_cli(root, fixture, "write", "--arm", "control", name, stdin=text)
            if wrote.returncode != 0:
                checks.append((f"blind arm writes {name}", False, first_line(wrote.stderr)))
        during = ledger_arms()
        checks.append(
            (
                "no compiler-arm ledger exists on disk while the blind arm works",
                bool(during) and all(policy_of(name) == "control" for name in during),
                ",".join(during),
            )
        )
        published = harness_cli(root, fixture, "publish", "--arm", "control")
        checks.append(
            (
                "blind arm reaches a passing publish with no compiler output on disk",
                published.returncode == 0 and "PASS" in published.stdout,
                first_line(published.stdout),
            )
        )
        checks.append(
            (
                "nothing the blind arm was shown carries a compiler diagnostic",
                "AIL." not in published.stdout,
                "",
            )
        )

        allowed = harness_cli(root, fixture, "start", "--arm", "ail")
        checks.append(
            (
                "compiler arm starts once the blind arm has finished",
                allowed.returncode == 0,
                first_line(allowed.stderr),
            )
        )
        after = ledger_arms()
        checks.append(
            (
                "the compiler-arm ledger appears only after the blind arm finished",
                any(policy_of(name) == "ail" for name in after),
                ",".join(after),
            )
        )
        late = harness_cli(root, fixture, "start", "--arm", "control-t2")
        checks.append(
            (
                "a new blind arm is refused once a compiler-arm ledger exists",
                late.returncode != 0 and "arm order violation" in late.stderr,
                first_line(late.stderr),
            )
        )
        source = sorted(broken_files(fixture))[0]
        blocked = harness_cli(root, fixture, "read", "--arm", "control", source)
        checks.append(
            (
                "the blind arm cannot use the harness once a compiler-arm ledger exists",
                blocked.returncode != 0 and "arm order violation" in blocked.stderr,
                first_line(blocked.stderr),
            )
        )
        audited = harness_cli(root, fixture, "audit")
        # Audit reports the whole run, so its exit code answers more than the
        # arm order. Read its arm-order lines, which is the claim under test.
        order_lines = [
            line
            for line in audited.stdout.splitlines()
            if "compiler-arm ledger existed" in line or "had finished before it started" in line
        ]
        checks.append(
            (
                "audit confirms the order from the ledgers alone",
                len(order_lines) == 2 and all(line.startswith("ok") for line in order_lines),
                "; ".join(line.strip() for line in order_lines) or first_line(audited.stderr),
            )
        )
    finally:
        shutil.rmtree(root, ignore_errors=True)
    return checks


def command_self_test(args: argparse.Namespace) -> None:
    """Prove the fixture is solvable, the gate cannot be gamed, and the arms are staggered.

    Runs no agent. Applies a reference repair directly, so it is labelled a
    self-test and never counted as an arm result.
    """
    fixture = fixture_named(args.fixture, args.runs_root)
    spec = contract(fixture)
    reference = Path(args.reference) if args.reference else fixture.reference
    if not reference.is_dir():
        fail(f"{reference} is not a directory of reference sources")
    checks: list[tuple[str, bool, str]] = []

    broken = broken_files(fixture)
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
        (
            "ledger diagnostics carry a code and a stable key",
            all(item["code"] and item["key"] for item in result["diagnostics"]),
            f"source={result['diagnostic_source']}, "
            f"codes={','.join(item['code'] for item in result['diagnostics'])}",
        )
    )
    classes = sorted({diagnostic_class(item) for item in result["diagnostics"]})
    declared_classes = sorted(item.upper() for item in spec["fault_classes"])
    checks.append(
        (
            f"broken fixture fails only in its declared classes {declared_classes}",
            set(classes) <= set(declared_classes),
            ",".join(classes),
        )
    )
    for excluded in spec["excluded_fault_classes"]:
        label = excluded.upper()
        structural = label != "ARCHITECTURE" or "architecture.json" not in broken
        checks.append(
            (
                f"no {excluded} failure is reachable in this fixture",
                label not in classes and label not in declared_classes and structural,
                "no architecture.json in the workspace"
                if label == "ARCHITECTURE" and structural
                else ",".join(classes),
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

    violations = specification_violations(broken, fixture)
    if spec["broken_satisfies_specification"]:
        checks.append(
            (
                "broken fixture satisfies the task specification, so a violation "
                "never points at a fault",
                not violations,
                ",".join(item["id"] for item in violations),
            )
        )
    else:
        checks.append(
            (
                "broken fixture violates the task specification",
                bool(violations),
                ",".join(item["id"] for item in violations),
            )
        )

    for name in spec["immutable_files"]:
        rejection = write_rejection(name, spec)
        checks.append(
            (
                f"harness.py write rejects the immutable file {name}",
                bool(rejection),
                rejection or "write was accepted",
            )
        )

    if "architecture.json" in broken:
        weakened = dict(broken)
        policy = json.loads(weakened["architecture.json"])
        policy["policy"]["allowed_group_dependencies"]["transport"] = ["contract", "domain"]
        weakened["architecture.json"] = json.dumps(policy, indent=2) + "\n"
        weakened_result = run_gate({"arm": "self-test", "files": weakened}, "publish")
        checks.append(
            (
                "weakened policy alone still does not publish the broken fixture",
                weakened_result["exit_code"] != 0,
                "",
            )
        )

    repaired = overlay(broken, reference)
    for name in spec["immutable_files"]:
        repaired[name] = broken[name]
    result = run_gate({"arm": "self-test", "files": repaired}, "publish")
    checks.append(("reference repair publishes", result["exit_code"] == 0, result["output"]))
    checks.append(("reference repair writes a revision store", result["store_written"], ""))
    checks.append(
        (
            "reference repair satisfies the task specification",
            not specification_violations(repaired, fixture),
            "",
        )
    )
    checks.append(
        (
            "reference repair keeps every immutable file byte-identical",
            all(repaired[name] == broken[name] for name in spec["immutable_files"]),
            ",".join(spec["immutable_files"]),
        )
    )

    for control in spec.get("negative_controls", []):
        directory = fixture.negative_controls / control["id"]
        if not directory.is_dir():
            checks.append((f"negative control {control['id']} exists", False, str(directory)))
            continue
        base = repaired if control["base"] == "reference" else broken
        candidate = overlay(base, directory)
        control_result = run_gate({"arm": "self-test", "files": candidate}, "publish")
        control_violations = specification_violations(candidate, fixture)
        compiler_denied = control_result["exit_code"] != 0
        want_compiler_fail = control["expect"]["compiler"] == "fail"
        want_violation = control["expect"]["specification"] == "violated"
        detail = (
            f"compiler {'denied' if compiler_denied else 'accepted'}, "
            f"specification {','.join(item['id'] for item in control_violations) or 'satisfied'}"
        )
        checks.append(
            (
                f"negative control {control['id']}: {control['requirement']}",
                compiler_denied == want_compiler_fail
                and bool(control_violations) == want_violation
                and not (
                    control_result["exit_code"] == 0
                    and not control_violations
                    and control_result["store_written"]
                ),
                detail,
            )
        )

    commented = dict(repaired)
    target = spec["comment_out_module"]
    commented[target] = strip_comments_target(commented[target])
    commented_violations = specification_violations(commented, fixture)
    checks.append(
        (
            f"commenting out {target} does not satisfy the specification",
            bool(commented_violations),
            ",".join(item["id"] for item in commented_violations),
        )
    )

    checks += arm_order_checks(fixture, reference, spec)

    failed = 0
    for label, ok, detail in checks:
        status = "ok  " if ok else "FAIL"
        suffix = f"  [{detail}]" if detail else ""
        print(f"{status} {label}{suffix}")
        failed += 0 if ok else 1
    print(f"\n{len(checks) - failed}/{len(checks)} self-test checks passed")
    raise SystemExit(0 if failed == 0 else 1)


def audit_arm_order(fixture: Fixture) -> list[tuple[str, bool, str]]:
    """Check from the ledgers alone that no compiler output existed during a blind trial.

    Two facts decide it, and both are recorded by the run rather than asserted
    here: a blind arm saw no compiler-arm ledger when it started, and every
    compiler arm started after the last blind arm had finished. Arms recorded
    before this order was enforced carry neither fact, so they are reported as
    what they are instead of being scored.
    """
    checks: list[tuple[str, bool, str]] = []
    blind = blind_arms(fixture)
    compiler = compiler_ledgers(fixture)
    legacy = sorted(item.label for item in blind + compiler if "stagger" not in item.state)
    if legacy:
        print(
            f"info arm order: {len(legacy)} arm(s) ran before the harness enforced the "
            f"order and carry no record of it: {', '.join(legacy)}"
        )
    for item in blind:
        if "stagger" not in item.state:
            continue
        seen = item.state["stagger"]["compiler_ledgers_present"]
        checks.append(
            (
                f"{item.label}: no compiler-arm ledger existed when this blind arm started",
                not seen,
                ",".join(seen),
            )
        )
    for item in compiler:
        if "stagger" not in item.state:
            continue
        started = item.state["started_at"]
        for other in blind:
            done = finished_at(other.state)
            if done is None:
                detail = "still in flight"
            else:
                detail = f"finished {done} > started {started}" if done > started else ""
            checks.append(
                (
                    f"{item.label}: blind arm {other.label} had finished before it started",
                    done is not None and done <= started,
                    detail,
                )
            )
    return checks


def command_audit(args: argparse.Namespace) -> None:
    """Audit finished runs against claims a reader can check without trusting us.

    Every check here reads the committed run ledger and the committed fixture. It
    answers: did any arm touch an immutable file, did any published workspace
    fail the task specification, did a failure class this fixture excludes ever
    appear, did any gate call break the write invariants, and did the arms run in
    the order the protocol requires.
    """
    fixture = fixture_named(args.fixture, args.runs_root)
    spec = contract(fixture)
    broken = broken_files(fixture)
    checks: list[tuple[str, bool, str]] = audit_arm_order(fixture)
    for state_path in sorted(fixture.runs.glob("*/state.json")):
        name = state_path.parent.name
        if not ARM_NAME.match(name):
            continue
        state = json.loads(state_path.read_text(encoding="utf-8"))
        for immutable in spec["immutable_files"]:
            checks.append(
                (
                    f"{name}: {immutable} byte-identical to the fixture",
                    state["files"].get(immutable) == broken[immutable],
                    "",
                )
            )
        published = state.get("published_sources") or {}
        if state["passed_at_attempt"]:
            violations = specification_violations(published, fixture)
            checks.append(
                (
                    f"{name}: published workspace satisfies the specification",
                    not violations,
                    ",".join(item["id"] for item in violations),
                )
            )
            checks.append(
                (
                    f"{name}: published sources are the arm's last written sources",
                    published == state["files"],
                    "",
                )
            )
        for excluded in spec["excluded_fault_classes"]:
            label = excluded.upper()
            hits = sorted(
                {
                    attempt["index"]
                    for attempt in state["attempts"]
                    for item in attempt["diagnostics"]
                    if diagnostic_class(item) == label
                }
            )
            checks.append(
                (
                    f"{name}: no {excluded} finding in any gate call",
                    not hits,
                    f"attempts {hits}" if hits else "",
                )
            )
        checks.append(
            (
                f"{name}: check never wrote a revision store",
                all(
                    not attempt["store_written"]
                    for attempt in state["attempts"]
                    if attempt["command"] == "check"
                ),
                "",
            )
        )
        checks.append(
            (
                f"{name}: failed publish never wrote a revision store",
                all(
                    not attempt["store_written"]
                    for attempt in state["attempts"]
                    if attempt["command"] == "publish" and attempt["exit_code"] != 0
                ),
                "",
            )
        )
        created = sorted({edit["path"] for edit in state["edits"] if edit["created"]})
        edited = sorted({edit["path"] for edit in state["edits"] if not edit["created"]})
        print(f"info {name}: edited {edited or 'nothing'}, created {created or 'nothing'}")

    if not checks:
        fail(f"no arm has run yet for fixture {fixture.name}")
    failed = 0
    for label, ok, detail in checks:
        status = "ok  " if ok else "FAIL"
        suffix = f"  [{detail}]" if detail else ""
        print(f"{status} {label}{suffix}")
        failed += 0 if ok else 1
    print(f"\n{len(checks) - failed}/{len(checks)} audit checks passed")
    raise SystemExit(0 if failed == 0 else 1)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    def with_fixture(name: str, **kwargs):
        item = sub.add_parser(name, **kwargs)
        item.add_argument(
            "--fixture",
            default=DEFAULT_FIXTURE,
            choices=sorted(FIXTURES),
            help=f"which broken workspace to run (default {DEFAULT_FIXTURE})",
        )
        item.add_argument(
            "--runs-root",
            help=(
                "directory holding this batch's runs-* and report-* directories "
                "(default: the harness directory). The arm order is enforced within "
                "one root; the self-test uses a temporary one."
            ),
        )
        return item

    def with_arm(name: str, handler, **kwargs):
        item = with_fixture(name, **kwargs)
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

    close = with_arm("close", command_close, help="close an arm the agent abandoned")
    close.add_argument("--operator", action="store_true")
    close.add_argument("--reason", default="", help="why this trial ended unfinished")

    report = with_fixture("report", help="write the two-arm measure table")
    report.add_argument("--operator", action="store_true")
    report.add_argument("--out")
    report.set_defaults(handler=command_report)

    audit = with_fixture("audit", help="audit finished runs of one fixture")
    audit.set_defaults(handler=command_audit)

    self_test = with_fixture("self-test", help="prove the fixture is solvable and gated")
    self_test.add_argument(
        "--reference", help="directory of reference sources (default: the fixture's own)"
    )
    self_test.set_defaults(handler=command_self_test)

    args = parser.parse_args()
    args.handler(args)


if __name__ == "__main__":
    main()
