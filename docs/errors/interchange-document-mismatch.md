# `interchange-document-mismatch`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An exchange file was made for another PDF, so its comments may land in the wrong places.

## Why

XFDF and FDF files record the PDF they came from: its permanent identifier (`/ID`) and its file name. When both files have an identifier and they differ, or, without identifiers, the file names differ, the comments were probably made on another document, or on another version of this one.

## How to fix

Check that the file belongs to this document before importing it. The import still runs if you go ahead: positions are copied as they are.

## Example

Importing `review-q2.xfdf`, made for `Q2-board-report.pdf`, into `Q3-board-report.pdf`.

## Data fields

- `file`: the PDF the exchange file names, when it names one.

## For contributors

The decisions and todos behind this code:

- [E5d](../todos/ongoing/E5d-xfdf-fdf.md): importing and exporting comments
- [E5 plan §2.3](../plans/editor/e5-review-stamps-interchange.md#23-interchange-and-the-summary)
