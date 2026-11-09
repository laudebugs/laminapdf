---
"@laminapdf/sdk": minor
"@laminapdf/viewer": minor
---

Read a document's outline and named destinations, and move through it with thumbnails. `Document.outline()` gives the outline's items with their titles, whether each opens, and where each goes: a page and a view in page space, whatever the file wrote. `Document.namedDestination(name)` finds one named destination. Both need the extract or text module, like `page.text()`.

In `@laminapdf/viewer`, `ViewerClient.outline()` and `namedDestination(name)` read them in the document Web Worker, and `Viewer.goToDestination(target)` goes there, with each kind of fit. `ThumbnailStrip` is a list of the pages as small pictures for a sidebar. It mounts only the thumbnails near its view, renders behind the main view's pages and goes to a page on a click or a key.
