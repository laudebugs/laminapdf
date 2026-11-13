---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

Edit bookmarks and named destinations: `addOutlineItem`, `updateOutlineItem`, `moveOutlineItem` and `deleteOutlineItem` add, change, move and delete bookmarks, with their colour and bold or italic style, and `setNamedDestination`, `renameNamedDestination` and `deleteNamedDestination` manage the names links jump to. A rename follows the bookmarks and links that use the name. The outline's links and counts stay consistent after every edit, and a broken outline is rewritten whole by the first edit. `outline()` now gives each bookmark's `id`, the name or web address it goes to, and its style; `namedDestinations()` lists every name; `destinationAt(page, view)` turns a place in the viewer into a destination. The Rust crates have the same ops (`laminapdf::AddOutlineItem` and the rest) and methods.
