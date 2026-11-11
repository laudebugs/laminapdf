// builder.markdown and Font.fromCollection over the text module (T4i): the faces of a collection,
// the warnings a build leaves, and the options record's checks.
import { Builder, Font, init, isLaminaPdfError } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init({ text: true }));

const COLLECTION = "fixtures/fonts/NotoSans-RegularBold.ttc";

describe("Font.fromCollection", () => {
  it("loads each face of a collection as its own font", () => {
    using b = new Builder();
    const bytes = read(COLLECTION);
    const regular = b.font(Font.fromCollection(bytes, 0)).postscriptName;
    const bold = b.font(Font.fromCollection(bytes, 1)).postscriptName;
    expect([regular, bold]).toEqual(["NotoSans-Regular", "NotoSans-Bold"]);
  });

  it.each([-1, 1.5, Number.NaN, 2 ** 32])("refuses %s as a face number", (index) => {
    expect(
      isLaminaPdfError(
        thrown(() => Font.fromCollection(read(COLLECTION), index)),
        "invalid-argument",
      ),
    ).toBe(true);
  });
});

describe("Builder.markdown", () => {
  it("leaves out raw HTML with a warning and keeps the pages it built", () => {
    using b = new Builder();
    const report = b.markdown("# Notes\n\nHello <span>there</span>.\n");
    expect(report.pages).toHaveLength(1);
    using doc = b.finish();
    // One per piece of HTML: the opening and the closing tag.
    expect(doc.warnings.map((w) => w.code)).toEqual(["markdown-html-ignored", "markdown-html-ignored"]);
  });

  it("refuses an unknown option", () => {
    using b = new Builder();
    const options = { contentz: 2 } as unknown as Parameters<Builder["markdown"]>[1];
    expect(
      isLaminaPdfError(
        thrown(() => b.markdown("# Hi\n", options)),
        "unknown-option",
      ),
    ).toBe(true);
  });
});
