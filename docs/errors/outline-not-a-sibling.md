# `outline-not-a-sibling`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

The op places a bookmark before or after another one, but that other bookmark isn't a child of the parent the op names.

## Why

A position is relative to the parent's children: `before` and `after` name one of them. A bookmark elsewhere in the outline can't mark a place under this parent.

## How to fix

Name a child of the same parent in `before` or `after`, or use `first` or `last`.

## Example

```rust
// "o14.0" is a child of "o12.0", not of the top level.
let op = MoveOutlineItem::new(intro, None, SiblingPosition::After("o14.0".parse()?));
doc.apply(vec![Op::from(op)])?; // outline-not-a-sibling
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E3b](../todos/done/E3b-bookmarks-and-destinations.md): the bookmark editor and named destinations
