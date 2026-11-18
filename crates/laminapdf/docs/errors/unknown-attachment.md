---
title: "unknown-attachment"
version: "0.6.0"
url: "https://docs.laminapdf.com/reference/codes/unknown-attachment/"
---

# unknown-attachment

- Type: error
- Kind: `usage` (the call or its arguments are wrong)

## What happened

An edit names an attachment by an ID that the document doesn't hold. Nothing changed.

## Why

An attachment's ID names its file specification in this document. The ID stops working when the attachment is deleted, and an ID read from another document never works here.

## How to fix

Read the attachments again (`Document::attachments` in Rust, `doc.attachments()` in TypeScript) and take the ID from that list. After an undo or a redo, read the list again before the next edit.

## Example

```json
{ "op": "deleteAttachment", "args": { "attachment": "e9.0" } }
```

Applied to a document with no attachment `e9.0`.

## Data fields

None.
