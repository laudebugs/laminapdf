# `invalid-page-labels`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

The ranges given to `Builder::page_labels` can't be written as a `/PageLabels` number tree.

## Why

ISO 32000-2 §7.7.2 and §12.4.2: the tree must have an entry for page index 0, its keys are page indexes in increasing order, and a label range's first number (`/St`) is at least 1. A range starting past the document's last page would label nothing.

## How to fix

Give the ranges sorted by first page, the first starting at page 0, each start a page of the document, and numbers from 1 (`PageLabel::with_first`).

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `what`: what is wrong (fixed text).
