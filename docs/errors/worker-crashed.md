# `worker-crashed`

- Type: error
- Kind: `internal` (a bug or an environment failure, not the caller's input)

## What happened

The worker running `@lamina/pdf` (from `@lamina/pdf/worker`) stopped: an uncaught error, a WebAssembly trap or the browser killing it (out of memory). Every call still in flight was rejected with this code, and the client emitted a `crashed` event.

## Why

A crash isn't detected by silence: a long synchronous call can keep the worker busy without answering, so there is no heartbeat. Only the worker's `error` or `messageerror` event, or a trap the worker reports, counts as a crash ([L2a](../todos/done/L2a-worker-client.md), ADR 0009 §3.6).

## How to fix

Call `client.restart()`, then open the document again: handles from the old worker are gone. An encrypted document needs its password again. If one file keeps crashing the worker, report it.

## Example

```ts
client.on("crashed", () => client.restart());
await doc.save(); // rejects with LaminaError { code: "worker-crashed", … } if the worker died
```

## Data fields

- `reason`: what ended the worker: `error`, `messageerror` or `trap` (a crash, which emits `crashed`), or `terminated` (`client.terminate()`, `restart()`, or a call made when no worker runs).
