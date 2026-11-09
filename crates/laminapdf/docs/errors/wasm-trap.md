---
title: "wasm-trap"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/wasm-trap/"
---

# wasm-trap

- Type: error
- Kind: `internal` (a bug in Lamina PDF or a broken invariant)

## What happened

The WebAssembly module trapped during the call: Lamina PDF hit a bug and stopped. `@laminapdf/sdk` discarded the module's instance, so the next call runs in a fresh one. Every document, page and builder from the old instance now throws `instance-reset`.

## Why

A trap stops the module in the middle of its work, so its memory may hold half-written state. Nothing in that instance can be trusted afterwards. A fresh instance keeps one failed call from breaking every call after it.

## How to fix

Open the document again from its bytes and retry. If the same call on the same file traps again, report it with the file: a trap is always a bug in Lamina PDF.

## Example

```ts
try {
  lines = doc.page(0).text().lines;
} catch (e) {
  if (!isLaminaPdfError(e, "wasm-trap")) throw e;
  doc = Document.open(bytes); // the old document throws instance-reset from now on
}
```

## Data fields

- `module`: the module that trapped: `base`, `text` or `extract`.
