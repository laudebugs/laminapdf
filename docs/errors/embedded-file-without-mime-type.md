# `embedded-file-without-mime-type`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Under a PDF/A profile, `Builder::attach` added a file whose MIME type, such as `text/xml`, is empty.

## Why

Software reading an archive years later needs to know what each attached file is. The PDF/A-3 archival standard (ISO 19005-3 §6.8) requires every embedded file to name its MIME type in the `/Subtype` of its stream, and lamina writes no `/Subtype` for an empty one.

## How to fix

Pass the file's MIME type to `Attachment::new`. For bytes of no particular type, use `application/octet-stream`.

## Example

```rust
let options = BuildOptions::default().with_profile(Profile::PdfA3b);
let mut builder = Builder::new(options);
builder.attach(Attachment::new("invoice.xml", xml, ""));
builder.finish()?; // embedded-file-without-mime-type: pass "text/xml"
```

## Data fields

- `profile`: the profile, such as `PDF/A-3b`.
- `name`: the file's name (untrusted: the caller's text).
