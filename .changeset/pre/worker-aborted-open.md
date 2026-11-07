---
"@laminapdf/sdk": patch
---

The `@laminapdf/sdk/worker` worker now frees the document of an `open()` or `openUrl()` you abort just as it finishes. Before, the call rejected with `cancelled`, but the document stayed in the worker's memory until the worker stopped. An `open()` aborted while it ran always leaked its document, since the worker reads the cancel only after the synchronous open has answered.
