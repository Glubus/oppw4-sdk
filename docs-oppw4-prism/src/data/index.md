# Data Bank Overview

`oppw4-data` is the collaborative data repository used by the SDK at runtime.
It exists so character metadata, costumes, assets, and references can improve
without recompiling SDK code.

The SDK reads generated views and source character folders from:

```text
oppw4-data/
  characters/
  generated/
  schemas/
```

The goal is not to finish every character inside the SDK repo. The goal is to
make the data easy to improve incrementally through community contributions.
