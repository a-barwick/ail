# AIL documentation

Use the smallest document that owns the fact you need:

1. [Current status](STATUS.md) — shipped boundary, hard limits, next move, and
   required checks.
2. [Specifications](../specs/README.md) — numbered language and compiler rules.
3. [Compiler guide](../compiler/README.md) — Rust APIs and focused commands.
4. [Workloads](workloads/) — the application behavior motivating a contract.
5. [Product](product.md) and [design](design.md) — short orientation only.

Numbered rules and conformance fixtures define behavior. If prose differs from a
fixture, the fixture wins. Historical decisions, proposals, milestones, and
external material live in [history/](history/) and are not routine reading.

Run `python3 tools/check_docs.py` after documentation changes.
