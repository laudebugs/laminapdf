# `pdfa-optional-content`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/A-1, and a watermark or background the edits added is in an optional content group, so a reader's layers panel can switch it off. Saving the file as it is writes one that no longer meets PDF/A-1.

## Why

PDF/A-1 forbids optional content (ISO 19005-1 §6.1.13): a page may not name a layer, and the catalog may not list one. PDF/A-2 and later allow it, with a default configuration that names itself and every group on.

## How to fix

Add the mark without `hideable`, or update it to a mark that isn't. If the file needn't stay PDF/A-1, save anyway or remove the claim with `removeConformanceClaims`: the warning never blocks a save.

## Example

A watermark added with `hideable: true` to a PDF/A-1b file, then `doc.plan()`, lists `pdfa-optional-content` with the pages that have it.

## Data fields

- `pages`: the IDs of the pages with a mark in an optional content group, in page order.

## For contributors

The decisions and todos behind this code:

- [E6d](../todos/done/E6d-watermarks-headers-footers.md): watermarks, backgrounds, headers, footers and Bates numbers
