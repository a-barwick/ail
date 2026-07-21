# Baseline calibration

M8 measures genuine agent changes and runtime behavior against the inputs frozen
by M7. It does not expose a completed implementation to an agent.

`benchmarks/tools/calibration.py` builds eight deterministic task starts:

- UC-001 receives the language's public V1 contracts and tests with the handler
  and validation implementation removed.
- UC-003 receives the accepted V1 implementation and the ordinary V2 tests,
  but no V2 implementation source.

Each workspace also contains the public fixtures and process contracts. It does
not contain another baseline language, the completed V2 reference source, or
the private fixture package.

Verify the task-start lock with:

```bash
python3 benchmarks/tools/calibration.py task-starts --check
```

The private package remains outside every generated workspace and becomes
available only to the post-run correctness verifier.
