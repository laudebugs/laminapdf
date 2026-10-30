# `not-rebased`

- Type: error
- Kind: `state` (the call isn't valid in the current state; it may succeed after a state change)

## What happened

The save wrote its complete output, but rebasing the document onto it failed. The document is `SavedNotRebased`.

## Why

Reading the output back failed (for example a limit, or a source that can't deliver the bytes yet).

## How to fix

Retry `rebase` with the output the error carries (`saved.source()`; in the `lamina` façade, `Error::saved_output()` and `Document::rebase`).

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

- `error`: the rebase's own error (its code, kind and location).
