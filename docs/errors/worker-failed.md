# `worker-failed`

- Type: error
- Kind: `internal` (a bug or an environment failure, not the caller's input)

## What happened

A call through `@laminapdf/pdf/worker` failed inside the worker with an error that isn't a `LaminaError`. The call was rejected with this code and the error's message; the worker and its documents keep working.

## Why

The worker passes lamina's own errors through unchanged and rejects caller mistakes it can detect (an unknown method, a parameter it can't receive) with `invalid-argument`. Anything else is a failure of the worker client itself, such as a result it couldn't send back or a bug in its code ([L2a](../todos/done/L2a-worker-client.md)).

## How to fix

Report it, with the call you made and the message. The same call through the sync API on the main thread is a workaround while it is fixed.

## Example

```ts
await doc.pageCount(); // rejects with LaminaError { code: "worker-failed", kind: "internal", … } on a worker client bug
```

## Data fields

None.
