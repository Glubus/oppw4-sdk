# fx_director

`fx_director` is the current home for difficulty and effect experiments.

Known state:

- there is a difficulty probe;
- more runtime addresses need to be confirmed with the game binary;
- menu initialization and the global difficulty variable still need investigation;
- LinkData alone may not be enough to fully understand difficulty behavior.

The plugin should keep probes explicit and configurable. Anything that writes memory or installs hooks must declare the matching capability and fail cleanly when the host cannot provide it.

Before this becomes a stable feature, we need:

- the place where difficulty is initialized in menus;
- the runtime value that represents global difficulty;
- how per-entry difficulty data is represented, if it exists;
- whether forced difficulty should be a memory write, a hook, a LinkData patch, or a combination.
