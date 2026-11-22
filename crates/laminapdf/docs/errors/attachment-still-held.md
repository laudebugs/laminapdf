---
title: "attachment-still-held"
version: "0.7.0"
url: "https://docs.laminapdf.com/reference/codes/attachment-still-held/"
---

# attachment-still-held

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

An edit deleted an attachment from the document's list and from every page and annotation it was associated with, but a file attachment annotation (the paperclip icon on a page) holds the same file. The file stays in the document while the annotation does.

## Why

A file attachment annotation is part of the page, so deleting it is an annotation edit, which an attachment edit never makes on its own.

## How to fix

Delete the annotation too (`deleteAnnot`) to take the file out of the document. Saving incrementally keeps the file in the earlier revision either way; save an optimised copy to remove it from the file's history.

## Example

Deleting an attachment that a `FileAttachment` annotation on page 2 holds returns `attachment-still-held` in the edit's warnings.

## Data fields

- `attachments`: each deleted attachment's ID and the annotation that holds it.
