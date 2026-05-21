# OPPW4 Prism Modding SDK

OPPW4 Prism is split into a small Windows loader and a larger SDK. The loader
does process bootstrap work. The SDK owns modding concepts: Lua, plugins,
character data, LinkData/RDB services, capabilities, logs, config roots, and
official plugin APIs.

This book is for two audiences:

- Lua modders who want to install mods, write `mod.lua`, use `std.*`, and call
  official plugin helpers.
- SDK/plugin developers who want to build Rust plugins, extend Lua surfaces,
  contribute data, or understand the loader/SDK architecture.

The reverse-engineering notes are not the main learning path. They remain in the
repository as research material. This book explains the stable project shape and
the APIs people should use.

## Mermaid

mdBook does not render Mermaid diagrams by default. Mermaid support normally
requires an extra preprocessor such as `mdbook-mermaid` or a custom HTML/JS
integration. This book starts without that dependency so local builds stay
simple. Diagrams should be written as plain Markdown lists or fenced `text`
blocks until Mermaid is explicitly added to the toolchain.
