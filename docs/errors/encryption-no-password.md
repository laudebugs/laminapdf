# `encryption-no-password`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A `setEncryption` operation had an empty user password and an empty owner password.

## Why

A file with neither password opens for anyone, and nobody can change its limits, so there would be nothing to protect. The check runs when the operation is applied, before anything is saved.

## How to fix

Give a user password, which readers type to open the file, an owner password, which lifts the file's limits, or both. With only a user password, the file's permissions are the ones the operation sets.

## Example

`setEncryption` with `user: ""` and `owner: ""`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E11a](../plans/editor/e11-security-and-sanitisation.md): the plan
- [E11b](../todos/ongoing/E11b-encrypt-on-write.md): encryption on save
