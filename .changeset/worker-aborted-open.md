---
"@laminapdf/sdk": patch
---

When you abort an `open()` or `openUrl()` of `@laminapdf/sdk/worker` just as the worker finishes it, the worker now frees the document it opened. Before, the call rejected with `cancelled`, but the document stayed in the worker's memory until the worker stopped. An `open()` aborted while it ran always hit this, since the worker reads the cancel only after the synchronous open has answered.
