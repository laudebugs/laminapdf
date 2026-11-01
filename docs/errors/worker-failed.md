# `worker-failed`

- Type: error
- Kind: `internal` (a bug or an environment failure, not the caller's input)

## What happened

A call through `@laminapdf/sdk/worker` failed inside the Web Worker with an error that isn't a `LaminaError`. The call was rejected with this code and the error's message; the Web Worker and its documents keep working.

## Why

The Web Worker passes lamina's own errors through unchanged and rejects caller mistakes it can detect (an unknown method, a parameter it can't receive) with `invalid-argument`. Anything else is a failure of lamina's client code, such as a result it couldn't send back or a bug in its code.

## How to fix

Report it, with the call you made and the message. The same call through the sync API on the main thread is a workaround while it is fixed.

## Example

```ts
await doc.pageCount(); // rejects with LaminaError { code: "worker-failed", kind: "internal", … } on a bug in lamina's client
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [L2a](../todos/done/L2a-worker-client.md): the worker client
