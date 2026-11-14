# `pdfa-annot-flags`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/A, and an annotation the edits added or changed doesn't print, or is hidden. Saving the file as it is writes one that no longer meets PDF/A.

## Why

PDF/A keeps a document looking the same wherever it is shown or printed, so every annotation must print and none may be hidden (ISO 19005-2 §6.3.2). The annotation's flags (`/F`) control both: the Print flag must be set, and Hidden, Invisible, NoView and ToggleNoView must be clear.

## How to fix

Change the annotation's flags back: set Print and clear the hiding flags. Lamina PDF's new annotations have the right flags unless the edit names others. If the file needn't stay PDF/A, save anyway: the warning never blocks a save.

## Example

`{ "op": "updateAnnot", "args": { "id": "a12.0", "flags": 2 } }` on a PDF/A-2b file hides the annotation, and `doc.plan()` lists `pdfa-annot-flags`.

## Data fields

- `annotations`: the IDs of the annotations with flags PDF/A forbids, in page order.

## For contributors

The decisions and todos behind this code:

- [E4c](../todos/done/E4c-appearances-markup-notes-caret.md): text markup, notes and carets
- [E4 plan §3](../plans/editor/e4-markup-and-drawing-annotations.md#3-conformance-the-edits-must-keep): conformance the edits must keep
