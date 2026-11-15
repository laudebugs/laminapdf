---
"@laminapdf/viewer": minor
---

A view that an app hides, for example under a page grid, keeps its place. Before, hiding the view's element refitted a fit zoom to no width, so the view came back on another page. Now a hidden view draws nothing, even when edits change its pages, and returns to where it was once shown, with no new Back entry. A page it was on moves with the edit. A jump made as it shows, such as `goToPage`, goes where the jump says.

The document worker answers two more questions about edits. `client.wouldViolate(ops)` lists the signatures or certification a save would break if the ops were applied, before they are. An edit's `EditResult` now carries what it did to links, bookmarks and form fields (`references` and `fields`), so an app can tell the person what moved.
