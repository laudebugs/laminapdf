# `content-syntax`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A content stream held bytes that are not a token (an unbalanced `)` or `>`, a bad hex digit, an unterminated string or array) or a stray `ID`/`EI`; they were skipped with the operands read so far (ADR 0023).

## Why

Damaged or truncated content.

## How to fix

Check the page's content around the reported offset.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

- `offset`: the problem's byte offset in the decoded content.

The `location` is `object` (the page for page content, the Form XObject for a form) with `streamOffset` in its decoded content; none for content interpreted on its own or a direct page.
