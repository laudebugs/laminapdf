---
title: "unknown-field"
version: "0.8.0"
url: "https://docs.laminapdf.com/reference/codes/unknown-field/"
---

# unknown-field

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A form op named a field that the document's form doesn't have, so nothing was changed.

## Why

Ops name form fields by the IDs the form view gives them. The form lists its fields in the interactive form dictionary (ISO 32000-2 §12.7.3), and an op can set a value only on a field it lists. An ID taken from another document, or from a field that has since been removed, names nothing here. So does the ID of a field that has fields below it: only the fields at the ends of the tree hold values.

## How to fix

Read the form's fields again and take the ID from that list.

## Example

```rust
let form = doc.form()?;
let name = form.fields.iter().find(|f| f.name == "name").map(|f| f.id.clone());
// Using an ID from another document instead fails with unknown-field.
```

## Data fields

None. The `location` is the field's `object` and `id`.
