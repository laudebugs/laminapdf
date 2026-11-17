// The object inspector's reads (O15) of the open document, through the extract module's copy as
// the properties view reads: the claims it makes, its fonts and colour profiles, its objects and
// revisions, one object with what refers to it, and a stream's first bytes. Nothing here changes
// the document; decoding a stream stays within the document's limits.

import type { Document } from "@laminapdf/sdk";
import type { InspectionOverview, ObjectDetail, StreamRead } from "../protocol.js";
import { textModel } from "./document.js";

/** Runs `read` on the extract module's copy of the open document, after the work queued on it. */
async function inspect<T>(read: (model: Document) => T, signal: AbortSignal): Promise<T> {
  const model = await textModel(signal);
  return model.ensure(() => read(model), { signal });
}

/** What the inspector shows before anything is picked: the claims, objects, revisions, fonts and colour. */
export function inspectionOverview(signal: AbortSignal): Promise<InspectionOverview> {
  return inspect(
    (model) => ({
      trailer: model.trailer(),
      conformance: model.conformance(),
      objects: model.objects(),
      revisions: model.revisions(),
      fonts: model.fonts(),
      colour: model.colorProfiles(),
    }),
    signal,
  );
}

/** Object `num` with what refers to it, written as PDF syntax too; `undefined` without such an object. */
export function objectDetail(num: number, signal: AbortSignal): Promise<ObjectDetail | undefined> {
  return inspect((model) => {
    const object = model.object(num);
    if (object === undefined) return undefined;
    return { object, referrers: model.referencesTo(num), syntax: model.objectSyntax(num) ?? "" };
  }, signal);
}

/** The first `max` bytes of stream `num`, raw or decoded; `undefined` when `num` isn't a stream. */
export function streamRead(
  num: number,
  view: "raw" | "decoded",
  max: number,
  signal: AbortSignal,
): Promise<StreamRead | undefined> {
  return inspect((model) => model.streamBytes(num, view, max), signal);
}
