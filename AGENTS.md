# AIL engineering instructions

## Report status like an engineer, not a committee

Write every status update in plain English for a VP of Engineering. Lead with what works, what changed, what is blocked, and what happens next. Use concrete compiler behavior, test results, and delivery dates when known. Do not use corporate jargon, governance language, ceremonial milestone language, vague reassurance, or planning activity as a substitute for shipped software.

## Know whether you are the orchestrator or a worker

Every thread must know its role. A thread coordinating the user's work and
delegating tasks is the **orchestrator**. A thread launched or assigned work by
another thread is a **worker**. A worker remains a worker for its lifetime and
must retain the orchestrator thread's full URL from its launch message or thread
metadata.

Only the orchestrator reports project-wide status. When the user asks for a
status check, the orchestrator reports the combined state of active work,
completed work, blockers, and the next delivery.

A worker reports scoped results and blockers to the orchestrator, but it must not
present itself as the project status source. If anyone asks a worker for status,
the worker replies in plain English: "I'm a worker thread responsible for
<assigned task>. Please ask the orchestrator for project status:" followed by a
clickable link labeled `orchestrator thread` whose target is the orchestrator's
full URL. The link must point to the actual orchestrator thread; never invent or
guess it. If the orchestrator URL is missing, the worker says that directly and
asks the requester to return to the thread that assigned the work.

## Commit production-ready work directly to main

Commit completed, verified changes directly to `main`. Do not create or use
feature branches. Keep each commit production-ready and reviewable. After every
commit, immediately push `main` to its remote with `git push origin main`. Work
is not delivered until the push succeeds. If the push fails, report the exact
blocker and leave the verified local commit intact.

AIL is an executable language for software agents. Agents are the primary
authors and operators. Humans must be able to audit canonical source, compiler
facts, diagnostics, authority, behavior, and every proposed change.

## Read before changing the system

Start with `docs/language.md` and the owning numbered specification plus its
tests or fixtures. Use `docs/STATUS.md` for current limits. Read a workload
record when it explains the application behavior behind that specification.

## Current system

`docs/language.md` describes what the compiler implements. `docs/STATUS.md`
lists what it still cannot do. Do not copy either into other files. The
compiler has bounded sequential `map`, not general iteration; do not describe
proposed features as shipped.

## Engineering rules

- Start from the failing behavior, missing capability, or measured constraint.
- Ship the smallest coherent implementation that changes the result.
- Add an executable check for every delivered behavior and rejection path.
- Do not infer general collections, networking, routing, or concurrency from
  sequential `map` or the pinned lookup host. Do not restart deferred
  measurement campaigns by default.
- Keep canonical source as the durable artifact. Expose inferred facts through
  the compiler instead of requiring hidden agent reasoning.
- Keep public types, errors, effects, capabilities, and stable identities
  explicit. Local facts may be inferred only when the compiler can report them.
- Preserve deterministic ordering and revision binding in compiler results.
- Reject stale handles and incomplete analysis. Never report a partial result as
  complete.
- Validate multi-file changes atomically. A failed change publishes nothing.
- Treat capability access as authority. Do not introduce ambient access to time,
  randomness, storage, network, filesystem, environment, or telemetry.
- Measure architecture with primitive facts at function and aggregate scopes.
  Splitting a function into helpers does not fix concentrated authority, state,
  dependencies, or review context.
- Keep project policy separate from language semantics. A project may reject a
  metric delta; AIL does not declare one universal complexity threshold.
- Use familiar syntax when it reduces errors. Do not add syntax novelty for
  aesthetics.
- Do not optimize for source-token count at the cost of more context retrieval,
  repair work, or human review risk.

## Delivery

Work directly on local `main`. Do not disturb unrelated changes. Commit only
complete changes that build and pass their focused checks.

For compiler work, run the narrow test first, then the full Rust checks before
declaring completion:

```bash
cargo +1.87.0 fmt --all --check
cargo +1.87.0 test --workspace
cargo +1.87.0 clippy --workspace --all-targets -- -D warnings
python3 tools/check_docs.py
```

Run the relevant specification or benchmark checker when the change touches its
contract. Do not edit locked fixtures or manifests merely to make a test pass.

Record a decision in `docs/decisions/` when it changes public semantics or makes
an expensive implementation choice. Keep the record technical: decision,
constraints, consequences, rejected alternatives, and validation.

## Writing

Write direct engineering English.

- Lead with what works, what fails, and what to do next.
- Name the compiler behavior: “the compiler rejects recursive calls.”
- Prefer test results and concrete constraints to reassurance.
- Remove committee language, process narration, and status adjectives that do
  not change technical meaning.
- Preserve terms such as deterministic, revision-bound, conformance, and
  normative only when they identify an exact property.
- Never turn a prototype, example, or planned feature into a language rule.

A proposal must identify the user-visible behavior, the agent work or risk it
removes, why the compiler or language must own it, the exact deterministic test,
and how a human audits the result.
