---
title: "unset-object"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/unset-object/"
---

# unset-object

- Type: error
- Kind: `internal` (a bug in Lamina PDF or a broken invariant)

## What happened

A writer object number was reserved but never given a value.

## Why

A bug in Lamina PDF.

## How to fix

Report it at https://github.com/laudebugs/laminapdf/issues with the steps (and, if you can share it, the file) that led to it.

## Example

Raised by a writer when an object number reserved during a save never received its object. No input should cause it.

## Data fields

None.
