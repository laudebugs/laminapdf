# `appearance-unavailable`

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF does not support yet)

## What happened

An edit adds an annotation, or changes what one draws, but the document has nothing to draw the annotation's appearance with.

## Why

Every annotation Lamina PDF makes carries its own appearance: the drawing that PDF readers show. Readers that find none either draw their own version or draw nothing, so the same file looks different from one reader to the next. Lamina PDF refuses the edit instead of writing an annotation without one.

The drawing code is in the `laminapdf-build` crate. The `laminapdf` crate and the `@laminapdf/sdk` package always connect it, so only code that uses `laminapdf-ops` directly meets this error.

## How to fix

Edit through the `laminapdf` crate or `@laminapdf/sdk`. If you use `laminapdf-ops` directly, give the document a generator first: `Document::with_appearances` with `laminapdf-build`'s `AnnotationAppearances`.

## Example

```json
{ "op": "addAnnot", "args": { "page": "p3.0", "spec": { "type": "note", "at": { "x": 72, "y": 720 } } } }
```

Applied to a `laminapdf_ops::Document` opened without `with_appearances`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E4c](../todos/ongoing/E4c-appearances-markup-notes-caret.md): appearances for text markup, notes and carets
- [E4 plan §2.2](../plans/editor/e4-markup-and-drawing-annotations.md#22-where-appearances-are-generated): where appearances are generated
