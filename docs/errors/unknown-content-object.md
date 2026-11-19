# `unknown-content-object`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An op changes or deletes an object Lamina PDF added to a page, and the page draws no added object with that ID.

## Why

The object was deleted, by an earlier op or by an undo, or the ID names an object on another page. An object whose drawing another app changed after Lamina PDF added it no longer counts as added, so it can't be found by its ID either: Lamina PDF leaves such content alone.

## How to fix

Read the page's added objects again and take the ID from that list.

## Example

```json
{ "op": "deleteContentObject", "args": { "id": "logo", "page": "p3.0" } }
```

Page `p3.0` draws no added object with the ID `logo`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E6b](../todos/done/E6b-place-images-and-shapes.md): images, shapes, logos and stamps placed as page content
