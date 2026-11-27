---
title: "interchange-skipped"
version: "0.8.0"
url: "https://docs.laminapdf.com/reference/codes/interchange-skipped/"
---

# interchange-skipped

- Type: warning
- Kind: `unsupported` (valid input that Lamina PDF does not support yet)

## What happened

An import left out a comment or a field it can't make. Everything else in the file was imported.

## Why

An import makes the comment kinds Lamina PDF draws: notes, text markup, carets, ink, shapes, lines, polygons, text boxes, stamps and file attachments, with their replies and statuses. It leaves out sounds, links, redactions, movies and kinds another app invented, a stamp drawn from a picture another app made, and form fields, whose values are form data rather than comments. It also leaves out a comment for a page the document doesn't have, and a reply whose comment is neither in the file nor in the document.

## How to fix

Nothing, if the rest is enough. Import a form's values with the form tools, and check the page count when comments are skipped for missing pages.

## Example

An XFDF file with a sound comment on page 12: the sound is skipped, the other comments are imported.

## Data fields

- `kind`: the annotation's subtype (`Sound`) or `field`.
- `reason`: `unsupported`, `form-field`, `missing-page`, `missing-parent` or `incomplete`.
- `page` and `line`: where it is, when known.
