# `unset-object`

- Type: error
- Kind: `internal` (a lamina bug or a broken invariant)

## What happened

A writer object number was reserved but never given a value.

## Why

A lamina bug.

## How to fix

Report it at https://github.com/laudebugs/lamina/issues with the steps (and, if you can share it, the file) that led to it.

## Example

Raised by a writer when an object number reserved during a save never received its object. No input should cause it.

## Data fields

None.
