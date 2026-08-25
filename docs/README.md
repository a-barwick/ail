# AIL documentation

1. [Language](language.md) — what the compiler implements.
2. [Status](STATUS.md) — current limits and required checks.
3. [Specifications](../specs/README.md) — exact rules and fixtures.
4. [Compiler guide](../compiler/README.md) — Rust APIs and focused commands.
5. [Workloads](workloads/) — the application behavior behind a contract.
6. [Proof of concept](poc.md) — two measured tests of whether compiler output
   helps an agent repair code.

If prose disagrees with a fixture, the fixture wins.

Run `python3 tools/check_docs.py` after documentation changes.
