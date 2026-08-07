# Compiler implementation decision

Rust is the compiler implementation language. ADR 0004 made that decision and
superseded the planned Rust/TypeScript spike comparison.

The technical reasons remain:

- strong representation invariants for syntax and semantic data;
- memory safety without a garbage collector;
- algebraic data types and pattern matching;
- mature parsing, testing, profiling, and native build tools; and
- a direct path to future native-code libraries if a backend is selected.

The costs are longer compile times, ownership friction in graph-heavy code, and
a steeper contributor learning curve. Manage those costs in the production Rust
compiler; do not maintain a second semantic implementation.

See [ADR 0004](decisions/0004-rust-compiler-stack.md) for the decision and
[compiler/README.md](../compiler/README.md) for the running implementation.
