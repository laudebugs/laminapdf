---
title: "invalid-document"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/invalid-document/"
---

# invalid-document

- Type: error
- Kind: `usage` (the caller passed something wrong)

## What happened

A `laminapdf.document/1` document didn't match its format: it has a key the format doesn't know, it lacks a key the format needs, or a value is of the wrong kind, such as text where a number belongs. Nothing was laid out.

## Why

A document describes its pages as JSON, so a program or a language model can write it and a schema can check it before anything is drawn. Lamina PDF reads only the keys the format defines. An unknown key is refused rather than ignored, because a misspelt key would otherwise drop its value without a word: `"spaceAftr": 12` would leave the space out of the PDF.

The same check runs after a template is filled, so a filled template must also be a valid document.

## How to fix

`data.pointer` is the JSON Pointer of the value at fault, such as `/blocks/3/style`. Correct that value. For a misspelt key, `didYouMean` lists the closest keys the format knows there.

A style, font or image the document names must also be defined: the style in the document's `styles`, the font and image among the files given beside it. `didYouMean` suggests only the format's own keys, never the document's names.

## Example

```rust
let document = FlowDocument::from_json(&json)?; // fails here, before any page is laid out
builder.document(&document, &resources)?;
```

## Data fields

- `pointer`: the JSON Pointer of the value, from the document's root.
- `reason`: what is wrong there: `unknown-key`, `missing-key`, `wrong-type`, `out-of-range` (a size, width or count below what the layout can use, or a heading level outside 1 to 6), `unknown-style`, `unknown-font`, `unknown-image` or `unsupported-schema`.
