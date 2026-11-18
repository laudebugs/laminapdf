# `field-is-signature`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A form-filling op named a signature field, so nothing was changed.

## Why

A signature field's value is the signature itself (ISO 32000-2 §12.7.5.5). Only signing writes it: setting it like any other field's value would put something in the file that looks like a signature and isn't one.

## How to fix

Leave signature fields out of the values you set. To sign a field, use a signing operation.

## Example

```rust
let signature = doc.form()?.fields.into_iter().find(|f| f.kind == FieldKind::Signature);
// Setting a value on that field fails with field-is-signature.
```

## Data fields

None. The `location` is the field's `object` and `id`.
