---
title: "annot-name-duplicate"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/annot-name-duplicate/"
---

# annot-name-duplicate

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

Two annotations on the same page have the same name in their `/NM` entry. ISO 32000-2 §12.5.2 asks for that name to be unique on its page, because comment exchange files (XFDF, FDF) and replies match annotations by it.

## Why

A writer copied an annotation without giving the copy a new name, or gave every annotation it made the same name.

## How to fix

Nothing to do: both annotations are read and kept, and Lamina PDF tells annotations apart by their own IDs, never by `/NM`. An import that matches comments by name may match the wrong one of the two, so a writer that makes new annotations should give each its own name.

## Example

```text
5 0 obj << /Type /Annot /Subtype /Text /NM (note-1) /Rect [72 700 92 720] >> endobj
6 0 obj << /Type /Annot /Subtype /Text /NM (note-1) /Rect [72 650 92 670] >> endobj
```

Both are on one page, so object 6 raises the warning.

## Data fields

None. The `location` is the second annotation's `object`, none for a direct dictionary.
