# `duplicate-tags-incomplete`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/UA-1, and a duplicated page's copy is partly untagged. PDF/UA-1 requires every piece of real content to be tagged (ISO 14289-1 §7.1), so saving the file as it is writes one that no longer meets it.

## Why

A copy shares its page's content, so Lamina PDF tags it with copies of the page's structure elements. An element that also holds content on another page, such as a paragraph that runs over a page break, can't be copied for one page alone. The copy's share of that content is left untagged rather than tied to the original's element.

## How to fix

Tag the copy's untagged content in a tagging tool, or duplicate only pages whose elements end on the page. If the file needn't stay PDF/UA-1, save anyway: the warning never blocks a save.

## Example

Duplicating a page whose last paragraph continues on the next page, in a PDF/UA-1 file, then `doc.plan()`, lists `duplicate-tags-incomplete`. The edit's `tagsIncomplete` names the copy.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E2k](../todos/ongoing/E2k-duplicate-pages.md): duplicating pages
- [E2 plan §3](../plans/editor/e2-page-organisation.md#3-conformance-the-edits-must-keep): what page edits do to a PDF/UA claim
