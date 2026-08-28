# AIL specifications

These files and their fixtures are the current compiler contract. The Rust
implementation must match them. Compiler behavior may not fill a gap in a
contract.

| Contract | Rules | Checker |
| --- | --- | --- |
| Core language | [core.md](core.md), [protocol.md](protocol.md) | `python3 specs/tools/core_contract.py check` |
| Interpreter | [runtime.md](runtime.md) | included in the core checker and runtime fixtures |
| Schema evolution | [evolution.md](evolution.md) | included with evolution fixtures |
| Architecture policy | [architecture.md](architecture.md) | `python3 specs/tools/architecture_contract.py check` |
| Bounded lists and `map` | [bounded-lists.md](bounded-lists.md) | `python3 specs/tools/bounded_list_contract.py check` |
| Outbound requests | [outbound request contract](outbound-requests.md) | `python3 specs/tools/outbound_request_contract.py check` |
| Bounded outbound map | [bounded-outbound-workflows.md](bounded-outbound-workflows.md) | `python3 specs/tools/bounded_outbound_workflow_contract.py check` |
| HTTP lookup host | [pinned-http-batch-lookup.md](pinned-http-batch-lookup.md), [private-catalog-dogfood.md](private-catalog-dogfood.md) | Rust tests under `service-host/` |

JSON files next to these specs encode fixtures and protocol shapes. They are
not a selected compiler transport. The outbound request contract publishes its
[protocol shapes](outbound-request-protocol.json).

[language.md](../docs/language.md) is the readable overview.
[STATUS.md](../docs/STATUS.md) lists current limits. Each contract is only as
broad as its numbered rules: lists are not a collection library, outbound map
is not general concurrency, and the HTTP host is not a service framework.
