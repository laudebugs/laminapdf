# `not-a-link`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A link op named an annotation that isn't a link, such as a comment or a highlight.

## Why

The link ops change what only a link has: where it goes, and the text it covers. A link op on another kind of annotation would write entries that annotation doesn't use.

## How to fix

Pass the ID of a link that `pageLinks` lists. Change or delete other annotations with `updateAnnot` and `deleteAnnot`.

## Example

```json
{ "op": "deleteLink", "args": { "link": "a16.0" } }
```

Object 16 is a highlight, not a link.

## Data fields

None. The `location` is the annotation's `object` and `id`.

## For contributors

The decisions and todos behind this code:

- [E3c](../todos/done/E3c-links.md): link annotations
