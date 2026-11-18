---
title: "unknown-blob"
version: "0.6.0"
url: "https://docs.laminapdf.com/reference/codes/unknown-blob/"
---

# unknown-blob

- Type: error
- Kind: `usage` (the call or its arguments are wrong)

## What happened

An edit names bytes, such as a replacement image, by an ID that the editing session doesn't hold.

## Why

Edits stay small by naming large payloads by the SHA-256 hash of their bytes instead of carrying the bytes. The session keeps every payload added to it, so undo, redo and a replayed list of edits find them again. This edit names an ID that was never added, so the session has nothing to use.

## How to fix

Add the bytes to the document first, then apply the edit with the ID that call returns: `Document::add_blob` in Rust, `doc.addBlob(bytes)` in TypeScript. When you replay a saved list of edits into a newly opened document, add its payloads again before the edits.

## Example

```json
{ "op": "editContent", "args": { "page": "p3.0", "edits": [{ "replaceImage": { "object": { "ordinal": 2, "hash": "a3f1c2d4e5f60718" }, "blob": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08" } }] } }
```

Applied to a document whose session holds no bytes with that ID.

## Data fields

None.
