---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

You can now protect a PDF with a password and limits. `doc.setEncryption({ user, owner, permissions })` (`SetEncryption` in Rust) encrypts the file with AES-256 when you next save it. The `user` password opens it, the `owner` password lifts the limits, and `permissions` sets what a reader may do: print (`"none"`, `"lowResolution"`, `"highResolution"`), `copy`, `fillForms`, `comment` and `assemble`. Screen readers stay allowed, and editing the contents is allowed only when there is no permissions password. Leave `owner` empty and the save makes one and returns it in `report.generatedOwnerPassword`.

An encrypted file now keeps its protection when you edit and save it: an incremental save encrypts what it appends under the key the file opened with, and a full rewrite encrypts everything again. Saves draw their random bytes from `crypto.getRandomValues`; pass `entropySeed` (64 hexadecimal characters) for a repeatable file in a test. `doc.encryptionInfo()` says how a file is protected (handler, revision, ciphers, key length, permissions and which password opened it), and `doc.hasPendingEncryption()` says whether a `setEncryption` waits for the next save. Passwords never reach an op log, an error or a file: a log holds them as `null`, and replaying it fails with `secret-missing`.

Five new codes say what went wrong: `encryption-no-password`, `encryption-pdfa-conflict` (PDF/A forbids encryption), `secret-missing`, `encryption-no-entropy` and `encryption-incomplete`, which stops a save that could not encrypt every string and stream. The editor refuses its comment tools, with the reason, on a file whose permissions forbid annotating.
