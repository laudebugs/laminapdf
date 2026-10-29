# `invalid-text-range`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A byte range of a text given to `Shaper::shape_range`, `BidiParagraph::visual_runs` or `BidiParagraph::line_levels`, or the range of a `ShapedRun` given to `pdf_run`, is past the end of the text or does not start and end on character boundaries.

## Why

Ranges are byte offsets into UTF-8 text; a range that cuts a character in two (or reaches past the end) names no text.

## How to fix

Take ranges from lamina's own results (`line_breaks`, `itemize`, `break_lines`, `ShapedRun::text_range`), or check them with `str::get` first, and pass the same text the range was computed on.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `start`, `end`: the range (bytes).
- `len`: the text's length (bytes).
