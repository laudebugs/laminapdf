# `limit-exceeded`

- Type: error
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

A hard `Limits` value was exceeded.

## Why

Hostile or unusually large input (ADR 0003 §3).

## How to fix

Raise the named limit only if you trust the input.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

`limit` (the limit's name).
