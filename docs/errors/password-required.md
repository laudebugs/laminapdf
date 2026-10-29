# `password-required`

- Type: error
- Kind: `security` (encryption, permissions or signatures block the call, or look tampered with)

## What happened

The document is encrypted with a non-empty user password and none was given.

## Why

Encrypted input.

## How to fix

Pass the password in `OpenOptions`.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

None.
