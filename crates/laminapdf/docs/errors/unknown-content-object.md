---
title: "unknown-content-object"
version: "0.8.0"
url: "https://docs.laminapdf.com/reference/codes/unknown-content-object/"
---

# unknown-content-object

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An op changes or deletes an object Lamina PDF added to a page, and the page draws no added object with that ID.

## Why

The object was deleted, by an earlier op or by an undo, or the ID names an object on another page. If another app changed an added object's drawing, Lamina PDF leaves that content alone and no longer finds it by its ID.

## How to fix

List the page's added objects again with `content.added`, and take the ID from that list.

## Example

```json
{ "op": "deleteContentObject", "args": { "id": "logo", "page": "p3.0" } }
```

Page `p3.0` draws no added object with the ID `logo`.

## Data fields

None.
