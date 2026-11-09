import { describe, expect, it } from "vitest";
import { downloadName } from "./download-name.ts";

describe("downloadName", () => {
  it("keeps an ordinary name and gives one without an extension .pdf", () => {
    expect(downloadName("Q3-board-report.pdf")).toBe("Q3-board-report.pdf");
    expect(downloadName("report")).toBe("report.pdf");
    expect(downloadName("Report.PDF")).toBe("Report.pdf");
  });

  it("drops a path and replaces characters a file system refuses", () => {
    expect(downloadName("../../etc/passwd")).toBe("passwd.pdf");
    expect(downloadName("a\\b:c*d?.pdf")).toBe("b_c_d_.pdf");
    expect(downloadName("tab\there\u0000.pdf")).toBe("tab_here_.pdf");
  });

  it("avoids hidden, empty and reserved names", () => {
    expect(downloadName(".hidden.pdf")).toBe("hidden.pdf");
    expect(downloadName("...")).toBe("document.pdf");
    expect(downloadName("")).toBe("document.pdf");
    expect(downloadName("CON.pdf")).toBe("_CON.pdf");
    expect(downloadName("lpt1.backup.pdf")).toBe("_lpt1.backup.pdf");
  });

  it("shortens a very long name", () => {
    expect(downloadName(`${"a".repeat(300)}.pdf`)).toBe(`${"a".repeat(200)}.pdf`);
  });
});
