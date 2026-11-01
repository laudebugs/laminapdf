# `link-without-structure`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Under PDF/UA-1, a link annotation has no `Link` structure element. The builder gives every link one automatically, except a link drawn inside an artifact, which stays outside the structure.

## Why

A screen reader finds links through the structure tree. PDF/UA-1, the standard for accessible PDF (ISO 14289-1), in §7.18.5 requires each link annotation to be nested in a `Link` element (Matterhorn Protocol 28-011; the protocol is the PDF Association's checklist of PDF/UA failures). A link inside decoration can't be reached that way.

## How to fix

Draw the link outside `Surface::artifact`, next to the text it belongs to; the builder then wraps it in a `Link` element.

## Example

```rust
surface.artifact(ArtifactKind::Layout, |s| {
    s.link(rect, LinkTarget::Uri("https://example.com".into()), Some("Example".into()));
    Ok(())
})?;
builder.finish()?; // link-without-structure
```

## Data fields

- `page`: the page's index (0-based).
- `count`: how many links on that page lack a `Link` element.
