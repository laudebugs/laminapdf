---
"@laminapdf/viewer": minor
---

A local file larger than 200 MiB now opens from a copy in the browser's origin-private file system. The document worker makes the copy once and reads it as needed, so it no longer holds the whole file in memory. Opening the same file again reads the copy already there. The worker reports copying through the client's `progress` event, and a cancel stops it. `ViewerOpenOptions.copyAbove` sets the threshold. A browser without OPFS, a copy that won't fit, a copy that fails partway and a file another tab is reading all open from memory, as before. When the copies would pass their budget, those opened longest ago go first. `ViewerOpenOptions.keepCopy: false` removes the copy when its document closes. `CACHE_FOLDER` names the OPFS folder of copies, so an app can empty it.
