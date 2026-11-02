# `repaired-base`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

An incremental update was requested for a document whose xref needed repair.

## Why

Appending to a repaired file would build on offsets that are wrong.

## How to fix

Save with a full rewrite instead.

## Example

```rust
let doc = Document::open_bytes(bytes_needing_repair, OpenOptions::default())?; // warns xref-repaired
let err = doc.save(&SaveOptions::default().with_mode(SaveMode::Incremental));
// repaired-base; SaveMode::Auto (the default) writes a full rewrite instead
```

## Data fields

None.
