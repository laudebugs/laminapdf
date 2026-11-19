# `interchange-malformed`

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An XFDF or FDF file didn't read, so nothing was imported. The message names the line where reading stopped.

## Why

An XFDF file must be well-formed XML with an `xfdf` element, and its numbers, colours and file data must parse. An FDF file must have a trailer whose `/Root` holds an `/FDF` dictionary. A file cut short, edited by hand or written by a broken exporter can fail either check.

## How to fix

Open the file in a text editor at the line the message names, or export the comments again from the app that made the file.

## Example

An XFDF file whose `<contents>` element is closed with `</content>`: the message says `near line 8`.

## Data fields

- `format`: `XFDF` or `FDF`.
- `line`: where reading stopped, counted from 1.

## For contributors

The decisions and todos behind this code:

- [E5d](../todos/ongoing/E5d-xfdf-fdf.md): importing and exporting comments
- [E5 plan §2.3](../plans/editor/e5-review-stamps-interchange.md#23-interchange-and-the-summary)
