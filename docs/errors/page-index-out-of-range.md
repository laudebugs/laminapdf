# `page-index-out-of-range`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A link (`LinkTarget::Page`), an outline item or a named destination (`Builder::destination`) names a page index the document doesn't have when the build finishes.

## Why

Page targets are resolved when `Builder::finish` (or `Appender::finish`) runs, so a link can name a page added after its own; by then every index must be one of the document's pages. Indexes are 0-based ([ADR 0027](../decisions/0027-coordinates-units-and-page-indexing.md) §6).

## How to fix

Use an index below the document's page count, or add the page before finishing.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `index`: the 0-based index given.
- `count`: the number of pages.
