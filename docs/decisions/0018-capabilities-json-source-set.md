# ADR 0018: `capabilities.json` at the source-set root

- Status: Accepted
- Date: 2026-08-28

## Context

`ailc check` and `ailc publish` built an `EvolutionWorkspace` with an empty
`CapabilityEnvironment`. Capability-using source sets failed as
`AIL.CAPABILITY.UNKNOWN_INTERFACE` until a Rust caller supplied interfaces.
ADR 0013 and ADR 0014 rejected inventing a project manifest or capability
syntax to fill that gap.

Austin chose one file at the same layer as `architecture.json`:
`capabilities.json` in the directory passed to `ailc check` and
`ailc publish`. The driver already knows how to look at exactly one
source-set-root JSON file and how to record a digest on publish.

## Decision

`ailc check` and `ailc publish` look only at
`<source-set-root>/capabilities.json`.

- The file is absent: the capability environment stays empty. Capability use
  still fails because the interfaces were not supplied. The driver does not
  invent defaults or search elsewhere.
- The file is present: load it into the existing `CapabilityEnvironment`
  shape, the same map of interface name to operations used by the locked
  capability fixtures.
- Reject malformed JSON, a value that is not that shape, missing required
  operation fields, duplicate capability names, and source that uses an
  interface the file did not grant.
- The loaded path and the environment digest are compiler facts. Publish
  records `capability_environment_digest` the same way it already records
  `architecture_settings_digest` when `architecture.json` is present.

The driver does not invent AIL capability syntax, a project manifest, a second
filename, a repository-root search, or a `.ail/` lookup. `ail-run` still
supplies an empty environment.

## Consequences

`compiler/examples/capability-declared` passes `ailc check` because its root
`capabilities.json` declares `JobsStore`. `compiler/examples/batch-lookup`
still fails `AIL.CAPABILITY.UNKNOWN_INTERFACE` because it has no
`capabilities.json`; a file under `.ail/` or in a parent directory does not
count.

A source set published under a loaded environment records a non-empty
`capability_environment_digest`. `ail-run` refuses that revision with
`AIL.RUN.CAPABILITY_ENVIRONMENT_DIGEST` until a later change teaches the
runner to supply those interfaces.

## Alternatives considered

- Keep the empty CLI environment until a project manifest exists: rejected.
  Capability-using source sets stay uncheckable from `ailc` after the path
  was chosen.
- Infer signatures from call sites so check looks green: rejected. That hides
  undeclared authority.
- Search the repository root, `.ail/`, or a second filename: rejected. One
  path, same layer as `architecture.json`.
- Invent capability syntax in `.ail` files: rejected. Interfaces stay host
  facts loaded into `CapabilityEnvironment`.

## Validation

`cargo +1.87.0 test -p ail-compiler --test ailc_check --test ailc_publish
--test ailc_findings` proves a declared source set passes check and publish,
an absent file does not load a decoy elsewhere, malformed JSON and duplicate
capability names are rejected as load errors, undeclared authority fails as
`AIL.CAPABILITY.UNKNOWN_INTERFACE`, and the loaded path and digest appear as
compiler facts.
