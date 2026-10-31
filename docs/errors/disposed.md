# `disposed`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

A `Document`, `Builder` or other handle of `@lamina/pdf` was used after `free()` released it, or after the `using` block that held it ended (`Symbol.dispose`).

## Why

Each handle owns memory inside the WebAssembly module. Freeing it returns that memory at once, so the handle has nothing left to work on ([L2b](../todos/ready/L2b-disposal-and-memory.md) adds the check that raises this code).

## How to fix

Free a handle only after its last use. To keep working with the document, open it again from its bytes, or create a new builder.

## Example

```ts
const doc = Document.open(bytes);
doc.free();
doc.save(); // throws LaminaError { code: "disposed", … }
```

## Data fields

None.
