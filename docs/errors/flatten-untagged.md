# `flatten-untagged`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file claims PDF/UA-1, and an edit flattened comments into the page. Their drawings are now page content marked as decoration (artifacts), which screen readers skip, and the comments themselves left the structure tree, so a screen reader no longer reads their text.

## Why

PDF/UA-1 asks that every annotation be tagged, with its text as its description. A flattened comment is no longer an annotation: what it drew is part of the page, and the words it held are gone.

## How to fix

Save knowing the file no longer meets PDF/UA-1, or keep the comments as annotations and flatten a copy of the file instead. The warning never blocks the edit or the save.

## Example

Flattening the comments of a file whose XMP metadata has `pdfuaid:part` 1, then `doc.plan()`, lists `flatten-untagged`.

## Data fields

- `annotations`: the IDs of the flattened annotations.

## For contributors

The decisions and todos behind this code:

- [E5e](../todos/ongoing/E5e-flatten-and-summary.md): flattening annotations
- [E5 plan §3](../plans/editor/e5-review-stamps-interchange.md#3-conformance-the-edits-must-keep): flattening in a tagged file
