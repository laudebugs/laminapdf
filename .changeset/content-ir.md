---
"@laminapdf/wasm": patch
---

`laminapdf-content` can now edit what a page draws. `ContentOps::read_page` reads a page's content into objects (text, paths, images, forms and shadings), each with the bytes it came from. A `Recorder`, passed to the interpreter or to `laminapdf-text`'s `TextExtractor::page_with`, gives each object its graphics state and its box on the page, so `hit_test` finds the objects under a point or in a rectangle. `replace` and `remove` change objects, and `encode` writes the page again, copying every byte no edit changed. A `ContentObjectRef` names an object so the name still finds it after the page is read again, and `find` refuses the name once the object has changed. Two new error codes, `content-object-stale` and `content-not-editable`, come with the edit operations that will build on the IR: one for a stale name, one for content that can't be changed safely.
