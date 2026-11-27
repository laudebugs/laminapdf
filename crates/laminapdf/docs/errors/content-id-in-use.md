---
title: "content-id-in-use"
version: "0.8.0"
url: "https://docs.laminapdf.com/reference/codes/content-id-in-use/"
---

# content-id-in-use

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An op adds an image, a shape or a page of another PDF to a page, and gives it an ID that another object Lamina PDF added to that page already has.

## Why

Each object Lamina PDF adds to a page carries its ID in the file, and later ops move, change or delete it by that ID. Two objects with one ID on a page would make those ops ambiguous, so the second is refused.

## How to fix

Give the new object an ID no other added object on the page has. To change the object that has the ID, update it with `content.update` (the op `updateContentObject`) instead of adding another.

## Example

```json
{ "op": "addContentObject", "args": { "id": "logo", "page": "p3.0", "object": { "image": { "blob": "…" } }, "matrix": [144, 0, 0, 44, 72, 72], "modified": "D:20261118120000Z" } }
```

An earlier op added an image with the ID `logo` to page `p3.0`, so this second one is refused.

## Data fields

None.
