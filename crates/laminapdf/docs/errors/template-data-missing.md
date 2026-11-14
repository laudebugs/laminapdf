---
title: "template-data-missing"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/template-data-missing/"
---

# template-data-missing

- Type: error
- Kind: `usage` (the caller passed something wrong)

## What happened

A document template asked its data for a value the data doesn't have. A `{ "$data": "/invoice/number" }`, an `{ "$each": "/items", … }` or an `{ "$item": "/name" }` points to a place in the data where nothing is. Nothing was laid out.

## Why

A template is a document with placeholders that point into the data with JSON Pointers ([RFC 6901](https://www.rfc-editor.org/rfc/rfc6901)). Filling it replaces each placeholder with the value it points to. Leaving a gap would print a document with something missing, so filling stops instead.

Only `$if` accepts a missing value: it leaves its block out.

## How to fix

`data.pointer` is the pointer the template used. Add the value to the data, or correct the pointer in the template. A pointer starts with `/` and names each key on the way down, with an array's items by number from 0: `/items/0/name`. Inside an `$each`, `$item` points into the current item, not the whole data.

## Example

```rust
let document = flow::document::fill(&template, &data)?; // fails when the data lacks a value the template names
```

## Data fields

- `pointer`: the JSON Pointer the template used.
- `placeholder`: which placeholder: `$data`, `$each` or `$item`.
