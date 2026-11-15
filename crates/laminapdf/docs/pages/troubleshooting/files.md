---
title: "Troubleshooting opening and saving files"
description: "Fixes for files that need a password, signed files that won't save, and files over a limit, from JavaScript, Rust or the command line."
version: "0.4.0"
url: "https://docs.laminapdf.com/troubleshooting/files/"
---

# Troubleshooting opening and saving files

Fixes for files that need a password, signed files that won't save, and files over a limit, from JavaScript, Rust or the command line.

These entries apply in every language and on the command line, because they come from the file itself. Each one starts from what you see, then gives the cause and the fix.

## Opening a file fails with `password-required` or `wrong-password`

**Cause:** the file is encrypted. `password-required` means you passed no password, and `wrong-password` means the one you passed matches neither of the file's passwords.

**Fix:** ask your user for the password and open the file again with it. Nothing else has changed. On the command line, pass it with `--password-file` or `LAMINA_PDF_PASSWORD`. See [Open password-protected files](../../pages/guides/password-protected-files.md).

## A signed file won't save

**Cause:** saving would break a signature, so the save fails with `would-invalidate-signatures`. A signed file saves only incrementally, and only with the edits its signatures allow.

**Fix:** keep the default save mode and make only edits the signatures allow. To save anyway, ask your user first, then pass `invalidateSignatures: true` (`SaveOptions::with_invalidate_signatures(true)` in Rust). See [Save a PDF](../../pages/guides/save-a-pdf.md) and [`would-invalidate-signatures`](../../errors/would-invalidate-signatures.md).

## Opening or reading a file fails with `limit-exceeded`

**Cause:** the file asked for more than a limit allows: a stream that decompresses past its cap, for example, or objects nested deeper than the nesting limit. Lamina PDF reads every file within limits, so a hostile file can't exhaust your memory or your time.

**Fix:** the error's `data.limit` names the limit as Rust spells it, such as `max_stream_decoded`. If you trust the file, raise that one limit in the open options (`maxStreamDecoded` in TypeScript), or on the command line with its flag (`--max-stream-decoded N`). Leave the limits alone for files you don't trust. See [Open files you don't trust](../../pages/guides/untrusted-files.md) and [`limit-exceeded`](../../errors/limit-exceeded.md).

Next: [extracting text](../../pages/troubleshooting/extraction.md), for pages whose text comes out empty or in the wrong order.
