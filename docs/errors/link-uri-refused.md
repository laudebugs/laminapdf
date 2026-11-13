# `link-uri-refused`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A bookmark's web address isn't a full `http`, `https` or `mailto` address in plain ASCII.

## Why

Lamina PDF writes only addresses a PDF viewer can open safely. Other schemes, `javascript:` among them, can run code or open local files. The PDF standard also stores a web address as 7-bit ASCII.

## How to fix

Use the full address, with its scheme and host, such as `https://example.com/page`. Percent-encode spaces and characters outside ASCII (`%20`, `%C3%A9`).

## Example

```rust
let target = OutlineTarget::Uri("javascript:alert(1)".into());
doc.apply(vec![Op::from(AddOutlineItem::new(None, SiblingPosition::Last, "Run", target))])?; // link-uri-refused
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E3b](../todos/done/E3b-bookmarks-and-destinations.md): the bookmark editor and named destinations
