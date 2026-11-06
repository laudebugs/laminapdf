# `extract-struct-tree-ignored`

- Type: warning
- Kind: `input` (a problem with the PDF file)

## What happened

The page is tagged, but its structure tree didn't pass the checks that decide whether to trust it, so extraction read the page in layout order instead.

## Why

A structure tree gives a page's reading order, but trees are often incomplete or wrong. Extraction uses a page's tree only when its elements cover at least 80% of the page's visible text, artifacts left out. Its order must also jump back up into an earlier column at most once in ten moves from block to block. A tree written by a tool that tags only some of the text, or that tags it in drawing order, fails one of the checks.

## How to fix

Nothing to do; the layout order is usually right. To use the tree anyway, extract with `ReadingOrder::Tagged` (`readingOrder: "tagged"`). Like every warning, it fails the call in strict mode.

## Example

A report whose tool tagged only its headings: the tree covers a tenth of the text, so the reason is `coverage`.

## Data fields

- `reason`: `coverage` or `order`, the check that failed.

The `location` has the page's `pageIndex`.

## For contributors

The decisions and todos behind this code:

- [ADR 0030](../decisions/0030-extraction-output-and-model-plugins.md): the extraction output and the reading-order sources
