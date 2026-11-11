// builder.flow over the text module: typed `laminapdf.document/1` documents (checked by tsc against
// the types generated from the committed schema), templates filled from data, and the errors a
// document or its data can raise.
import { Builder, type FlowDocument, type FlowTemplate, init, isLaminaPdfError } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { thrown } from "./helpers.js";

beforeAll(() => init({ text: true }));

/** The style every block names when it names none: a standard 14 font, so no font is passed. */
const BODY = { body: { font: "Helvetica", size: 10 } };

const NOTE: FlowDocument = {
  schema: "laminapdf.document/1",
  styles: BODY,
  blocks: [
    { type: "heading", level: 1, text: "Note" },
    { type: "paragraph", text: "Plain Latin text, set in the standard 14 fonts." },
  ],
};

describe("Builder.flow", () => {
  it("lays out a document as pages and reports them", () => {
    using b = new Builder();
    const report = b.flow(NOTE);
    expect(report.pages).toHaveLength(1);
    using doc = b.finish();
    expect(doc.pageCount).toBe(1);
  });

  it("fills a template from data: values, repeated blocks and blocks shown only when a value is set", () => {
    const template: FlowTemplate = {
      schema: "laminapdf.document/1",
      styles: BODY,
      blocks: [
        { type: "paragraph", text: { $data: "/title" } },
        { $each: "/lines", do: { type: "paragraph", text: { $item: "" } } },
        { $if: "/paid", do: { type: "paragraph", text: "Paid." } },
      ],
    };
    using b = new Builder();
    b.flow(template, { data: { title: "Lines", lines: ["one", "two"], paid: false } });
    using doc = b.finish();
    expect(doc.page(0).text().text()).toMatch(/Lines\s+one\s+two\s*$/);
  });

  it("names the data a template lacks in the error's data", () => {
    const template: FlowTemplate = {
      schema: "laminapdf.document/1",
      styles: BODY,
      blocks: [{ type: "paragraph", text: { $data: "/customer/name" } }],
    };
    using b = new Builder();
    const error = thrown(() => b.flow(template, { data: { customer: {} } }));
    expect(isLaminaPdfError(error, "template-data-missing")).toBe(true);
    expect((error as { data?: { pointer?: string } }).data?.pointer).toBe("/customer/name");
  });

  it("refuses a misspelt key with the closest one", () => {
    const misspelt = { schema: "laminapdf.document/1", styles: BODY, blokcs: [] } as unknown as FlowDocument;
    using b = new Builder();
    const error = thrown(() => b.flow(misspelt));
    expect(isLaminaPdfError(error, "invalid-document")).toBe(true);
    expect((error as { didYouMean?: string[] }).didYouMean).toContain("blocks");
  });

  it("refuses an unknown option", () => {
    using b = new Builder();
    const options = { dat: {} } as unknown as Parameters<Builder["flow"]>[1];
    expect(
      isLaminaPdfError(
        thrown(() => b.flow(NOTE, options)),
        "unknown-option",
      ),
    ).toBe(true);
  });

  it("types documents by the schema: tsc refuses a misspelt key and an unknown block type", () => {
    const misspelt: FlowDocument = {
      schema: "laminapdf.document/1",
      styles: BODY,
      // @ts-expect-error: `blokcs` is not a key of a document.
      blokcs: [],
    };
    const unknownBlock: FlowDocument = {
      schema: "laminapdf.document/1",
      styles: BODY,
      // @ts-expect-error: `heading1` is not a block type.
      blocks: [{ type: "heading1", text: "Title" }],
    };
    const placeholder: FlowDocument = {
      schema: "laminapdf.document/1",
      styles: BODY,
      // @ts-expect-error: a placeholder is not a value of a plain document, only of a template.
      blocks: [{ type: "paragraph", text: { $data: "/x" } }],
    };
    expect([misspelt, unknownBlock, placeholder]).toHaveLength(3);
  });
});
