# `profile-version-conflict`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`BuildOptions::with_version(PdfVersion::V2_0)` was combined with a conformance profile (`BuildOptions::with_profile`) that is built on PDF 1.7. `Builder::finish` wrote nothing.

## Why

PDF/UA-1 (ISO 14289-1), PDF/A-2 (ISO 19005-2) and PDF/A-3 (ISO 19005-3) each define their rules on top of ISO 32000-1, which is PDF 1.7. A file that says it is PDF 2.0 can't conform to them; validators such as veraPDF check the header first.

## How to fix

Leave the version at its default, 1.7 (or choose a lower one: the builder raises it to what the content needs). PDF 2.0 has its own profiles, PDF/A-4 and PDF/UA-2, which lamina doesn't write yet.

## Example

```rust
let options = BuildOptions::default()
    .with_version(PdfVersion::V2_0)
    .with_profile(Profile::PdfA2b);
Builder::new(options).finish()?; // profile-version-conflict
```

## Data fields

- `profile`: the profile, such as `PDF/A-2b`.
