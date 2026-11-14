---
title: "imported-tags-dropped"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/imported-tags-dropped/"
---

# imported-tags-dropped

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/UA-1, and pages copied in from another document came without the tags their source gave them. PDF/UA-1 requires every piece of real content to be tagged (ISO 14289-1 §7.1), so saving the file as it is writes one that no longer meets it.

## Why

Tags live in the document's structure tree, which ties each page's content to elements such as headings and paragraphs. A page copied from another document brings its content but not the elements of the other document's tree. Lamina PDF therefore leaves the copy's content untagged rather than tied to elements the document lacks.

## How to fix

Tag the copied pages in a tagging tool, or copy pages from a document that has no tags to lose. If the file needn't stay PDF/UA-1, save anyway: the warning never blocks a save.

## Example

Copying a page of a tagged report into a PDF/UA-1 file with `insertPagesFrom`, then `doc.plan()`, lists `imported-tags-dropped`. The edit's `tagsDropped` names the copy.

## Data fields

None.
