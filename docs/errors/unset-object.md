# `unset-object`

- Type: error
- Kind: `internal` (a Lamina PDF bug or a broken invariant)

## What happened

A writer object number was reserved but never given a value.

## Why

A Lamina PDF bug.

## How to fix

Report it at https://github.com/laudebugs/laminapdf/issues with the steps (and, if you can share it, the file) that led to it.

## Example

Raised by a writer when an object number reserved during a save never received its object. No input should cause it.

## Data fields

None.
