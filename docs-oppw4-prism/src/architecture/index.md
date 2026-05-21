# Architecture

OPPW4 Prism is split into small layers:

```text
dinput8 proxy
  -> loader
    -> SDK core
      -> SDK service plugins
        -> feature plugins
          -> Lua mods and data bank
```

The rule is simple: lower layers provide primitives, upper layers provide game meaning.

The proxy/loader should not know about characters, active characters, Lua APIs, LinkData policies, or RDB rules. SDK core routes services and validates capabilities. Service plugins understand game formats. Lua mods consume stable APIs.

Mermaid note: mdBook does not render Mermaid diagrams by default. Add `mdbook-mermaid` later if the docs need rendered graphs; until then, this book uses plain text diagrams so it builds with stock mdBook.
