---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Describe an image for screen readers. `doc.content.setAlt(index, object, alt)` sets the alternative text (`/Alt`) of the `Figure` element that owns an object a page draws, or the nearest `Figure` above it, and replaces the text the figure had. The text can't be empty and holds at most 2,000 characters. An untagged file, an object outside marked content and an object no figure owns are refused with `invalid-op`. An object that is drawn inside a tagged form, or whose marked content belongs to more than one figure, is refused too. Objects that share a figure share its text. `doc.content.objects(index)` now reports the figure's current text as `alt` on an object that one figure owns, and in Rust `PageObject::alt` does. The page's content stays as it was, and undo brings the old text back. In the Rust library, the `SetAlt` op does the same.
