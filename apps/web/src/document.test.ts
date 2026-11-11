import { describe, expect, it } from "vitest";
import { repairsFrom, repairsText, wantsDebug, windowTitle } from "./document.ts";

describe("windowTitle", () => {
  it("names the app alone while nothing is open", () => {
    expect(windowTitle(undefined)).toBe("Lamina PDF");
  });

  it("prefers the document's own title to its file name", () => {
    expect(windowTitle({ title: "Annual report", fileName: "report.pdf" })).toBe("Annual report · Lamina PDF");
  });

  it("falls back to the file name when the title is missing or blank", () => {
    expect(windowTitle({ fileName: "scan.pdf" })).toBe("scan.pdf · Lamina PDF");
    expect(windowTitle({ title: "  ", fileName: "scan.pdf" })).toBe("scan.pdf · Lamina PDF");
  });

  it("starts with a dot while the document has unsaved changes", () => {
    expect(windowTitle({ fileName: "scan.pdf", unsaved: true })).toBe("• scan.pdf · Lamina PDF");
  });
});

describe("repairsFrom", () => {
  it("keeps one entry per code, in the order the codes first came", () => {
    const repairs = repairsFrom([
      { code: "xref-repaired" },
      { code: "stream-length-recovered", location: { pageIndex: 3 } },
      { code: "xref-repaired" },
      { code: "stream-length-recovered", location: { pageIndex: 0 } },
      { code: "stream-length-recovered", location: { pageIndex: 3 } },
    ]);
    expect(repairs).toEqual([{ code: "xref-repaired" }, { code: "stream-length-recovered", pages: [1, 4] }]);
  });

  it("is empty for a file that needed no repair", () => {
    expect(repairsFrom([])).toEqual([]);
  });
});

describe("repairsText", () => {
  it("writes one code per line with its pages", () => {
    expect(repairsText([{ code: "xref-repaired" }, { code: "stream-length-recovered", pages: [1, 4] }])).toBe(
      "xref-repaired\nstream-length-recovered (pages 1, 4)",
    );
  });
});

describe("wantsDebug", () => {
  it("is true only for a debug key in the fragment", () => {
    expect(wantsDebug("#debug")).toBe(true);
    expect(wantsDebug("#url=https%3A%2F%2Fh%2Fa.pdf&debug")).toBe(true);
    expect(wantsDebug("")).toBe(false);
    expect(wantsDebug("#debugger")).toBe(false);
  });
});
