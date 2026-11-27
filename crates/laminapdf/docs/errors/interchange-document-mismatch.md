---
title: "interchange-document-mismatch"
version: "0.8.0"
url: "https://docs.laminapdf.com/reference/codes/interchange-document-mismatch/"
---

# interchange-document-mismatch

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An XFDF or FDF file was made for another PDF, so its comments may land in the wrong places.

## Why

XFDF and FDF files record the PDF they came from: its permanent identifier (`/ID`) and its file name. Lamina PDF compares the identifier the XFDF or FDF file records with the document's. When either has none, it compares the file names, if you passed the document's name as `file`. A difference means the comments were probably made on another document, or on another version of this one.

## How to fix

Check that the file belongs to this document before importing it. The import still runs if you go ahead: positions are copied as they are.

## Example

Importing `review-q2.xfdf`, made for `Q2-board-report.pdf`, into `Q3-board-report.pdf`.

## Data fields

- `file`: the PDF the exchange file names, when it names one.
