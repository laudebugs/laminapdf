# `encryption-pdfa-conflict`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state; it may succeed after a change)

## What happened

A `setEncryption` operation named a file whose metadata claims PDF/A.

## Why

The PDF/A standard forbids encryption, so an encrypted file can't keep the claim. Lamina PDF won't write a file that claims PDF/A and breaks it.

## How to fix

Remove the PDF/A claim first (the `removeConformanceClaims` operation), then protect the file, or leave the file unprotected.

## Example

Protecting `pdfa-form.pdf` with a password.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E11a](../plans/editor/e11-security-and-sanitisation.md): the plan
- [E11b](../todos/done/E11b-encrypt-on-write.md): encryption on save
