# `no-pending-save`

- Type: error
- Kind: `state` (the call isn't valid in the current state; it may succeed after a state change)

## What happened

`rebase` was called, but no save is waiting for it.

## Why

Rebasing onto other bytes would not match the undo history, and unsaved changes would be lost.

## How to fix

Call `rebase` only after a save that left the document `SavedNotRebased`.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

None.
