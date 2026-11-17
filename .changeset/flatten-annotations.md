---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

You can now flatten comments into the page. `doc.flattenAnnotations(scope, intent)` draws each annotation's appearance into its page where it was, then removes the annotation with its pop-up and replies, as one undo step. The scope is `"all"`, `{ pages }` or `{ annotations }`, and the intent `"view"` or `"print"` decides which marks count as shown. The new `flattenAnnots` op does the same for annotations you name, in every module. A hidden mark stays as it is, and so does one that has no appearance and is of a kind Lamina PDF can't draw, such as a file attachment, which the edit's warnings list as `flatten-no-appearance`. Naming a form field's widget fails with `flatten-widget`, and a file that claims PDF/UA-1 gets `flatten-untagged` when you save, because the comments leave the structure tree.
