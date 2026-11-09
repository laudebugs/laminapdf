---
title: "already-initialized"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/already-initialized/"
---

# already-initialized

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

`init()` or `loadText()` of `@laminapdf/sdk` named a source (a URL, `Response`, bytes or `WebAssembly.Module`) for a module that an earlier call had already loaded, or started loading, from another source (or from the default file).

## Why

Each module is instantiated once per JavaScript realm: every document and builder lives in that instance. Loading a second copy from another source would leave two instances whose objects can't be mixed, so the second source is refused rather than silently ignored.

## How to fix

Pass the source on the first call only; later calls (from other parts of the app) can call `init()` without one, which waits for the same load. Calling again with the very same source object is allowed.

## Example

```js
await init(new URL("/wasm/lamina_pdf_base.wasm", location.href));
await init(); // fine: the module is loaded
await init(otherBytes); // throws { code: "already-initialized", … }
```

## Data fields

None.
