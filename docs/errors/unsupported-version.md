# `unsupported-version`

- Type: error
- Kind: `unsupported` (valid input that lamina doesn't implement)

## What happened

An op log has a schema version this reader doesn't know.

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

## For contributors

The decisions and todos behind this code:

- [ADR 0006](../decisions/0006-op-log-and-undo.md) §6
