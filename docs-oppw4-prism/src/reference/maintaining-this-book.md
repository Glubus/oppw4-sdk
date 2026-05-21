# Maintaining This Book

This book is the public-facing documentation for SDK developers and Lua modders.

Build it with:

```bash
mdbook build docs-oppw4-prism
```

Guidelines:

- keep pages short and task-focused;
- prefer examples over internal history;
- link concepts through `SUMMARY.md`;
- update the book when architecture boundaries change;
- keep reverse-engineering evidence in dedicated notes when it is uncertain;
- do not commit `docs-oppw4-prism/book/`.

When adding a new public API, add:

- the purpose of the API;
- one small example;
- capability requirements if it is a plugin API;
- data requirements if it depends on `oppw4-data`;
- failure modes when they are useful to modders.
