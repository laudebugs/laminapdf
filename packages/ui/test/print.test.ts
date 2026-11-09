import { describe, expect, it } from "vitest";
import { printRules } from "../src/platform/print.js";

describe("printRules", () => {
  it("gives each page a named page of its own size when fitting", () => {
    const rules = printRules(
      [
        { width: 612, height: 792 },
        { width: 841.89, height: 595.276 },
      ],
      "fit",
    );
    expect(rules).toContain("@page p0 { size: 612pt 792pt; margin: 0; }");
    expect(rules).toContain("@page p1 { size: 841.89pt 595.28pt; margin: 0; }");
    expect(rules).toContain(".lamina-print-page-1 { page: p1; }");
  });

  it("leaves the paper size to the print window at actual size", () => {
    const rules = printRules([{ width: 612, height: 792 }], "actual");
    expect(rules).toContain("@page p0 { margin: 0; }");
  });
});
