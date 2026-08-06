# M27 non-official architecture-feedback pilot

This directory retains one complete, explicitly non-official run of the locked
M23 `CancelJob` repair task. It shows that one Amp operator could use M26's
compact rejection and structured contributors to produce an accepted repair.
It is not comparative, statistical, official, or evidence of broader project
success.

The evidence package contains:

- `prompt.txt`: the exact operator prompt;
- `operator-report.json`: the operator's final canonical report;
- `repaired-candidate.json`: the candidate frozen after final comparison;
- `pilot.json`: environment, restrictions, inspected findings, validation log,
  comparison, and limitations; and
- `pilot.lock.json`: the SHA-256 lock for the manifest, which in turn locks the
  other artifacts.

The initial compact output established that all six behavior cases passed but
publication was denied because `transport:dispatch` grew and transport acquired
jobs-store authority, jobs state, and a forbidden persistence dependency. The
structured drill-down identified the exact dispatch, capability, and state
contributors and made the ownership error actionable.

The repair routes the transport adapter to `domain:handle:job.cancel` and moves
the jobs-store capability plus jobs state reads and writes to that domain
handler. Transport remains registration and adaptation only. The retained
candidate passes all six behavior cases and the architecture policy. It differs
from the locked valid candidate only by an empty `changed_units` field and one
redundant domain-to-contract `type-use` edge; the candidate was not edited after
that comparison. As required by the pilot prompt, the report's final compact
fields quote the locked valid response; the Rust replay checks the retained
non-identical candidate independently.

Verify the retained evidence and replay the candidate with:

```bash
python3 -m unittest benchmarks.tests.test_architecture_pilot
python3 benchmarks/tools/harness.py verify-architecture-pilot
cargo +1.87.0 test --workspace --test m26_architecture_delta
```

To replay the operator procedure itself, start from the locked M23 fixtures in
a clean checkout and follow `prompt.txt`. Preserve the recorded version, mode,
permissions, restrictions, and tool availability when interpreting any result.
One operator repairing one seeded candidate supports only a later go, revise,
or stop decision; no broader claim follows.
