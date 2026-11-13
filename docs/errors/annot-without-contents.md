# `annot-without-contents`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/UA-1, and an annotation the edits added or changed has no description (`/Contents`). Saving the file as it is writes one that no longer meets PDF/UA-1.

## Why

PDF/UA-1 asks every annotation for a description that a screen reader can read in place of the mark (ISO 14289-1 §7.18.1). A highlight without one is silent: the reader hears that something is marked, not what. Lamina PDF fills the description with the marked text when the edit comes from a text selection, so this warning means the edit gave an empty one or removed it.

## How to fix

Give the annotation a description: for a highlight, underline or strike-out, the text it marks; for a note, its comment; for a caret, what goes there. If the file needn't stay PDF/UA-1, save anyway: the warning never blocks a save.

## Example

A highlight added to a PDF/UA-1 report with `"contents": ""`, then `doc.plan()`, lists `annot-without-contents` with the highlight's ID in `annotations`.

## Data fields

- `annotations`: the IDs of the annotations without a description, in page order.

## For contributors

The decisions and todos behind this code:

- [E4c](../todos/ongoing/E4c-appearances-markup-notes-caret.md): text markup, notes and carets
- [E4 plan §3](../plans/editor/e4-markup-and-drawing-annotations.md#3-conformance-the-edits-must-keep): conformance the edits must keep
