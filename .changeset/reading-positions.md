---
"@laminapdf/viewer": minor
---

The viewer can put a reader back where they stopped. `client.open()` now answers with `fingerprint`, a hash of the file's first revision (`"fp1:"` and a SHA-256). It stays the same for a renamed copy and after an incremental update, so it can key a saved position. `viewer.viewPosition()` gives the place in view as a page and point, the zoom and the rotation, and `viewer.restore(position)` goes back there as a history jump, so Back returns to page 1. Restored in the task that opened the document, the first frame paints that page and no other. A view opened or restored while its container is hidden now waits until it is shown, then starts at the top or at the restored place. Before, it kept the previous document's scroll.
