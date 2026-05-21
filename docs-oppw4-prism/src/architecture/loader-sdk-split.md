# Loader And SDK Split

The loader has a narrow job:

- inject/load the SDK;
- provide Windows/process primitives;
- expose the minimal ABI needed by SDK core.

The loader should not contain:

- active character logic;
- SDK service policy;
- Lua module registration;
- character bank logic;
- LinkData or RDB interpretation;
- plugin business logic.

SDK core receives primitives from the loader and builds the plugin world on top of them.

This keeps the `dinput8` proxy replaceable and reduces the risk that one game-specific feature pollutes the bootstrap layer.
