# `content-tags-broken`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file claims PDF/UA-1, and after an edit to a page's content, the page draws tagged content that the structure tree doesn't map to an element. A screen reader no longer reads that content in its place. Saving the file as it is writes one that no longer meets PDF/UA-1.

## Why

Lamina PDF keeps the tags up to date as it edits: content an edit deletes leaves the structure tree, and content an edit moves or rewrites keeps its place in it. It can only do so through the page's entry in the tree's parent tree, which maps each piece of tagged content to its element. When that entry is missing or doesn't list the content, often because the file's tags were already incomplete, the edit can't keep the tags whole, and PDF/UA-1 asks that all real content be tagged (ISO 14289-1 §7.1).

## How to fix

Undo the edit, or save without the PDF/UA claim: apply `removeConformanceClaims` with `pdfua` set, then tag the page again with an accessibility tool. The warning never blocks the edit or the save.

## Example

Deleting a paragraph from a page whose parent-tree entry lacks a slot for its figure, in a file whose XMP metadata has `pdfuaid:part` 1, then `doc.plan()`, lists `content-tags-broken`.

## Data fields

- `pages`: the IDs of the pages, in page order.

## For contributors

The decisions and todos behind this code:

- [E7i](../todos/ongoing/E7i-structure-and-conformance-maintenance.md): tags kept through content edits
- [ADR 0022 §4](../decisions/0022-content-stream-editing.md#4-tagging-survives-an-edit): tagging survives an edit
