# `direct-outline-item`

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF doesn't implement)

## What happened

The bookmark is stored as a dictionary inside another bookmark, not as an object of its own, so the outline ops can't change it.

## Why

PDF files usually store each bookmark as its own object, with a number. Bookmarks find their neighbours and parent by those numbers, and a bookmark stored inside another has none, so its ID is made of its path instead.

## How to fix

Apply another outline op first, such as moving any bookmark to where it already is. It rewrites the outline, gives every bookmark its own object and new ID, and reports that it repaired the outline. Then read the outline again for the new IDs.

## Example

```rust
// The outline holds a bookmark as a dictionary inside its parent, so its ID is a path ID.
let item: OutlineItemId = "o12.0/First".parse()?;
doc.apply(vec![Op::from(DeleteOutlineItem::new(item, ChildItems::Delete))])?; // direct-outline-item
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E3b](../todos/done/E3b-bookmarks-and-destinations.md): the bookmark editor and named destinations
