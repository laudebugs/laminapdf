---
title: "tag-element-unclosed"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/tag-element-unclosed/"
---

# tag-element-unclosed

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Builder::finish` was called while structure elements opened with `Surface::open_element` were still open.

## Why

An element that continues across pages is written when `Builder::close_element` closes it, since only then is its content complete. An element never closed would leave the structure tree with a reference to an element that doesn't exist (ISO 32000-2 §14.7.2), so the build stops instead.

## How to fix

Close every element from `Surface::open_element` with `Builder::close_element` once its last page is finished, before `Builder::finish`.

## Example

```rust
let paragraph = page.surface().open_element(TagKind::P);
page.finish()?;
builder.finish()?; // tag-element-unclosed: the paragraph is still open
```

## Data fields

- `count`: how many elements are still open.
