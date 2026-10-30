# `strict`

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

Strict mode refused something the tolerant reader would have repaired.

## Why

The file needs repair; see the warning code in the data.

## How to fix

Open without strict mode, or fix the file.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

`warning` (the warning code).
