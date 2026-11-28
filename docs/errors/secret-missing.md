# `secret-missing`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An operation needed a password and the op log it came from had none. An op log writes each password as `null`, so a log can't replay an encrypted save.

## Why

A password in a log would reach disks, crash reports and other devices. Lamina PDF keeps passwords in memory only and never serialises them.

## How to fix

Pass the password again when you apply the operation, instead of replaying it from a log. A host that restores a session applies `setEncryption` again with the password the person types.

## Example

`{ "op": "setEncryption", "args": { "user": null } }` applied from a saved log.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E11a](../plans/editor/e11-security-and-sanitisation.md): the plan
- [E11b](../todos/done/E11b-encrypt-on-write.md): encryption on save
