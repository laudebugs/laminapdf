// Make a PDF/A-3 invoice a Factur-X (ZUGFeRD) one: attach its structured data as factur-x.xml,
// the document's alternative form, and record the Factur-X properties in the XMP metadata, which
// keeps the file valid PDF/A-3.
// The XML here is a placeholder: a real one is the invoice in the EN 16931 Cross Industry Invoice
// syntax, made by your invoicing system. The file must already be PDF/A-3.
import { Document } from "@laminapdf/sdk";

/** The Factur-X XMP namespace and prefix (Factur-X 1.0, §6.2.2). */
const FX = "urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#";

/** A placeholder for the invoice's Cross Industry Invoice XML. */
const XML = `<?xml version="1.0" encoding="UTF-8"?>
<rsm:CrossIndustryInvoice xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100">
  <!-- The invoice's data, from your invoicing system. -->
</rsm:CrossIndustryInvoice>
`;

/**
 * The invoice with factur-x.xml attached and the Factur-X properties recorded, saved incrementally.
 *
 * @param savedAt - the time of the save, as a PDF date (`D:20261110093000Z`) your app takes
 *   from its clock.
 */
export function attachFacturX(bytes: Uint8Array, savedAt: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);

  // PDF/A-3 asks every attached file for an association, a MIME type and a modification date.
  const xml = doc.attachFileOp("factur-x.xml", new TextEncoder().encode(XML), "text/xml", {
    attachTo: "document",
    relationship: "alternative",
    description: "Factur-X invoice",
    modified: savedAt,
  });
  doc.transact((tx) => {
    tx.apply({ op: "attachFile", args: xml });
    const fx = (name: string, value: string) => tx.setXmpProperty(FX, "fx", name, { type: "text", value });
    fx("DocumentType", "INVOICE");
    fx("DocumentFileName", "factur-x.xml");
    fx("Version", "1.0");
    fx("ConformanceLevel", "MINIMUM");
  });

  // An attachment that breaks the file's PDF/A level says so before the save; this one keeps it.
  for (const warning of doc.plan({ mode: "incremental", metadataDate: savedAt }).conformanceWarnings) {
    console.warn(warning.code);
  }
  return doc.save({ mode: "incremental", metadataDate: savedAt });
}
