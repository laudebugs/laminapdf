---
"@laminapdf/sdk": minor
---

Every handle on WebAssembly memory frees with `free()` or `using`: `Document`, `PageRef`, `SaveJob`, `Builder`, `Page` and `Image`. A second `free()` does nothing, and any other call afterwards throws `disposed` instead of a glue error. A page or save job is freed with its document, and freeing a document now frees its memory at once, even while pages of it are still held. Pages of a freed builder throw `disposed` too (`builder-finished` once it finishes), and a freed `Image` throws `disposed` instead of decoding again.

Each module's memory may grow to 2 GiB. Past that, the module traps and the call throws `out-of-memory`; any other trap throws `wasm-trap`. The next call runs in a fresh instance, and handles of the old one throw `instance-reset`. The new `reset()` gives grown memory back by starting every module afresh, and the new `onLeak(hook)` reports handles the garbage collector reclaims without a `free()`.
