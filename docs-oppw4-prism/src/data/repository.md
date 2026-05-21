# Repository And Workflow

`oppw4-data` is a submodule inside the SDK repo.

Use it when you want to:

- add missing character metadata;
- improve costume definitions;
- add asset paths;
- document body parts;
- add evidence for IDs or paths.

Typical workflow:

```text
1. edit source JSON under oppw4-data/characters/<character>/
2. run the data generator/validator
3. inspect generated files under oppw4-data/generated/
4. commit data changes in oppw4-data
5. update the SDK submodule pointer when needed
```

Data changes should stay data-only. SDK behavior changes belong in
`oppw4-sdk`.
