# `unsupported-version`

- Type: error
- Kind: `unsupported` (valid input that lamina doesn't implement)

## What happened

An op log has a schema version this reader doesn't know (ADR 0006 §6).

## Why

A newer lamina wrote it.

## How to fix

Upgrade lamina.

## Example

```rust
doc.apply_log(r#"{ "v": 9, "ops": [] }"#)?; // unsupported-version, data.version = 9
```

## Data fields

`version`.
