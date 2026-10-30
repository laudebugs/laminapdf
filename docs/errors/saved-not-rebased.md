# `saved-not-rebased`

- Type: error
- Kind: `state` (the call isn't valid in the current state; it may succeed after a state change)

## What happened

The document was saved to a sink lamina can't read back, and it waits for `rebase`: ops, undo, redo and saves are refused until then (ADR 0005 §3).

## Why

`save_full` to a write-only sink leaves the document `SavedNotRebased`: the bytes it would continue from are not available to it.

## How to fix

Call `rebase` with the saved bytes, then retry.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

None.
