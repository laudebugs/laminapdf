// Text added to pages as content: doc.content.addText, update (a text patch) and remove, saved and
// reopened, in the text module. A test's title starts with the capability it proves (its tag:
// capabilities A.01 and A.12, column pdf), which `cargo xtask capabilities` counts.
import { Document, init, isLaminaPdfError, type NewText } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init({ text: true }));

const AT = "D:20261118120000Z";
const NOTE: NewText = {
  id: "note",
  rect: [72, 100, 288, 148],
  runs: [{ text: "Reviewed " }, { text: "twice", bold: true }],
  fonts: ["helvetica"],
  modified: AT,
};

function savedAndReopened(doc: Document): Document {
  return Document.open(doc.save({ mode: "incremental" }));
}

function textOf(doc: Document): string {
  return doc.page(0).text().text().replace(/\s+/g, " ");
}

describe("text added to pages", () => {
  it("[cap A.01 pdf round-trip] adds text in a standard font that reads back and reopens where it was put", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    doc.content.addText(0, NOTE);
    using again = savedAndReopened(doc);
    const [note] = again.content.added(0);
    expect([note?.id, note && "text" in (note.object ?? {}), note?.bounds]).toEqual([
      "note",
      true,
      [72, 100, 288, 148],
    ]);
    expect(textOf(again)).toContain("Reviewed twice");
  });

  it("[cap A.01 pdf round-trip] sets text in a font given by its bytes in five scripts and reads each back", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    const scripts: [string, string, string][] = [
      ["latin", "NotoSans-Regular.ttf", "Zoë reviewed the draft twice"],
      ["arabic", "NotoSansArabic-Regular.ttf", "مرحبا بالعالم"],
      ["hebrew", "NotoSansHebrew-Regular.ttf", "שלום עולם"],
      ["devanagari", "NotoSansDevanagari-Regular.ttf", "नमस्ते दुनिया"],
      ["chinese", "NotoSansSC-Regular.otf", "你好，世界"],
    ];
    scripts.forEach(([id, file, text], i) => {
      const top = 100 + 40 * i;
      doc.content.addText(0, {
        id,
        rect: [72, top, 400, top + 32],
        runs: [{ text }],
        fonts: [doc.addBlob(read(`fixtures/fonts/${file}`))],
        size: 16,
        modified: AT,
      });
    });
    using again = savedAndReopened(doc);
    const read_back = textOf(again);
    for (const [, , text] of scripts) expect(read_back).toContain(text);
  });

  it("[cap A.12 pdf round-trip] draws each character in the first font of the chain that has it", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    const text = "Grüße שלום end";
    doc.content.addText(0, {
      ...NOTE,
      id: "mixed",
      runs: [{ text }],
      fonts: [
        doc.addBlob(read("fixtures/fonts/NotoSans-Regular.ttf")),
        doc.addBlob(read("fixtures/fonts/NotoSansHebrew-Regular.ttf")),
      ],
    });
    using again = savedAndReopened(doc);
    expect(textOf(again)).toContain(text);
  });

  it("changes the text, moves it and removes it after a save", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    doc.content.addText(0, NOTE);
    using again = savedAndReopened(doc);
    again.content.update(0, "note", {
      modified: AT,
      text: { width: 216, height: 48, runs: [{ text: "Changed after the save" }] },
      matrix: [1, 0, 0, -1, 100, 300],
    });
    expect(textOf(again)).toContain("Changed after the save");
    expect(again.content.added(0)[0]?.bounds).toEqual([100, 252, 316, 300]);
    const removed = again.content.remove(0, "note");
    expect([again.content.added(0), removed.stillInEarlierRevisions]).toEqual([[], true]);
  });

  it("refuses a character no font has, a font that isn't one, and an ID the page has", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    const code = (change: Partial<NewText>) => {
      const error = thrown(() => doc.content.addText(0, { ...NOTE, ...change }));
      return isLaminaPdfError(error) ? error.code : error;
    };
    expect(code({ runs: [{ text: "check ✓" }] })).toBe("content-glyph-missing");
    expect(code({ fonts: [doc.addBlob(new TextEncoder().encode("not a font"))] })).toBe("content-font-invalid");
    doc.content.addText(0, NOTE);
    expect(code({})).toBe("content-id-in-use");
  });
});
