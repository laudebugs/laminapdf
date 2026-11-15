---
title: "worker-crashed"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/worker-crashed/"
---

# worker-crashed

- Type: error
- Kind: `internal` (a bug or an environment failure, not the caller's input)

## What happened

The Web Worker running `@laminapdf/sdk` (from `@laminapdf/sdk/worker`) stopped: an uncaught error, a WebAssembly trap or the browser killing it (out of memory). Every call still in flight was rejected with this code, and the client emitted a `crashed` event.

## Why

A long synchronous call can keep the Web Worker busy without answering, so Lamina PDF has no heartbeat and never treats silence as a crash. Only the Web Worker's `error` or `messageerror` event, or a trap it reports, counts as a crash.

## How to fix

Call `client.restart()`, then open the document again: handles from the old Web Worker are gone. An encrypted document needs its password again. If one file keeps crashing the Web Worker, report it.

## Example

```ts
client.on("crashed", () => client.restart());
await doc.save(); // rejects with LaminaPdfError { code: "worker-crashed", … } if the Web Worker died
```

## Data fields

- `reason`: what ended the Web Worker: `error`, `messageerror` or `trap` (a crash, which emits `crashed`), or `terminated` (`client.terminate()`, `restart()`, or a call made when no Web Worker runs).
