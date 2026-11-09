---
"@laminapdf/viewer": minor
---

A local file larger than 200 MiB now opens from a copy in the browser's origin-private file system (OPFS). The document worker makes the copy once, reuses it on later opens and reads it as needed, so it no longer holds the whole file in memory. The worker reports copying through the client's `progress` event, and a cancel stops it. `ViewerOpenOptions.copyAbove` sets the threshold.

The file opens from memory, as before, when the browser lacks OPFS, the copy won't fit or fails partway, or another tab is reading the file. When the copies would pass their budget, those opened longest ago are deleted first. `ViewerOpenOptions.keepCopy: false` removes the copy when its document closes. `CACHE_FOLDER` names the OPFS folder of copies, so an app can empty it.
