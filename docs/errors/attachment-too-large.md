# `attachment-too-large`

- Type: error
- Kind: `limit` (a limit was reached)

## What happened

A file to attach is larger than the document can read back out, so it wasn't attached. Nothing changed.

## Why

Reading an attached file's bytes is held to the document's `maxStreamDecoded` limit (64 MB by default), as every stream is. A larger file attached to the document could never be saved out of it again on this device, so Lamina PDF refuses it instead.

## How to fix

Attach a smaller file. If the device can hold more, open the document with a higher `maxStreamDecoded` limit; `attachmentLimit()` (`Document::attachment_limit` in Rust) tells you the limit in force.

## Example

Attaching a 100 MB video to a document opened with the default limits.

## Data fields

- `size`: the file's size in bytes.
- `limit`: the limit in bytes.

## For contributors

The decisions and todos behind this code:

- [E3f](../todos/done/E3f-attachments-and-associated-files.md): the attachment ops
- [E3 plan §2.1](../plans/editor/e3-navigation-attachments-inspection.md#21-ops): the ops and their payloads
