import { describe, expect, it } from "vitest";
import {
  type DocumentPropertiesData,
  formatDate,
  formatFileSize,
  pageSizeSummary,
  paperName,
  permissionsValue,
  propertiesText,
  propertySections,
} from "../src/reader/properties.ts";

const BASE: DocumentPropertiesData = {
  title: "Q3 Board Report",
  creator: "Northwind report template",
  producer: "Northwind print service",
  created: "2026-10-14T09:12:00Z",
  version: "1.7",
  pageCount: 3,
  fileSize: 1_262_480,
  linearized: false,
  tagged: true,
  fonts: [
    { name: "Helvetica", kind: "Type1", embedded: false, subset: false },
    { name: "NotoSans-Regular", kind: "Type0", embedded: true, subset: true },
  ],
  signatures: 2,
  contains: { javascript: false, attachments: 3, forms: false },
};

const CONTEXT = {
  pages: [
    { width: 612, height: 792 },
    { width: 612, height: 792 },
    { width: 1224, height: 792 },
  ],
  labels: ["1", "2", "A-1"],
  locale: "en-GB",
};

describe("properties", () => {
  it("names paper sizes either way up, and gives others in inches", () => {
    expect(paperName(595.28, 841.89)).toBe("A4");
    expect(paperName(792, 612)).toBe("US Letter");
    expect(paperName(500, 500)).toBe("6.94 × 6.94 in");
  });

  it("summarises one page size, or the mix, most common first", () => {
    expect(pageSizeSummary([{ width: 612, height: 792 }])).toBe("US Letter · all pages");
    expect(pageSizeSummary(CONTEXT.pages)).toBe("Mixed: 2 US Letter, 1 Tabloid");
  });

  it("gives the file size short and exact", () => {
    expect(formatFileSize(1_262_480, "en-GB")).toBe("1.3 MB (1,262,480 bytes)");
    expect(formatFileSize(512, "en-GB")).toBe("512 bytes");
  });

  it("formats dates in the locale, a date alone without shifting its day", () => {
    expect(formatDate("2026-10-14", "en-GB")).toBe("14 Oct 2026");
    expect(formatDate("not a date", "en-GB")).toBe("not a date");
  });

  it("merges the creator and producer into Made with, and labels the pages by their runs", () => {
    const { description, document } = propertySections(BASE, CONTEXT);
    expect(description.rows.find((r) => r.label === "Made with")?.value).toBe(
      "Northwind report template · saved by Northwind print service",
    );
    expect(document.rows.find((r) => r.label === "Pages")?.value).toBe("3 · labelled 1–2, A-1");
    expect(document.rows.find((r) => r.label === "Contains")?.value).toBe("3 attachments · no forms · no JavaScript");
  });

  it("describes encryption, the author's permissions and signatures", () => {
    const permissions = {
      print: false,
      printHighQuality: false,
      modify: false,
      copy: true,
      annotate: true,
      fillForms: true,
      accessibility: true,
      assemble: false,
    };
    const encrypted: DocumentPropertiesData = {
      ...BASE,
      encryption: { revision: 6, keyBits: 256, cipher: "aes-256", permissions },
    };
    const rows = propertySections(encrypted, CONTEXT).security.rows;
    expect(rows.map((r) => r.value)).toEqual([
      "Password protected · AES 256-bit (revision 6)",
      "Printing: not allowed · Copying text: allowed · Changing the document: not allowed · Comments: allowed · Filling forms: allowed",
      "2 signatures present, not verified",
    ]);
    expect(permissionsValue({ ...permissions, print: true })).toContain("Printing: standard quality only");
  });

  it("writes every section as plain text for Copy all", () => {
    const text = propertiesText(BASE, CONTEXT);
    expect(text).toContain("Description\nTitle: Q3 Board Report");
    expect(text).toContain("Fonts\nHelvetica · Type1 · Not embedded · shown with a similar font");
    expect(text).toContain("Security\nEncryption: None");
  });
});

describe("a cipher this version doesn't name", () => {
  it("still reads as password protected, never as unencrypted", () => {
    const rows = propertySections({ ...BASE, encryption: { revision: 7, keyBits: 256, cipher: "other" } }, CONTEXT)
      .security.rows;
    expect(rows[0]?.value).toBe("Password protected · a cipher Lamina PDF doesn't name, 256-bit (revision 7)");
  });
});
