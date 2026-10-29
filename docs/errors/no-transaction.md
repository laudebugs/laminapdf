# `no-transaction`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

`apply`, `commit` or `rollback` was called without an open transaction.

## Why

Calling code skipped `begin`.

## How to fix

Call `begin` first, or use `transact`.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

None.
