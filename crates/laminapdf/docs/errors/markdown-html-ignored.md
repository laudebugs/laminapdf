---
title: "markdown-html-ignored"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/markdown-html-ignored/"
---

# markdown-html-ignored

- Type: warning
- Kind: `unsupported` (valid input that Lamina PDF can't handle yet)

## What happened

`Builder::markdown` found raw HTML in the Markdown, such as a `<div>` block or an inline `<span>`, left it out of the PDF and laid out the rest as usual. The builder raises one warning for each HTML block and each inline HTML tag.

## Why

Markdown lets authors mix in HTML, and a browser would draw it. Laying out HTML needs a web page's layout rules, which a PDF builder doesn't have. Rather than print the tags as text, the builder leaves the HTML out and says where it was.

## How to fix

Write the content in Markdown instead: `**bold**` for `<b>`, a table for `<table>`, an image for `<img>`. If the HTML is only a comment, such as `<!-- draft -->`, remove it or ignore the warning.

## Example

```rust
let report = builder.markdown("Hello <span>there</span>\n", &MarkdownBuildOptions::new())?;
assert_eq!(report.warnings[0].code(), "markdown-html-ignored");
```

## Data fields

- `offset`: where the HTML starts, as a byte offset into the Markdown.
