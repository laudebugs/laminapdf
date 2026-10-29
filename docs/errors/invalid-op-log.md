# `invalid-op-log`

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An op log does not parse: an unknown op or field, a bad value, or not JSON.

## Why

Hand-written or corrupted op logs.

## How to fix

Validate the log against the op schema.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

`detail` (**untrusted**: may quote the log).
