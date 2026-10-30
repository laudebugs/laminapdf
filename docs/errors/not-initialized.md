# `not-initialized`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

A function or class of `@lamina/pdf` (`Document.open`, `new Builder`, …) was called before `init()` had finished loading and instantiating the WebAssembly module.

## Why

`@lamina/pdf` is synchronous after `init()` ([ADR 0016](../decisions/0016-package-design.md) §3), but fetching and compiling the `.wasm` file is asynchronous, so it happens once, up front, in `init()`. Until that promise resolves there is no module to call.

## How to fix

Await `init()` once, before the first call; calling it again is cheap (it is idempotent). In a Cloudflare Worker, pass the imported module: `await init({ module: wasm })`.

## Example

```js
import { Document, init } from "@lamina/pdf";
Document.open(bytes); // throws { code: "not-initialized", … }
await init();
Document.open(bytes); // works
```

## Data fields

None.
