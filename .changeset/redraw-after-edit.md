---
"@laminapdf/viewer": patch
---

Edits show on screen. `viewer.applyEdit(ops)` applies `@laminapdf/sdk`'s ops to the open document as one transaction and redraws only the pages they touched: a turned page draws turned, at its new size, while every other page keeps what it drew. `ViewerClient.applyEdit(ops)` makes the edit alone, and `viewer.invalidate(pages, sizes?)` redraws any pages you name. A metadata edit redraws nothing, and a failed edit leaves the pages as they were.
