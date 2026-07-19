# Answer-free benchmark task starts

Status: **Frozen by M7**

This directory locks the agent-visible starting workspace for each combination
of UC-001 or UC-003 and Rust, Go, Python, or TypeScript.

The package lock is independent of the later agent-trial runner. It records
every output file, its protection role and SHA-256 digest, a tree digest for
each configuration, and the expected starting-state checks.

Build one workspace with:

```bash
python3 benchmarks/tools/task_starts.py build \
  --language rust \
  --task UC-001 \
  --output /tmp/ail-rust-uc001
```

Verify all eight deterministic packages and their visibility boundaries with:

```bash
python3 benchmarks/tools/task_starts.py check
```

Run the exact-tool starting-state checks as well with:

```bash
python3 benchmarks/tools/task_starts.py check --run-starting-state
```

The generated workspace contains only the selected task text, one language,
ordinary visible tests, normal tool configuration, and public fixtures needed
by those tests. The package lock, source recipes, reference baselines, private
fixtures, hidden seed locations, and M7 freeze metadata remain outside the
workspace.
