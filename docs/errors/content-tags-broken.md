# `content-tags-broken`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

The file claims PDF/UA (part 1 or 2), and a page an edit changed draws content its tags don't cover. The content may be marked content the structure tree doesn't map to an element, content outside any marked content, or a tagged form or image the edit took off the page while the structure tree still names it. A screen reader no longer reads that content in its place.

Usually the file's tags were incomplete before the edit, which is why this is an input warning: Lamina PDF checks every page an edit changes. Saving the file as it is writes one that doesn't meet PDF/UA.

## Why

Lamina PDF keeps the tags up to date as it edits: content an edit deletes leaves the structure tree, and content an edit moves or rewrites keeps its place in it. It can only do so through the structure tree's parent tree, whose entry for the page maps each piece of tagged content to its element. When that entry is missing or doesn't list the content, often because the file's tags were already incomplete, the edit can't keep the tags whole, and PDF/UA asks that all real content be tagged (ISO 14289-1 §7.1).

## How to fix

Undo the edit, or save without the PDF/UA claim: apply `removeConformanceClaims` with `pdfua` set, then tag the page again with an accessibility tool. The warning never blocks the edit or the save.

## Example

Deleting a paragraph from a page whose parent-tree entry has no slot for the paragraph or its figure, in a file whose XMP metadata has `pdfuaid:part` 1, then `doc.plan()`, lists `content-tags-broken`.

## Data fields

- `pages`: the IDs of the pages, in page order.

## For contributors

The decisions and todos behind this code:

- [E7i](../todos/done/E7i-structure-and-conformance-maintenance.md): tags kept through content edits
- [ADR 0022 §4](../decisions/0022-content-stream-editing.md#4-tagging-survives-an-edit): tagging survives an edit
