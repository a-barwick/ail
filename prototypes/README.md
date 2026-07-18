# Prototypes

This directory is for disposable, bounded implementation spikes used to resolve
architecture decisions.

Each prototype must:

- live in its own subdirectory;
- keep dependencies and build artifacts local;
- implement an explicitly shared fixture and protocol;
- document setup, measurements, and incomplete behavior;
- avoid being imported by another prototype or treated as production code; and
- be deletable without changing normative documentation.

A prototype becomes production only through an accepted architecture decision
and an intentional migration.
