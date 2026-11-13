---
title: "revision-index-out-of-range"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/revision-index-out-of-range/"
---

# revision-index-out-of-range

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Document::open_revision` was asked for a revision the file doesn't have.

## Why

Revisions are numbered from 0, the original file, to one less than `Document::revision_count`, the latest revision. A file that was never updated, or one opened by repairing a broken cross-reference table, has one revision.

## How to fix

Ask for an index below `revision_count()`.

## Example

```rust
let count = doc.revision_count()?; // 2
doc.open_revision(count, OpenOptions::default())?; // revision-index-out-of-range
```

## Data fields

- `index`: the index asked for.
- `count`: how many revisions the file has.
