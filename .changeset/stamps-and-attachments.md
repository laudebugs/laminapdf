---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Stamp a page, attach a file to a spot on it, and play sound comments. `doc.stampAt(page, area, stamp)` gives the edit that adds a stamp: one of sixteen standard names such as `approved` or `confidential`, drawn with no font, a JPEG or PNG added with `addBlob`, or dynamic lines such as "APPROVED by Ada Lovelace" set in Helvetica. `doc.attachFileAt(page, at, name, bytes, mime)` gives the edit that attaches a file under an icon; the document's attachments list it and `attachmentBytes` gives its bytes back. Lamina PDF draws every stamp and icon itself, so pdf.js, PDFium and Acrobat show the same picture.

`pageAnnotations` now reports each stamp's name, each attachment's file and each sound comment's format and length as `details`, and `doc.soundAsWav(id)` returns a sound comment's recording as a WAV file that any browser plays. An image stamp's bytes that aren't a JPEG or PNG fail with the new code `stamp-image-invalid`, and a sound that can't be played with `sound-unavailable`.
