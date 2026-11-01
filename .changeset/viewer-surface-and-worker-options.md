---
"@laminapdf/viewer": minor
"@laminapdf/sdk": minor
---

`@laminapdf/viewer` exports only its documented API: `ViewerClient`, `createViewer` and `Viewer`, the types of their options, events and state, the protocol types `client.call` takes, and `MIN_ZOOM`/`MAX_ZOOM`. The layout, canvas pool, bitmap cache, tile and zoom helpers, `ByteSource`, `blobSource`, `bufferSource`, `readAll`, `PRIORITY`, `renderKey` and `DEFAULT_TIMEOUTS` are no longer exported. Its worker no longer keeps a log of every render, which grew without bound, and no longer serves test-only methods. `@laminapdf/sdk/worker/server`'s `serve()` takes a `saveBudget` (work units per save step), and `workerStats()` reports the handles and calls a worker holds. `@laminapdf/sdk/worker` exports the `Emitter` its client uses.
