---
title: "outline-move-into-itself"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/outline-move-into-itself/"
---

# outline-move-into-itself

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A move names the bookmark itself, or one of the bookmarks under it, as its new parent or as the place to put it next to.

## Why

A bookmark goes with its children when it moves, so it can't move into its own branch: the outline would become a loop.

## How to fix

Choose a parent outside the bookmark's branch. To move children up, move them one by one, or delete the bookmark with `children: promote`.

## Example

```rust
// "o14.0" is one of "o12.0"'s children.
let op = MoveOutlineItem::new("o12.0".parse()?, Some("o14.0".parse()?), SiblingPosition::Last);
doc.apply(vec![Op::from(op)])?; // outline-move-into-itself
```

## Data fields

None.
