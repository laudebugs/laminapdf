# `unknown-page-mark`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An op changes or deletes a page mark Lamina PDF added (a watermark, a background, a header or footer, or a Bates number), and one of the pages it names draws no mark with that ID.

## Why

The mark was deleted, by an earlier op or by an undo, or the ID belongs to a mark on other pages, or to an object added with `addContentObject`, which `updateContentObject` and `deleteContentObject` change. A mark is found by the ID it was added with, so a mark another app changed is the page's own content and has no ID to find.

## How to fix

List the page's marks again with `content.pageMarks`, and take the ID and the pages from that list.

## Example

```json
{ "op": "deletePageMark", "args": { "id": "draft", "pages": ["p3.0", "p4.0"] } }
```

Page `p4.0` draws no mark with the ID `draft`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E6d](../todos/ongoing/E6d-watermarks-headers-footers.md): watermarks, backgrounds, headers, footers and Bates numbers
