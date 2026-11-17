# `unknown-attachment`

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

## For contributors

The decisions and todos behind this code:

- [E3f](../todos/done/E3f-attachments-and-associated-files.md): the attachment ops
- [E3 plan §2.1](../plans/editor/e3-navigation-attachments-inspection.md#21-ops): the ops and their payloads
