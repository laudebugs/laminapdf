# `out-of-memory`

- Type: error
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

The WebAssembly module needed more memory than it could get, so it stopped in the middle of the call. Each module may grow to at most 2 GiB, and a browser tab or server can run out before that. `@laminapdf/sdk` discarded the module's instance, so the next call runs in a fresh one. Every document, page and builder from the old instance now throws `instance-reset`.

## Why

A module's memory grows as the module needs more and never shrinks. Many open documents, a huge image, or a file built to exhaust memory can each push it to its limit. The open limits stop most hostile files before memory runs out, and the maximum catches the rest.

## How to fix

Open the document again, then keep less in memory at once. Free each document and builder as soon as you're done with it, or use `using`. For files from people you don't trust, lower the open limits, such as `maxStreamDecoded` and `maxImagePixels`, so they fail with `limit-exceeded` first. An app can also open each document in its own Web Worker, with `@laminapdf/sdk/worker`, and end the Web Worker afterwards: that returns all of its memory.

## Example

```ts
try {
  doc = Document.open(bytes, { limits: { maxStreamDecoded: 64_000_000 } });
} catch (e) {
  if (isLaminaPdfError(e, "out-of-memory")) showError("This file is too large to open here.");
  else throw e;
}
```

## Data fields

- `module`: the module that ran out: `base`, `text` or `extract`.
- `memoryBytes`: the size of its memory when it stopped.

## For contributors

The decisions and todos behind this code:

- [L2b](../todos/done/L2b-disposal-and-memory.md): the memory maximum and the fresh instance
- [ADR 0016](../decisions/0016-package-design.md): the maximum, set when the modules are built
- [ADR 0003](../decisions/0003-reader-tolerance-and-limits.md) §3: the limits that come first
