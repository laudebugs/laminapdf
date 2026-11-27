---
title: "xfa-removed"
version: "0.8.0"
url: "https://docs.laminapdf.com/reference/codes/xfa-removed/"
---

# xfa-removed

- Type: warning
- Kind: `unsupported` (valid input that Lamina PDF does not support yet)

## What happened

Flattening a form removed its XFA part while fields other than signatures were left. Those fields stay fillable as a plain PDF form; anything only the XFA part did, such as its own scripts, is gone.

## Why

Some forms carry their fields twice: as PDF form fields, and in an XFA part that Adobe's apps show instead. Flattening draws the chosen fields into their pages and removes them from the PDF form, but it can't remove them from the XFA part. Left in place, the XFA part would make Adobe's apps show those fields again, empty and editable, over the answers drawn on the page, so Lamina PDF removes it.

## How to fix

Nothing: the flattened answers are on the page, and the fields that are left keep their values. Flatten every field at once if you want no form left at all. The warning never blocks the edit or the save.

## Example

```json
{ "op": "flattenForm", "args": { "fields": ["f12.0"], "intent": "print" } }
```

Applied to a form with an XFA part, where `f12.0` is its `name` field and two other text fields stay.

## Data fields

None.
