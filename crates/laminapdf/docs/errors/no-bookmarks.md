---
title: "no-bookmarks"
version: "0.8.0"
url: "https://docs.laminapdf.com/reference/codes/no-bookmarks/"
---

# no-bookmarks

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Document::create_contents_page` or `contents_listing` was asked for a contents page, but none of the document's bookmarks goes to one of its pages.

## Why

A contents page lists the bookmarks, each with the page it goes to. Bookmarks that open a web address or another file have no page in the document, so they are left out. When every bookmark is like that, or the document has none, the page would list nothing.

## How to fix

Add bookmarks that go to the document's pages, then create the contents page.

## Example

```rust
let mut doc = Document::open_bytes(bytes, OpenOptions::default())?; // a file with no bookmarks
doc.create_contents_page(&ContentsOptions::new(), &ContentsFonts::new())?; // no-bookmarks
```

## Data fields

None.
