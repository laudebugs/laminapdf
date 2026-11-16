# `appearance-needs-text`

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF does not support yet)

## What happened

An edit adds a text box or callout, or changes one, but this build of Lamina PDF can't lay out its text. The box uses a font given by its bytes, or fallback fonts, and the layout those need was left out of the build.

## Why

Laying out text in any font takes shaping, line breaking and font subsetting, about 470 KB of WebAssembly once compressed. The WebAssembly modules most apps load first leave that out to stay small. The extract module, which the reader uses, lays out boxes in Helvetica, Times and Courier with their built-in metrics. The base module lays out no text boxes at all. The text module and the native crates lay out every box.

## How to fix

Use Helvetica, Times or Courier for the box, or open the document in the text module: call `init({ text: true })` before `Document.open`. In Rust, the `laminapdf` crate with its default features lays out every box.

## Example

```json
{ "op": "addAnnot", "args": { "page": "p3.0", "spec": { "type": "freeText", "rect": { "x0": 72, "y0": 700, "x1": 272, "y1": 730 }, "text": [[{ "text": "مرحبا" }]], "font": "5f6c…" } } }
```

Applied to a document opened in the extract module.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E4e](../todos/ongoing/E4e-freetext-callouts.md): text boxes and callouts
- [E4 plan §2.2](../plans/editor/e4-markup-and-drawing-annotations.md#22-where-appearances-are-generated): where appearances are generated
