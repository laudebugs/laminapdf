# `unsupported-version`

- Type: error
- Kind: `unsupported` (valid input that lamina doesn't implement)

## What happened

An op log has a schema version this reader doesn't know (ADR 0006 §6).

## Why

A newer lamina wrote it.

## How to fix

Upgrade lamina.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

`version`.
