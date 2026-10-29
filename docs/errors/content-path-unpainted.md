# `content-path-unpainted`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A content stream ended with a path that no painting operator ended; it was dropped (ADR 0023).

## Why

A producer bug, or truncated content.

## How to fix

Nothing to do.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `offset`: the byte offset of the path's first segment in the decoded content.

The `location` is `object` (the page for page content, the Form XObject for a form) with `streamOffset` in its decoded content; none for content interpreted on its own or a direct page.
