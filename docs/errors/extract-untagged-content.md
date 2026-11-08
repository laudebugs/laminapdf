# `extract-untagged-content`

- Type: warning
- Kind: `input` (a problem with the PDF file)

## What happened

The page was read in its structure tree's order, but some of its visible text isn't in the tree and isn't marked as an artifact. Extraction kept that text and put it after the tree's blocks, in layout order.

## Why

In a tagged file, every piece of visible content should belong to a structure element or be marked as an artifact (ISO 32000-2 §14.8.2.2). Text that is neither has no place in the tree's order. A tool that adds text after tagging a document, such as a stamp or a page number, often leaves text like this.

## How to fix

Nothing to do in Lamina PDF. To give that text its place in the reading order, tag it in the file, or mark it as an artifact if it isn't content. Like every warning, it fails the call in strict mode.

## Example

A tagged contract with a "DRAFT" stamp drawn after tagging: the stamp's text comes after the contract's blocks.

## Data fields

- `chars`: how many characters (spaces left out) lie outside the tree.

The `location` has the page's `pageIndex`.

## For contributors

The decisions and todos behind this code:

- [ADR 0030](../decisions/0030-extraction-output-and-model-plugins.md): the extraction output and the reading-order sources
