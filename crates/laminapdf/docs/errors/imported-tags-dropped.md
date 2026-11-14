---
title: "imported-tags-dropped"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/imported-tags-dropped/"
---

# imported-tags-dropped

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/UA-1 but isn't tagged, and pages copied in from another document came without the tags their source gave them. PDF/UA-1 requires every piece of real content to be tagged (ISO 14289-1 §7.1), so saving the file as it is writes one that no longer meets it.

## Why

Tags live in the document's structure tree, which ties each page's content to elements such as headings and paragraphs. A page copied into a tagged document brings its elements along. A document that isn't tagged has no tree to take them, and a tagged file must tag all its content, so Lamina PDF leaves the copy's content untagged.

## How to fix

Tag the document, and the copied pages with it, in a tagging tool, or copy pages from a document that has no tags to lose. If the file needn't stay PDF/UA-1, save anyway: the warning never blocks a save.

## Example

A file claims PDF/UA-1 in its metadata, but its catalog doesn't mark it tagged. Copying a page of a tagged report into it with `insertPagesFrom` makes `doc.plan()` list `imported-tags-dropped`, and the edit's `tagsDropped` names the copy.

## Data fields

None.
