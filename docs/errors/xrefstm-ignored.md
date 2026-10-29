# `xrefstm-ignored`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A hybrid file's `/XRefStm` could not be read and was ignored.

## Why

The stream is damaged or points at the wrong offset.

## How to fix

Nothing; the classic table is used.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

`offset` (where `/XRefStm` points).
