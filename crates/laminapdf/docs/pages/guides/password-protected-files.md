---
title: "Open password-protected files"
description: "Open PDFs that need a password, ask again when it is missing or wrong, and save them afterwards."
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/guides/password-protected-files/"
---

# Open password-protected files

Open PDFs that need a password, ask again when it is missing or wrong, and save them afterwards.

You'll open a PDF that needs a password, and ask your user again when the password is missing or wrong. You need this for bank statements, contracts and other files that people protect before they share them.

This example opens an upload with an optional password and returns the outcome as a value instead of throwing. Its test opens a file encrypted with AES-256 three times: with the right password, with none and with a wrong one:

`packages/sdk/examples/open-untrusted.ts`:

```ts
// Open bytes from someone else: an optional password, tighter limits than the defaults, and the
// outcome as a value rather than an exception. The example writes nothing: it returns what it found.
import { Document, isLaminaPdfError } from "@laminapdf/sdk";

const MiB = 1024 * 1024;
/** Limits for an upload endpoint: each decoded stream, and all of them together. */
const LIMITS = { maxStreamDecoded: 16 * MiB, maxDecodedTotal: 64 * MiB } as const;

/** What opening a file gave: its page count and warning codes, or why it didn't open. */
export type Opened =
  | { readonly ok: true; readonly pages: number; readonly warnings: readonly string[] }
  | { readonly ok: false; readonly code: string; readonly hint: string | undefined };

/** Opens `bytes`, with `password` if the file needs one, under {@link LIMITS}. Needs `init()`. */
export function openUntrusted(bytes: Uint8Array, password?: string): Opened {
  try {
    using doc = Document.open(bytes, { ...(password === undefined ? {} : { password }), limits: LIMITS });
    // Repairs and dropped values are warnings: the file opened, but not exactly as written.
    return { ok: true, pages: doc.pageCount, warnings: doc.warnings.map((w) => w.code) };
  } catch (e) {
    // `password-required`, `wrong-password`, `not-pdf`, `limit-exceeded`, …: a coded error whose
    // message and hint never contain text from the file. Anything else is a bug: let it throw.
    if (isLaminaPdfError(e)) return { ok: false, code: e.code, hint: e.hint };
    throw e;
  }
}
```

## Pass the password

Pass the password when you open the file: `Document.open(bytes, { password })`. The password goes only to the WebAssembly module. Lamina PDF never stores or logs it, and never puts it in an error.

A PDF can have two passwords. The user password opens the file, and the owner password also lifts its restrictions, such as on printing or copying. Lamina PDF checks your password against both:

- **Older files** (RC4 and AES-128) check the user password first.
- **AES-256 files** check the owner password first, as PDF 2.0 requires. So a password that matches both counts as the owner's.

## When the password is missing or wrong

Opening fails with `password-required` when you pass no password, and with `wrong-password` when it matches neither. Nothing else has changed, so ask your user and open the file again.

## Passwords with accents and other symbols

A character such as "é" can be written in more than one way in Unicode, so the same password can arrive as different bytes. In TypeScript, Lamina PDF tries the password as you give it and, after `wrong-password`, again in its NFKC form, the Unicode form that makes such spellings match. Older files also accept a password's Latin-1 bytes. AES-256 files accept the password as given or in the normalised form that PDF 2.0 asks for (SASLprep).

## Save an encrypted file

Once you've edited an encrypted file, it saves only as an unencrypted [full rewrite](../../pages/reference/glossary.md#full-rewrite), with `removeEncryption: true`. Without that option, saving fails with `encrypted-base` (see [save a PDF](../../pages/guides/save-a-pdf.md#when-a-file-refuses-a-save)). Lamina PDF can't yet write encrypted files.

> **Details:** Lamina PDF reads files encrypted with the standard security handler: RC4 (revisions 2–4), AES-128 (revision 4) and AES-256 (revisions 5 and 6), including crypt filters and unencrypted metadata. It decrypts strings and streams as the file loads.

Next: [open files you don't trust](../../pages/guides/untrusted-files.md).
