# moveset_patcher

`moveset_patcher` owns moveset patch requests.

It should receive resolved character and moveset intent from SDK-facing APIs instead of making high-level data decisions itself. The SDK and data bank resolve aliases such as `garp` versus `garp_young`; the patcher applies the final target.

Rules:

- patch only the intended runtime entry;
- do not patch both old and young variants unless the request explicitly targets both;
- keep LinkData row/entry mechanics behind the LinkData service;
- keep mod-facing APIs stable even if the underlying LinkData layout changes.

Older code had experimental helpers around entry counts and moveset changes. Those ideas can come back, but they should return as a clean public API over `sdk_linkdata`, not as raw magic numbers embedded in the patcher.
