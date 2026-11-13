---
title: "worker-version-mismatch"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/worker-version-mismatch/"
---

# worker-version-mismatch

- Type: error
- Kind: `usage` (the caller did something the API doesn't allow)

## What happened

`connect()` from `@laminapdf/sdk/worker` started a Web Worker, and its handshake named a different protocol or package version than the client. The client rejected `connect()` and sent nothing else.

## Why

The client and the Web Worker exchange messages whose shape belongs to one build. A Web Worker from another version (an older deploy, a CDN copy, a separately bundled copy) could misread them, so the handshake fails loudly instead. PDF.js learned the same lesson.

## How to fix

Let the bundler serve the Web Worker from the same `@laminapdf/sdk` package as the page: `connect()` contains the literal `new Worker(new URL(…, import.meta.url))` that Vite and webpack follow. If you pass `{ worker }` yourself, build it from the same package version.

## Example

```ts
const client = await connect({ worker: new Worker("/old/lamina-worker.js", { type: "module" }) });
// rejects: LaminaPdfError { code: "worker-version-mismatch", … }
```

## Data fields

- `expected`: the client's `{ protocol, version }`.
- `actual`: what the Web Worker answered.
