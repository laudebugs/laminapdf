# `invalid-date`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An Info date is not a valid PDF date (ISO 32000-2 §7.9.4); the raw string is kept.

## Why

Writers that write free-form dates.

## How to fix

The `Info` view keeps the raw string; read it there if you need it.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

`key` (which entry).
