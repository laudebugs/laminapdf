---
title: "markdown-image-missing"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/markdown-image-missing/"
---

# markdown-image-missing

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

`Builder::markdown` met an image, such as `![Chart](chart.png)`, whose bytes weren't passed in. It left the image out of the PDF and laid out the rest of the document as usual.

## Why

The builder reads no files and fetches nothing from the network, so it can only draw images the caller supplies. An image path the options don't list, or a web address, has nothing behind it.

## How to fix

Read the image yourself and pass it with `MarkdownBuildOptions::with_image`, under the same path the Markdown uses.

## Example

```rust
let chart = Image::png(&std::fs::read("chart.png")?)?;
let options = MarkdownBuildOptions::new().with_image("chart.png", chart);
builder.markdown("![Chart](chart.png)\n", &options)?;
```

## Data fields

- `offset`: where the image starts, as a byte offset into the Markdown.
