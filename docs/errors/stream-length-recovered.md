# `stream-length-recovered`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A stream's `/Length` was wrong or unresolvable; its end was found by scanning for `endstream`.

## Why

Writers that edit streams without updating `/Length`.

## How to fix

Nothing; re-saving writes the correct length.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

None.
