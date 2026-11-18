---
title: "content-object-stale"
version: "0.6.0"
url: "https://docs.laminapdf.com/reference/codes/content-object-stale/"
---

# content-object-stale

- Type: error
- Kind: `state` (the call conflicts with the object's current state)

## What happened

An edit names an object on a page, such as a paragraph, an image or a drawing, but the page's content has changed since the name was taken. Lamina PDF refused the edit and changed nothing.

## Why

An edit names a page's object by its place among the page's objects and a hash of the bytes that draw it. When Lamina PDF applies the edit, it finds the object at that place and checks the hash. A different hash means another edit, or another program, has changed the page since the name was taken. Lamina PDF won't guess which object the edit meant, so it refuses rather than change the wrong one.

## How to fix

Read the page's objects again and take the object's name from that list, then apply the edit. When you replay a saved list of edits, apply them to the same file they were made on, in the order they were made.

## Example

```rust
use laminapdf_content::{ContentOps, ContentOwner};
use laminapdf_cos::Limits;

let owner = ContentOwner::Page("p3.0".parse()?);
let before = ContentOps::from_parts(owner.clone(), vec![(None, b"BT (Draft) Tj ET".to_vec())], &Limits::default())?;
let name = before.reference(before.objects()[0].id()).expect("an object");
// The text changed since the name was taken.
let after = ContentOps::from_parts(owner, vec![(None, b"BT (Final) Tj ET".to_vec())], &Limits::default())?;
assert_eq!(after.find(&name), None); // an edit with this name fails with content-object-stale
```

## Data fields

- `ordinal`: the object's place among the page's objects, from 0.
