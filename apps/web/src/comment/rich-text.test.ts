import { describe, expect, it } from "vitest";
import { type EditorNode, editorHtml, isBlank, needsFallback, plainOf, richTextOf } from "./rich-text.ts";

/** A text node. */
const t = (value: string): EditorNode => ({ nodeType: 3, nodeName: "#text", nodeValue: value, childNodes: [] });

/** An element with `children`, and an inline style. */
const el = (name: string, children: EditorNode[] = [], style?: EditorNode["style"]): EditorNode => ({
  nodeType: 1,
  nodeName: name,
  nodeValue: null,
  childNodes: children,
  ...(style === undefined ? {} : { style }),
});

const root = (...children: EditorNode[]) => el("DIV", children);

describe("richTextOf", () => {
  it("reads bold and italic, as tags or inline styles, into one span per run of a style", () => {
    const box = root(
      t("Check "),
      el("B", [t("this "), el("I", [t("now")])]),
      el("SPAN", [t(" please")], { fontWeight: "700" }),
    );
    expect(richTextOf(box)).toEqual([
      [
        { text: "Check " },
        { text: "this ", bold: true },
        { text: "now", bold: true, italic: true },
        { text: " please", bold: true },
      ],
    ]);
  });

  it("starts a paragraph at each block and line break, as a browser lays the box out", () => {
    // Chromium and WebKit wrap each new line in a div; Firefox, and Shift+Enter, break with br.
    const box = root(
      t("One"),
      el("DIV", [t("Two")]),
      el("DIV", [el("BR")]),
      el("DIV", [t("Four"), el("BR"), t("Five")]),
    );
    expect(plainOf(richTextOf(box))).toBe("One\nTwo\n\nFour\nFive");
  });

  it("drops the empty line a trailing break leaves", () => {
    expect(richTextOf(root(t("End"), el("BR")))).toEqual([[{ text: "End" }]]);
  });

  it("reads a non-breaking space as a space", () => {
    expect(plainOf(richTextOf(root(t("a b"))))).toBe("a b");
  });
});

describe("editorHtml", () => {
  it("writes a div per paragraph, styles as b and i, and escapes the text", () => {
    const text = [[{ text: "a < b" }, { text: "bold", bold: true, italic: true }], []];
    expect(editorHtml(text)).toBe("<div>a &lt; b<b><i>bold</i></b></div><div><br></div>");
  });
});

describe("isBlank", () => {
  it("is true for spaces and empty paragraphs only", () => {
    expect([isBlank([[{ text: "  " }], []]), isBlank([[{ text: "x" }]])]).toEqual([true, false]);
  });
});

describe("needsFallback", () => {
  it("is true for Arabic, which the standard fonts lack, and false for Latin text", () => {
    expect([needsFallback([[{ text: "Hello مرحبا" }]]), needsFallback([[{ text: "Café façade" }]])]).toEqual([
      true,
      false,
    ]);
  });
});
