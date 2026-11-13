---
title: "unsupported-version"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/unsupported-version/"
---

# unsupported-version

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF doesn't implement)

## What happened

An op log has a schema version this reader doesn't know.

## Why

A newer Lamina PDF wrote it.

## How to fix

Upgrade Lamina PDF.

## Example

```rust
doc.apply_log(r#"{ "v": 9, "ops": [] }"#)?; // unsupported-version, data.version = 9
```

## Data fields

`version`.
