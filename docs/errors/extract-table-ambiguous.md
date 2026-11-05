# `extract-table-ambiguous`

- Type: warning
- Kind: `unsupported` (valid input that Lamina PDF doesn't implement)

## What happened

Extraction found a table drawn with lines, but the cells its missing lines would join don't make a rectangle. Extraction kept the table and gave it one cell per square of its grid, so a merged cell's text sits in the first of the squares it covers.

## Why

A table's lines tell where cells are merged: a missing line between two squares joins them. Joined squares that form an L or a T shape don't describe a cell, usually because a line is drawn slightly short, drawn twice in different places, or covered by a white rectangle.

## How to fix

Nothing to do in Lamina PDF; the file is valid. Like every warning, it fails the call in strict mode. Check that table's cells: the text of a merged cell may be split across its neighbours.

## Example

A table whose header spans two columns over the first row only, but whose line under that header stops halfway.

## Data fields

None.

The `location` has the page's `pageIndex`.

## For contributors

The decisions and todos behind this code:

- [L3b](../todos/done/L3b-extract-tables-and-serialisers.md): tables and extraction output
