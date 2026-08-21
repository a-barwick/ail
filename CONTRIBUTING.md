# Contributing

Read [the language](docs/language.md) and [current limits](docs/STATUS.md)
first.

To add behavior, write a failing executable check, implement the smallest
semantics that change the result, and run the checks in `docs/STATUS.md`. Do
not treat sequential `map` or the pinned lookup host as a path to general
collections, networking, routing, or concurrency.

Examples illustrate behavior. Numbered specs and fixtures define it.

Record a decision in `docs/decisions/` when a change alters public semantics or
makes an expensive implementation choice.

```bash
python3 tools/check_docs.py
```
