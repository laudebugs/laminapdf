# `missing-lang`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A PDF/UA-1 document (`Profile::PdfUa1`) reached `Builder::finish` without a natural language: `Builder::lang` was never called, or was given a blank tag.

## Why

A screen reader picks its voice and pronunciation from the document's language (`/Lang` in the catalog, ISO 32000-2 §14.9.2). PDF/UA-1 §7.2 requires that the language of all text can be determined (Matterhorn Protocol 11-001 and 11-006).

## How to fix

Set the document's language with a BCP 47 tag. Text in another language can carry its own tag with `Tag::with_lang`.

## Example

```rust
builder.lang("en-GB");
```

## Data fields

None.
