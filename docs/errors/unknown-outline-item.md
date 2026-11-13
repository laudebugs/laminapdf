# `unknown-outline-item`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An outline op names a bookmark by an ID that the document's outline doesn't hold.

## Why

The ID came from another document, or from an older read of this one: a bookmark that an earlier op deleted is gone, and the IDs a file's bookmarks have depend on that file.

## How to fix

Read the outline again and take the item's ID from it. A bookmark an op adds reports its ID in the edit's result.

## Example

```rust
doc.apply(vec![Op::DeleteOutlineItem(DeleteOutlineItem::new("o999.0".parse()?, ChildItems::Delete))])?; // unknown-outline-item
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E3b](../todos/done/E3b-bookmarks-and-destinations.md): the bookmark editor and named destinations
