# `encryption-incomplete`

- Type: error
- Kind: `security` (encryption, permissions or signatures block the call, or look tampered with)

## What happened

A save could not encrypt one of the file's strings or streams, so it wrote nothing.

## Why

A file encrypted in part leaks what it was meant to hide through the parts left as plain text. The PDFex research (ACM CCS 2019) showed that attack, so Lamina PDF refuses to write such a file.

## How to fix

Report the bug with the file that triggers it. The document is unchanged, and saving with the encryption removed (`withRemoveEncryption`) still works.

## Example

A save of an encrypted file whose key length doesn't fit its cipher.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E11a](../plans/editor/e11-security-and-sanitisation.md): the plan
- [E11b](../todos/ongoing/E11b-encrypt-on-write.md): encryption on save
