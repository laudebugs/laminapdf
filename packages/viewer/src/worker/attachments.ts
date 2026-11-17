// The files the open document carries (O14): listed with what holds each, their bytes for saving
// out, a portfolio's schema, and the bytes an `attachFile` edit names, kept in the session. Each
// read goes to the model, which holds the edits, after the calls queued before it.

import type { AttachmentFile, Collection, DocumentAttachment } from "@laminapdf/sdk";
import { current, followEdit } from "./document.js";
import { serial } from "./edit.js";

/** The files document `id` carries, each with what holds it, as its edits leave them. */
export function readAttachments(id: number, signal: AbortSignal): Promise<DocumentAttachment[]> {
  const opened = current(id);
  return serial(opened, () => opened.model.ensure(() => opened.model.attachments(), { signal }));
}

/** The bytes of attachment `attachment` of document `id`, and how they compare with its checksum. */
export function readAttachmentBytes(attachment: string, id: number, signal: AbortSignal): Promise<AttachmentFile> {
  const opened = current(id);
  return serial(opened, () => opened.model.ensure(() => opened.model.attachmentBytes(attachment), { signal }));
}

/** The largest file, in bytes, document `id` can read back out and so attach. */
export function readAttachmentLimit(id: number): number {
  return current(id).model.attachmentLimit();
}

/** The portfolio document `id` is, or `undefined`. */
export function readCollection(id: number, signal: AbortSignal): Promise<Collection | undefined> {
  const opened = current(id);
  return serial(opened, () => opened.model.ensure(() => opened.model.collection(), { signal }));
}

/**
 * Keeps `bytes` in document `id`'s session, and in its text-capable copy: the copy applies every
 * edit the model applies (to give the text the edits leave), so it needs the bytes an
 * `attachFile` edit names too. An `attachFile` edit names them by the ID this answers.
 */
export function addBlob(bytes: Uint8Array, id: number): Promise<string> {
  const opened = current(id);
  return serial(opened, async () => {
    const blob = opened.model.addBlob(bytes);
    followEdit(opened, { blob: bytes });
    return blob;
  });
}
