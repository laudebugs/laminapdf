# `inline-image-unterminated`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

No `EI` ends an inline image (neither at its declared or computed length nor by the heuristic of ADR 0023); the rest of the content was skipped.

## Why

Damaged or truncated content.

## How to fix

Check the page: everything after the inline image is missing.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

- `offset`: the `BI`'s byte offset in the decoded content.

The `location` is `object` (the page for page content, the Form XObject for a form) with `streamOffset` in its decoded content; none for content interpreted on its own or a direct page.
