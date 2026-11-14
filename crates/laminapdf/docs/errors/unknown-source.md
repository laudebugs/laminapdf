---
title: "unknown-source"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/unknown-source/"
---

# unknown-source

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An op, or a call that reads a source document, names a source by an ID that this session doesn't hold. A source document is another PDF that ops copy pages from, and the session only knows the ones added to it.

## Why

The source was never added, or it was added to another session. An op log names sources only by ID, so a log replayed in a new session (after the document reopens, or on another device) needs its sources added again first.

## How to fix

Add the source document's bytes with `add_source` (in Rust; the bindings come with the ops that copy pages) before applying ops that name it. The same bytes always give the same ID, so adding the file again restores the source the log names.

## Example

```rust
let id = other_session.add_source(bytes, None)?;
doc.source_document(&id)?; // unknown-source: `doc` never added these bytes
```

## Data fields

None.
