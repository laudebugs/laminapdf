# `content-unknown-operator`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A content stream used an operator ISO 32000-2 Annex A does not define, outside a `BX`/`EX` compatibility section; it was skipped (ADR 0023).

## Why

A producer wrote a private or misspelled operator.

## How to fix

Nothing to do; check the page's rendering if the operator mattered.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `offset`: the operator's byte offset in the decoded content.

The `location` is `object` (the page for page content, the Form XObject for a form) with `streamOffset` in its decoded content; none for content interpreted on its own or a direct page.
