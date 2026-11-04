# `disposed`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

A `Document`, `Builder` or other handle of `@laminapdf/sdk` was used after `free()` released it, or after the `using` block that held it ended (`Symbol.dispose`). A page or save job is freed with its document, and a page with its builder.

## Why

Each handle owns memory inside the WebAssembly module. Freeing it returns that memory at once, so the handle has nothing left to work on.

## How to fix

Free a handle only after its last use. To keep working with the document, open it again from its bytes, or create a new builder.

## Example

```ts
const doc = Document.open(bytes);
doc.free();
doc.save(); // throws LaminaPdfError { code: "disposed", … }
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [L2b](../todos/done/L2b-disposal-and-memory.md): the check on every handle, and on the pages and save jobs of `@laminapdf/wasm`
