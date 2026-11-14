# `link-not-tagged`

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF does not support yet)

## What happened

The file claims PDF/UA-1, the standard for accessible PDFs, but it isn't tagged: it has no structure tree, so a new link has no element to join.

## Why

PDF/UA-1 asks for every link to be a `Link` element of the structure tree, which tells a screen reader what the link is and which text it covers. A link added outside the tree would break the file's claim, so Lamina PDF refuses it instead.

## How to fix

Export it again as a tagged PDF from the program that made it, then add the link. If the claim is wrong, remove it, or add the link to a copy that doesn't claim PDF/UA-1.

## Example

```json
{ "op": "addLink", "args": { "page": "p3.0", "rect": { "x0": 72, "y0": 700, "x1": 180, "y1": 714 }, "target": { "uri": "https://example.com/" } } }
```

The file's XMP metadata has `pdfuaid:part` 1, but its catalog has no `/StructTreeRoot`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E3c](../todos/done/E3c-links.md): link annotations
