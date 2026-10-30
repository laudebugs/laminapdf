# `nested-transaction`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

A transaction was started, or undo/redo called, while one is open (ADR 0006 §2).

## Why

Calling code opened a second transaction.

## How to fix

Commit or roll back first, or use a savepoint.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

None.
