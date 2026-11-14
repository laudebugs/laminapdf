---
title: "not-initialized"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/not-initialized/"
---

# not-initialized

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

A function or class of `@laminapdf/sdk` (`Document.open`, `new Builder`, …) was called before `init()` had finished loading and instantiating the WebAssembly module.

## Why

`@laminapdf/sdk` is synchronous after `init()`, but fetching and compiling the `.wasm` file is asynchronous, so it happens once, up front, in `init()`. Until that promise resolves there is no module to call.

## How to fix

Await `init()` once, before the first call; calling it again is cheap (it is idempotent). In a Cloudflare Worker, pass the imported module: `await init({ module: wasm })`.

## Example

```js
import { Document, init } from "@laminapdf/sdk";
Document.open(bytes); // throws { code: "not-initialized", … }
await init();
Document.open(bytes); // works
```

## Data fields

None.
