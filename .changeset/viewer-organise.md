---
"@laminapdf/viewer": minor
---

A view that an app hides, for example under a page grid, keeps its place. Before, hiding the view's element refitted a fit zoom to no width, so the view came back on another page. Now a hidden view draws nothing, even when edits change its pages, and, once shown again, returns to where it was, with no new Back entry. If an edit moved or deleted the page it was on, it returns to that page's new place, or to the nearest page that stayed. A jump made as the view shows again, such as `goToPage`, wins over the kept place.

The document worker answers two more questions about edits. Before an app applies ops, `client.wouldViolate(ops)` lists the signatures or the certification that saving them would break. An edit's `EditResult` now carries what it did to links, bookmarks and form fields (`references` and `fields`), so an app can tell the person what moved.
