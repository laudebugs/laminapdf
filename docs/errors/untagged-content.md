# `untagged-content`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Under PDF/UA-1, something was drawn outside every structure element and artifact: a path, an image or text that is neither part of the document's structure nor marked as decoration.

## Why

Assistive technology reads a tagged document through its structure tree, and skips content outside it or reads it out of order. For this reason, the PDF/UA-1 standard for accessible PDF (ISO 14289-1 §7.1) requires every piece of content to be tagged or marked as an artifact. This is condition 01-005 of the Matterhorn Protocol, the PDF Association's checklist of PDF/UA failures.

## How to fix

Draw real content inside `Surface::tag` (or give a text run `TextRun::tag`) and decoration, such as rules, backgrounds and page numbers, inside `Surface::artifact`. `Builder::tagging_diagnostics` lists what is still untagged on each page.

## Example

```rust
surface.path(|p| { p.rect(rule); }).fill(FillRule::NonZero);
builder.finish()?; // untagged-content: wrap it in surface.artifact(ArtifactKind::Layout, …)
```

## Data fields

- `page`: the page's index (0-based).
- `content`: what was drawn: `path`, `image` or `text`.
- `count`: how many drawings of that kind.
