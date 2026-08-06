# Post-M27 cleanup ledger

Status: **Complete review record**

This ledger records only findings confirmed against post-M27 commit
`06441144f397bd7af30afd76b0674373a1e9c434`. The independent read-only audit is
in [the review thread](https://ampcode.com/threads/T-019fd07d-8a3e-70ec-a03b-d65fd1fd8d35).
This cleanup preserves accepted behavior and starts no successor milestone.

| Finding and evidence | Smallest correction | Files | Risk and focused check | Status |
| --- | --- | --- | --- | --- |
| The documentation index's current-position section stopped after M24 even though its milestone summary mentioned M25–M27. | Record the bounded implementation and non-official pilot consistently. | `docs/README.md` | Low; `python3 tools/check_docs.py` | Resolved |
| The specification index omits the accepted M24 contract and its checker. The AIL verification manifest digest-locks this index as an accepted compiler input. | Retain the index until a separately authorized contract-lock revision can update it together with its digest. | `specs/README.md`, `benchmarks/baselines/ail/verification-manifest.json` | Medium; AIL public harness verification rejected the documentation edit | Deferred — frozen artifact contract |
| Accepted architectural-health requirements still described M24–M26 contract and implementation work as future. | Preserve the requirements while recording which bounded milestones delivered them and which benchmark calibration remains open. | `docs/requirements/architectural-health.md` | Medium; architecture acceptance, contract, and docs checks | Resolved |
| UC-007 called delivered conformance evidence future work, then initially attributed its human-review criterion too broadly to M25–M26. | Attribute machine-verifiable evidence to M25–M26 and only one bounded non-official actionability observation to M27. | `docs/use-cases/UC-007-architectural-regression-control.md` | Low; architecture and docs checks | Resolved |
| Design direction said the whole architecture feature remained non-normative after the bounded M24 rules were accepted. | Distinguish the accepted bounded contract from the broader proposed manifest. | `docs/design-direction.md` | Medium; architecture contract and docs checks | Resolved |
| Current entry points described M26 as in progress or architectural enforcement as wholly proposed. | Bring the root and compiler summaries through M27 without defining later work. | `README.md`, `compiler/README.md` | Low; docs check | Resolved |
| Governing agent guidance still called accepted UC-007 proposed and implied an active Rust milestone. | Record M23–M27 completion, no active successor, bounded acceptance, and the broader manifest's proposed status. | `AGENTS.md` | Medium; docs and milestone-alignment checks | Resolved |
| The Rust baseline's direct commands did not pin the 1.88.0 toolchain used by its locked runner descriptor. | Prefix those commands with `rustup run 1.88.0`. | `benchmarks/baselines/rust/README.md` | Low; docs check and public harness verification | Resolved |
| The M24 checker still printed `M24 remains Active`. | Report that the accepted contract remains fixed instead of claiming an active milestone. | `specs/tools/architecture_contract.py` | Low; architecture contract check | Resolved |
| `serde_json` is declared as both a normal and dev dependency even though normal dependencies are available to tests. The AIL verification manifest digest-locks `Cargo.toml`, so removal is not an isolated dependency cleanup. | Retain the declaration unless a separately authorized contract change revises the verification manifest. | `compiler/ail-compiler/Cargo.toml`, `benchmarks/baselines/ail/verification-manifest.json` | Medium; AIL public harness verification rejected removal | Deferred — frozen artifact contract |
| The custom SHA-256 implementation had one source fixture vector but no empty, short, multi-block, or 55/56/63/64-byte padding-transition vectors. | Add independent known-answer vectors without changing the implementation. | `compiler/ail-compiler/tests/m16_protocol.rs` | Low; focused M16 test | Resolved |
| The new pilot verifier had semantic mutation tests but no direct harness subcommand dispatch or CLI error-mapping test. | Test successful dispatch and `ArchitecturePilotError` mapping through `harness.main`. | `benchmarks/tests/test_harness.py` | Low; focused harness and pilot tests | Resolved |
| `benchmarks/tests/test_fixtures.py` imported `json` but never referenced it. | Delete the import; keep the used `mock` import. | `benchmarks/tests/test_fixtures.py` | Low; focused fixture tests | Resolved |
| `architecture.rs` contains snapshot derivation, policy evaluation, canonical encoding, and rendering in one 2,634-line module. The responsibilities are distinct, but their private types and ordering rules are coupled. | Decide module boundaries separately before moving code; do not perform cosmetic helper splitting during cleanup. | `compiler/ail-compiler/src/architecture.rs` | High; would require all M25/M26 fixtures and compiler checks | Deferred — design decision required |
| The public free `validate_architecture_change` function only forwards to the workspace method, but integration tests import it and external callers may do the same. | Establish public API compatibility policy before deleting or deprecating it. | `compiler/ail-compiler/src/architecture.rs`, `compiler/ail-compiler/src/lib.rs` | Medium; public API and M26 tests | Deferred — API decision required |
| `docs/STATUS.md` combines the current handoff with detailed prior milestone results. The roadmap has shorter delivery records, but not every operational detail. | Define an authoritative historical replacement before pruning the status record. | `docs/STATUS.md`, `docs/roadmap.md` | Medium; risks erasing useful implementation evidence | Deferred — no complete replacement |

No production code, dependency package, fixture, specification, ADR, benchmark
evidence package, or whole document met the deletion standard. In particular,
the M8 records, M23 acceptance package, M24 fixtures, and M27 pilot remain
intentional evidence.
