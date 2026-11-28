# `encryption-no-entropy`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A save that encrypts the file was given no random bytes. Nothing was written.

## Why

Encryption needs a random file key, salts and a fresh initialisation vector for every string and stream. Lamina PDF's core reads no clock and no operating-system source, so the host passes the bytes.

## How to fix

Pass an entropy seed of 32 random bytes with the save: `crypto.getRandomValues` in a browser or worker, `getrandom` in native code. The SDK does this for you. A fixed seed gives a repeatable file and weak keys, so use one only in tests.

## Example

`doc.save(&mut out, &SaveOptions::new())` on an encrypted document with changes, without `with_entropy_seed`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E11a](../plans/editor/e11-security-and-sanitisation.md): the plan
- [E11b](../todos/ongoing/E11b-encrypt-on-write.md): encryption on save
