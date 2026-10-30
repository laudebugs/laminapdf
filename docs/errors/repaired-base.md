# `repaired-base`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

An incremental update was requested for a document whose xref needed repair.

## Why

Appending to a repaired file would build on offsets that are wrong.

## How to fix

Save with a full rewrite instead.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

None.
