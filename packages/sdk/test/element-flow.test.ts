// The element-flow example's choices that its golden can't show: a header too long for the page
// fails instead of losing text, and the body takes as many pages as it needs. Its placements are
// the Rust example's, byte for byte (test/suite/examples.ts); crates/laminapdf/tests/element_flow.rs
// checks them against the font's metrics.
import { Document, init } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { elementFlow } from "../examples/element-flow.js";
import { read } from "./helpers.js";

beforeAll(() => init({ text: true }));

const photo = () => read("fixtures/images/photo-rgb.jpg");
const report = { title: "Report", summary: "One line.", notes: "", body: "" };

describe("the element-flow example", () => {
  it("throws text-overflow for a header too long for the page, rather than dropping text", () => {
    const long = "Revenue rose twelve per cent over the quarter. ".repeat(80);
    expect(() => elementFlow(photo(), { ...report, summary: long })).toThrow(
      expect.objectContaining({ code: "text-overflow" }),
    );
  });

  it("writes one page when the body fits, and more when it doesn't", () => {
    const pages = (body: string) => {
      using doc = Document.open(elementFlow(photo(), { ...report, body }));
      return doc.pageCount;
    };
    expect(pages("Short.")).toBe(1);
    expect(pages("A paragraph that goes on. ".repeat(120))).toBeGreaterThan(1);
  });
});
