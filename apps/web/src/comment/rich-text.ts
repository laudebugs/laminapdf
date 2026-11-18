// A text box's text while it is typed (Paper C03): what the `contenteditable` box holds read back
// as the rich-text subset a FreeText keeps (paragraphs of spans, each plain, bold or italic; E4
// plan §11), and that subset written as the box's HTML. Reading walks a minimal node interface the
// browser's DOM satisfies, so it runs in Node with plain objects. Also: the CSS font of each
// standard family, and whether text needs the fallback font the standard fonts lack.

import type { AnnotFont, RichText, TextSpan } from "@laminapdf/sdk";

/** What reading needs of a DOM node. */
export interface EditorNode {
  /** 1 for an element, 3 for text. */
  readonly nodeType: number;
  /** `DIV`, `B`, `#text`, … */
  readonly nodeName: string;
  readonly nodeValue: string | null;
  readonly childNodes: ArrayLike<EditorNode>;
  /** An element's inline style. */
  readonly style?: { readonly fontWeight?: string; readonly fontStyle?: string };
}

/** Elements that start a paragraph: what browsers wrap each line of a `contenteditable` in. */
const BLOCKS = new Set(["DIV", "P", "LI", "H1", "H2", "H3", "H4", "H5", "H6", "BLOCKQUOTE"]);

/** The style a node gives the text inside it. */
interface Style {
  readonly bold: boolean;
  readonly italic: boolean;
}

function styleOf(node: EditorNode, outer: Style): Style {
  const name = node.nodeName;
  const weight = node.style?.fontWeight ?? "";
  const bold =
    name === "B" ||
    name === "STRONG" ||
    weight === "bold" ||
    weight === "bolder" ||
    Number.parseInt(weight, 10) >= 600 ||
    (outer.bold && weight !== "normal" && weight !== "400");
  const slant = node.style?.fontStyle ?? "";
  const italic = name === "I" || name === "EM" || slant === "italic" || (outer.italic && slant !== "normal");
  return { bold, italic };
}

/** The rich text `root`, the editing box, holds: one paragraph per line, as browsers lay lines out. */
export function richTextOf(root: EditorNode): RichText {
  const paragraphs: TextSpan[][] = [[]];
  // Whether the paragraph being read has anything in it yet, a line break included.
  let started = false;
  const add = (text: string, style: Style) => {
    const spans = paragraphs[paragraphs.length - 1] as TextSpan[];
    const last = spans[spans.length - 1];
    if (last !== undefined && (last.bold ?? false) === style.bold && (last.italic ?? false) === style.italic) {
      spans[spans.length - 1] = { ...last, text: last.text + text };
    } else {
      spans.push({ text, ...(style.bold ? { bold: true } : {}), ...(style.italic ? { italic: true } : {}) });
    }
    started = true;
  };
  const breakLine = () => {
    paragraphs.push([]);
    started = false;
  };
  const walk = (node: EditorNode, style: Style) => {
    if (node.nodeType === 3) {
      const text = (node.nodeValue ?? "").replace(/ /g, " ").replace(/\r?\n/g, " ");
      if (text !== "") add(text, style);
      return;
    }
    if (node.nodeType !== 1) return;
    if (node.nodeName === "BR") {
      breakLine();
      return;
    }
    const block = BLOCKS.has(node.nodeName);
    // A block starts a new paragraph unless the one before is still empty.
    if (block && started) breakLine();
    const inner = styleOf(node, style);
    for (let i = 0; i < node.childNodes.length; i++) walk(node.childNodes[i] as EditorNode, inner);
    // A block's last line break only ends its line, as a browser draws it.
    if (block) {
      const spans = paragraphs[paragraphs.length - 1] as TextSpan[];
      if (spans.length > 0 || started) breakLine();
    }
  };
  for (let i = 0; i < root.childNodes.length; i++)
    walk(root.childNodes[i] as EditorNode, { bold: false, italic: false });
  // The paragraph a trailing break or block leaves open holds nothing.
  while (paragraphs.length > 1 && (paragraphs[paragraphs.length - 1] as TextSpan[]).length === 0) paragraphs.pop();
  return paragraphs;
}

/** Whether `text` holds nothing but spaces. */
export function isBlank(text: RichText): boolean {
  return text.every((p) => p.every((s) => s.text.trim() === ""));
}

/** `text` with one paragraph per line, and no styles. */
export function plainOf(text: RichText): string {
  return text.map((p) => p.map((s) => s.text).join("")).join("\n");
}

function escape(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/** `text` as the editing box's HTML: a `div` per paragraph, `b` and `i` for the styles. */
export function editorHtml(text: RichText): string {
  return text
    .map((spans) => {
      const inner = spans
        .filter((s) => s.text !== "")
        .map((s) => {
          let html = escape(s.text);
          if (s.italic === true) html = `<i>${html}</i>`;
          if (s.bold === true) html = `<b>${html}</b>`;
          return html;
        })
        .join("");
      return `<div>${inner === "" ? "<br>" : inner}</div>`;
    })
    .join("");
}

/** The CSS font of a box's font, near enough to what the appearance draws while it is typed. */
export function cssFont(font: AnnotFont | undefined): string {
  switch (font) {
    case "times":
      return '"Times New Roman", Times, "Liberation Serif", serif';
    case "courier":
      return '"Courier New", Courier, "Liberation Mono", monospace';
    default:
      return 'Helvetica, Arial, "Liberation Sans", sans-serif';
  }
}

/** Arabic and its presentation forms: what the bundled fallback font, Noto Sans Arabic, draws. */
const ARABIC = /[؀-ۿݐ-ݿࡰ-ࣿﭐ-﷿ﹰ-ﻼ]/u;

/**
 * Whether `text` holds characters the standard fonts lack that the fallback font draws: Arabic.
 * The standard fonts cover Latin text (WinAnsiEncoding); any other script fails when the box is
 * drawn, as `annotation-glyph-missing`.
 */
export function needsFallback(text: RichText): boolean {
  return ARABIC.test(plainOf(text));
}
