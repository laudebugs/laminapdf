// An invoice from a JSON template and its data: the template lays out the heading, the text and
// a table, and the data fills in the number, the line items and the total.
//
// The table runs onto a second page with its header row repeated, and the total stays with the
// last rows. The template names its two fonts, `regular` and `bold`; their bytes come beside it.
// The output is checked against PDF/UA-1, which the template asks for.
import { Builder, type BuilderOptions, type FlowTemplate, Font } from "@laminapdf/sdk";

/**
 * The invoice's bytes. Needs `init({ text: true })`.
 *
 * @param template - the invoice template, a `laminapdf.document/1` document with placeholders.
 * @param data - the values its placeholders read.
 * @param regular - the font the template calls `regular`.
 * @param bold - a collection whose second face (index 1) is the one it calls `bold`.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function invoiceFromJson(
  template: FlowTemplate,
  data: unknown,
  regular: Uint8Array,
  bold: Uint8Array,
  options: BuilderOptions = {},
): Uint8Array<ArrayBuffer> {
  // The builder checks the profiles it is given: the ones the template asks for.
  using b = new Builder({ ...options, profiles: ["PDF/UA-1"] });
  b.flow(template, {
    data,
    fonts: { regular: Font.fromBytes(regular), bold: Font.fromCollection(bold, 1) },
  });
  using doc = b.finish();
  return doc.save();
}
