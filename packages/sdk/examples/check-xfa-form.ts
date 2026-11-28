// Check whether a PDF form is an XFA form, and make a static one show the same answers in every
// app. An XFA form is an older Adobe format that only Adobe's apps lay out. A dynamic one has
// placeholder pages and nothing to fill here; a static one has real pages and ordinary fields,
// but Adobe's apps show the XFA part's own answers, so the part is removed.
import { Document } from "@laminapdf/sdk";

/**
 * The form as it saves: without its XFA part when it is a static XFA form, unchanged otherwise.
 * Needs `init({ extract: true })`.
 */
export function checkXfaForm(form: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(form);
  const xfa = doc.xfaFacts();
  if (xfa === undefined) {
    console.warn("not an XFA form");
  } else if (xfa.kind === "dynamic") {
    // Only Adobe's apps lay it out: leave it as it is.
    console.warn("a dynamic XFA form: open it in Adobe Acrobat");
  } else {
    console.warn("a static XFA form: removing its XFA part");
    doc.apply([{ op: "removeXfa", args: {} }]);
  }
  return doc.save({ mode: "fullRewrite" });
}
