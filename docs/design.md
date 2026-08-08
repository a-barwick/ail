# Design overview

AIL's compiler pipeline is:

```text
canonical source -> parse and format -> resolution and checking
  -> revision-bound semantic graph -> deterministic interpreter
```

Canonical formatting, explicit capability authority, ordered observable effects,
immutable revisions, structured diagnostics, and atomic candidate validation
make compiler facts inspectable and reproducible. Project architecture policy
consumes compiler facts; it does not define language semantics.

The exact rules are in [specs/](../specs/README.md). Current implementation
limits are in [STATUS.md](STATUS.md); do not infer support from this overview or
from historical proposals.
