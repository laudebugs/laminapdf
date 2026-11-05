---
id: encrypted-password
title: Open an encrypted file and handle a wrong password
checker: encrypted-password.mjs
inputs: [fixtures/encrypted/r4-aes-128-user.pdf => secret.pdf]
output: open-secret.mjs
---
`secret.pdf` is password-protected. Write a script `open-secret.mjs`, run with `node open-secret.mjs <file> [password]`, that opens the file and prints exactly one line of JSON to stdout:

- `{"status":"ok","pages":<page count>}` when the file opens;
- `{"status":"wrong-password"}` when a password was given but is wrong;
- `{"status":"password-required"}` when the file needs a password and none was given.

The script exits with code 0 in all three cases. Decide which case happened from the library's error, without reading the error's message text. The password for `secret.pdf` is `user-pw`.
