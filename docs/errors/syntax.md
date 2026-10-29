# `syntax`

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A token or structure did not match the PDF grammar.

## Why

Malformed input.

## How to fix

Use another copy of the file.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

None. The `location` is a file `offset`, or `object` with `streamOffset` inside an object stream.
