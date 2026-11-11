---
title: "template-data-type"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/template-data-type/"
---

# template-data-type

- Type: error
- Kind: `usage` (the caller passed something wrong)

## What happened

A document template found data of the wrong kind where a placeholder pointed. Either an `$each` ran over something that isn't a list, a `$data` or `$item` put an object or a list where the document needs text or a number, or a placeholder's pointer isn't a valid JSON Pointer. Nothing was laid out.

## Why

Each placeholder needs one kind of value. `$each` repeats a block once per item, so it needs a list. A text placeholder is printed as it is, so it needs text, a number or `true`/`false`. Templates have no expressions, so they can't turn an object into text: the data arrives formatted.

## How to fix

`data.pointer` is the pointer the template used, and `data.expected` the kind it needs there. Point to a value of that kind, or change the data so the value has it. For a total or a date, put the formatted text in the data, such as `"total": "1,250.00 €"`.

## Example

```rust
let document = flow::document::fill(&template, &data)?; // fails when "/items" isn't a list
```

## Data fields

- `pointer`: the JSON Pointer the template used.
- `placeholder`: which placeholder: `$data`, `$each` or `$item`.
- `expected`: the kind the placeholder needs: `list`, `text, a number or true/false`, or `a JSON Pointer such as /items` (a pointer starts with `/`; `""` is the whole value).
