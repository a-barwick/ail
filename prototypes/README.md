# Prototypes

This directory is for disposable, bounded implementation spikes used to resolve
architecture decisions.

Each prototype must:

- live in its own subdirectory;
- keep dependencies and build artifacts local;
- implement an explicitly shared fixture and protocol tied to identified
  requirements and proposed semantic rules;
- document setup, measurements, and incomplete behavior;
- avoid being imported by another prototype or treated as production code; and
- be deletable without changing normative documentation.

A prototype must not invent missing application requirements or language
semantics silently. Incidental behavior is evidence to review, not a decision.

A prototype becomes production only through an accepted architecture decision
and an intentional migration.
