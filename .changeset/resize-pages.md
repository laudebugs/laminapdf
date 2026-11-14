---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Resize pages and turn them to portrait or landscape. `doc.resizePages(ids, { width, height })` puts pages on new paper, scaling the page to fit and centring it, or with `{ mode: "extend", anchor }` keeping the content's size and placing it at an anchor. `doc.setOrientation(ids, "portrait" | "landscape", method)` turns each page shown the other way a quarter turn, by `/Rotate` (`"rotate"`, the default) or by swapping the paper's sides and turning the content into it (`"swapBox"`). The content moves as one piece, and everything that points into the page moves with it: annotations and form fields, outline items, named destinations, links' destinations and `/OpenAction`, and structure elements' Layout `/BBox`. A page whose content uses a filter Lamina PDF can't decode yet is refused with `unsupported-filter`.

In Rust: the ops `ResizePages` (with `ResizeMode` and `Anchor`) and `SetOrientation` (with `PageOrientation` and `OrientationMethod`).
