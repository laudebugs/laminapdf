# `interchange-too-large`

- Type: error
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

An exchange file is larger than the document's limits allow, so it wasn't read.

## Why

An import reads the whole file into memory, and the document's `Limits::max_stream_decoded` bounds how large it may be, as it bounds any one stream. The bound keeps a hostile file from using up the memory of a browser tab or a server.

## How to fix

Raise `Limits::max_stream_decoded` when you open the document if files this large are expected, or import the comments in parts.

## Example

Importing a 300 MB XFDF file with the default limits.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E5d](../todos/ongoing/E5d-xfdf-fdf.md): importing and exporting comments
- [E5 plan §2.3](../plans/editor/e5-review-stamps-interchange.md#23-interchange-and-the-summary)
