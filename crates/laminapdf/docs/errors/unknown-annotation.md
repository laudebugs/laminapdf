---
title: "unknown-annotation"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/unknown-annotation/"
---

# unknown-annotation

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An annotation op names an annotation by an ID that no page of the document lists in its `/Annots`.

## Why

The annotation was deleted, by an earlier op or along with its parent comment, or the ID comes from another document. An ID read before an undo can also name an annotation that the undo took away.

## How to fix

Read the page's annotations again and take the ID from that list.

## Example

```json
{ "op": "deleteAnnot", "args": { "id": "a99.0" } }
```

No page lists object 99 as an annotation.

## Data fields

None. The `location` is the annotation's `object` and `id`.
