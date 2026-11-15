---
title: "link-without-contents"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/link-without-contents/"
---

# link-without-contents

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Under PDF/UA-1, a link annotation has no description: `Surface::link` or `TextRun::link` was called with `None` (or an empty string) as its contents.

## Why

A screen reader announces a link by its `/Contents`, its alternate description. The PDF/UA-1 standard for accessible PDF (ISO 14289-1 §7.18.5) requires one on every link. This is condition 28-012 of the Matterhorn Protocol, the PDF Association's checklist of PDF/UA failures.

## How to fix

Pass a short description of where the link goes as the link's contents.

## Example

```rust
surface.link(rect, LinkTarget::Uri("https://example.com/pay".into()), None);
builder.finish()?; // link-without-contents: pass Some("Pay the invoice online".into())
```

## Data fields

- `page`: the page's index (0-based).
