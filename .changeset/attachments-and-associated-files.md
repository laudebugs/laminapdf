---
"@laminapdf/sdk": minor
"@laminapdf/viewer": minor
---

Attached files: `attachments()` lists every file a document carries with what holds it (its list of attachments, the document, a page or an annotation it is associated with, a paperclip annotation), `attachmentBytes(id)` gives a file's bytes with its checksum checked, and `collection()` reads a portfolio's schema and files. `attachFile`, `updateAttachment` and `deleteAttachment` edit them, with `attachFileOp` for a transaction. Attaching to a file that claims PDF/A-1 or PDF/A-2 warns `pdfa-attachment-forbidden` in `plan()`, and on PDF/A-3 `pdfa-attachment-incomplete` until the file has an association, a modification date and a MIME type. A file with usage rights holds each attachment edit to its `/EF` right. `addBlob` now works in every module.

The builder's attachments now record an MD5 `/CheckSum` too.

In Rust: `AttachFile`, `UpdateAttachment`, `DeleteAttachment`, `AttachTarget`, `Document::attachments`, `attachment_bytes`, `collection` and `attach_file`; `AfRelationship` is now `laminapdf_model`'s, with three more relationships.

The viewer's worker gains `attachments`, `attachmentBytes`, `collection` and `addBlob`, and replays attached bytes after a restart.
