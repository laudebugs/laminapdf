// The object inspector's reads (O15) of the open document, through the extract module's copy as
// the properties view reads. Each is asked for when the inspector shows it: the summary when it
// opens (the trailer, counts and the version, nothing that reads every object), a tab's facts
// when the tab is first shown, an object when the tree or the detail needs it, what refers to an
// object when its detail asks, and a stream's first bytes. Nothing here changes the document;
// decoding a stream stops at what is shown and stays within the document's limits.

import type { CosValue, Document, ObjectEntry } from "@laminapdf/sdk";
import type {
  InspectionSummary,
  InspectionTab,
  InspectionTabData,
  ObjectDetail,
  ObjectReferrers,
  StreamRead,
} from "../protocol.js";
import { textModel } from "./document.js";

/** Runs `read` on the extract module's copy of the open document, after the work queued on it. */
async function inspect<T>(read: (model: Document) => T, signal: AbortSignal): Promise<T> {
  const model = await textModel(signal);
  return model.ensure(() => read(model), { signal });
}

/** The objects `value` refers to, one level down, each once. */
function referenced(value: CosValue, out = new Set<number>()): Set<number> {
  if (value.type === "reference") out.add(value.num);
  else if (value.type === "array") for (const item of value.items) referenced(item, out);
  else if (value.type === "dictionary") for (const [, item] of value.entries) referenced(item, out);
  return out;
}

/** The entries of the objects `nums` names that exist, read alone. */
function entries(model: Document, nums: Iterable<number>): ObjectEntry[] {
  return [...nums].flatMap((num) => model.objectEntry(num) ?? []);
}

/** What the inspector shows as it opens: the trailer, what it refers to, the counts and the version. */
export function inspectionSummary(signal: AbortSignal): Promise<InspectionSummary> {
  return inspect((model) => {
    const trailer = model.trailer();
    const claims = model.conformance();
    return {
      trailer,
      entries: entries(model, referenced(trailer)),
      objects: model.objectCount(),
      revisions: model.revisionCount(),
      headerVersion: claims.headerVersion,
      catalogVersion: claims.catalogVersion,
    };
  }, signal);
}

/** The facts of tab `tab`, read when the tab is first shown. */
export function inspectionTab(tab: InspectionTab, signal: AbortSignal): Promise<InspectionTabData> {
  return inspect((model): InspectionTabData => {
    switch (tab) {
      case "revisions":
        return { tab, revisions: model.revisions() };
      case "fonts":
        return { tab, fonts: model.fonts() };
      case "colour":
        return { tab, colour: model.colorProfiles() };
      default:
        return { tab: "conformance", conformance: model.conformance() };
    }
  }, signal);
}

/** Object `num`, the entries of what it refers to and its PDF syntax; `undefined` without such an object. */
export function objectDetail(num: number, signal: AbortSignal): Promise<ObjectDetail | undefined> {
  return inspect((model) => {
    const object = model.object(num);
    if (object === undefined) return undefined;
    return { object, children: entries(model, referenced(object.value)), syntax: model.objectSyntax(num) ?? "" };
  }, signal);
}

/** The places that refer to object `num`, with the entries of the objects that hold them. */
export function objectReferrers(num: number, signal: AbortSignal): Promise<ObjectReferrers> {
  return inspect((model) => {
    const referrers = model.referencesTo(num);
    const holders = new Set(referrers.flatMap((r) => (r.from === undefined ? [] : [r.from])));
    return { referrers, entries: entries(model, holders) };
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
