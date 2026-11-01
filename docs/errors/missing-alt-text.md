# `missing-alt-text`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Under PDF/UA-1, a page has a `Figure` element without alternate text: neither `/Alt` (`Tag::with_alt`) nor `/ActualText` (`Tag::with_actual_text`).

## Why

A screen reader can't describe a picture by itself. PDF/UA-1, the standard for accessible PDF (ISO 14289-1), in §7.3 requires every `Figure` to carry a text alternative (Matterhorn Protocol 13-004; the protocol is the PDF Association's checklist of PDF/UA failures). lamina checks only that a description exists; whether it describes the figure well is for a person to judge.

## How to fix

Describe the figure with `Surface::figure(&image, rect, alt)` or `Tag::new(TagKind::Figure).with_alt(…)`. If the picture is decoration that carries no meaning, draw it inside `Surface::artifact` instead.

## Example

```rust
surface.tag(TagKind::Figure, |s| { s.image(&chart, rect); Ok(()) })?;
builder.finish()?; // missing-alt-text: use Tag::new(TagKind::Figure).with_alt("Sales by month")
```

## Data fields

- `page`: the page's index (0-based).
