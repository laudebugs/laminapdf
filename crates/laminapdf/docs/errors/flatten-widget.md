---
title: "flatten-widget"
version: "0.8.0"
url: "https://docs.laminapdf.com/reference/codes/flatten-widget/"
---

# flatten-widget

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An edit asked to flatten an annotation that is a form field's widget. Flattening annotations draws comments and marks into the page and leaves form fields alone, so nothing was changed.

## Why

A widget is the part of a form field you see and type into. Removing it alone would leave its field in the form with nothing to show it, so `flatten_form` flattens a field together with all of its widgets. A signature field is never flattened: it would make a signature look present when it can no longer be checked.

## How to fix

Leave widgets out of the annotations you flatten. `Document::flatten_annotations` with `FlattenScope::All` or `FlattenScope::Pages` (`doc.flattenAnnotations` in TypeScript) leaves them out for you. To flatten the form fields, use `Document::flatten_form` (`doc.flattenForm`).

## Example

```json
{ "op": "flattenAnnots", "args": { "annots": ["a18.0"], "intent": "view" } }
```

Applied to a page where `a18.0` is a text field's widget.

## Data fields

None. The `location` is the widget's `object` and `id`.
