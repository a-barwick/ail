# AIL engineering instructions

## Report status like an engineer, not a committee

Write every status update in plain English for a VP of Engineering. Lead with what works, what changed, what is blocked, and what happens next. Use concrete compiler behavior, test results, and delivery dates when known. Do not use corporate jargon, governance language, ceremonial milestone language, vague reassurance, or planning activity as a substitute for shipped software.

## Commit production-ready work directly to main

Commit completed, verified changes directly to local `main`. Do not create or
use feature branches. Keep each commit production-ready and reviewable. Do not
push unless the maintainer explicitly asks.

AIL is an executable language for software agents. Agents are the primary
authors and operators. Humans must be able to audit canonical source, compiler
facts, diagnostics, authority, behavior, and every proposed change.

## Read before changing the system

Read these files in order:

1. `docs/README.md`
2. `docs/project-intent.md`
3. `docs/application-vision.md`
4. `docs/use-cases/README.md`
5. `docs/requirements/README.md`
6. `docs/design-direction.md`
7. `docs/architecture-health.md`
8. `docs/spec-review.md`
9. `docs/roadmap.md`
10. `docs/STATUS.md`

Then read the numbered specification, use case, requirement, and decision that
owns the behavior you will change.

## Current system

The Rust compiler implements canonical syntax, static checking, structured
diagnostics, immutable revisions, semantic inspection, validated rename,
schema-impact queries, atomic multi-file validation, deterministic
interpretation, the M24 architecture checks, ordinary calls, and explicit
modules, imports, aliases, and qualified cross-module references.

The compiler rejects recursive calls and import cycles. It has no iteration,
general collections, concurrency, networking, production runtime, native
backend, package registry, or deployment system. Do not write about proposed
features as if they work.

`docs/STATUS.md` states the current executable move. If no work is active, do
not infer a new project from old plans.

## Engineering rules

- Start from the failing behavior, missing capability, or measured constraint.
- Ship the smallest coherent implementation that changes the result.
- Add an executable check for every delivered behavior and rejection path.
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
