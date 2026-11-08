# `model-failed`

- Type: error
- Kind: `internal` (something failed outside the caller's control)

## What happened

A layout model, an OCR engine or the page renderer that feeds them failed while an extraction was using it. The extraction stopped and returned no partial result.

## Why

Models are plug-ins: the extraction calls the model and the renderer it was given, and stops when either reports an error. The error's message is the plug-in's own, and never holds text from the document.

## How to fix

Check the plug-in's own logs or setup: a missing weights file, a runtime that failed to load, a page too large to render. To get a result meanwhile, extract without the model; the rules alone still give every page's blocks.

## Example

```rust
let options = laminapdf::extract::ExtractOptions::new(); // no models: rules only
let extraction = doc.extract(&options)?;
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [ADR 0030](../decisions/0030-extraction-output-and-model-plugins.md) §6
- [L3c](../todos/done/L3c-model-plugins.md)
