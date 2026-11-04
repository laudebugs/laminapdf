# `instance-reset`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

A document, page, builder or save job was used after its WebAssembly instance was replaced. `reset()` replaces every loaded module's instance, and a trap (`wasm-trap` or `out-of-memory`) replaces the instance that trapped. The handle belonged to the old instance, so it has nothing left to work on.

## Why

Each handle lives in one instance's memory, and a fresh instance starts empty. Lamina PDF can't move a document across, so every handle from the old instance stops working at once.

## How to fix

Open the document again from its bytes, or create a new builder. Unsaved edits are lost with the old instance, so save often in an app that calls `reset()` or may run out of memory.

## Example

```ts
const doc = Document.open(bytes);
reset();
doc.pageCount; // throws LaminaPdfError { code: "instance-reset", … }
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [L2b](../todos/ongoing/L2b-disposal-and-memory.md): `reset()` and trap recovery
