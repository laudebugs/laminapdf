// What copy writes (UI/UX plan §5.3): the selected pieces of the text layer's spans, joined in one
// of two modes. Copy as paragraph, the default, follows the reading order of Lamina PDF's
// extraction: the pieces go block by block, a block's lines join into one line (a word broken by
// a hyphen at a line's end joined again), and blocks are separated by a blank line, so a paragraph
// pasted from a two-column page reads as one paragraph, column after column. Copy exact keeps the
// layout's line breaks. Both write the same text as HTML too, a paragraph per block. A table's
// lines keep their breaks in both modes, since joining them would merge its cells.

import type { ReadingBlock } from "../protocol.js";
import { type Piece, separator } from "./spans.js";

/** How copy joins a selection: by paragraph (reading order) or as laid out. */
export type CopyMode = "paragraph" | "exact";

/** What copy writes to the clipboard. */
export interface Copied {
  readonly text: string;
  readonly html: string;
}

/** U+00AD, which marks where a word may break: written as a code, since it is invisible. */
const SOFT_HYPHEN = String.fromCharCode(0xad);

/** The Latin ligatures of Unicode's Alphabetic Presentation Forms, as the letters they join. */
const LIGATURES: Readonly<Record<string, string>> = {
  ﬀ: "ff",
  ﬁ: "fi",
  ﬂ: "fl",
  ﬃ: "ffi",
  ﬄ: "ffl",
  ﬅ: "st",
  ﬆ: "st",
};

/** `text` with each ligature written as its letters, so the copied words can be searched. */
export function expandLigatures(text: string): string {
  return text.replace(/[ﬀ-ﬆ]/g, (c) => LIGATURES[c] ?? c);
}

/** One block of the copy: its pieces, and whether it is a table (whose lines keep their breaks). */
interface Group {
  readonly pieces: Piece[];
  readonly table: boolean;
}

/**
 * The copy of `pieces` (the selection's, in page and run order). `blocks` gives a page's
 * reading-order blocks, or `undefined` when they aren't known (yet): that page's pieces then form
 * one block in run order.
 */
export function copySelection(
  pieces: readonly Piece[],
  mode: CopyMode,
  blocks: (page: number) => readonly ReadingBlock[] | undefined,
): Copied {
  const groups = mode === "paragraph" ? byBlock(pieces, blocks) : byPage(pieces);
  const paragraphs = groups
    .map((group) => {
      const lines = linesOf(group.pieces);
      return mode === "exact" || group.table ? lines.map((l) => l.replaceAll(SOFT_HYPHEN, "")) : [joinLines(lines)];
    })
    .filter((lines) => lines.some((l) => l !== ""));
  const text = paragraphs.map((lines) => lines.join("\n")).join(mode === "paragraph" ? "\n\n" : "\n");
  const html = paragraphs.map((lines) => `<p>${lines.map(escapeHtml).join("<br>")}</p>`).join("");
  return { text, html };
}

/** The pieces page by page, in run order (copy exact). */
function byPage(pieces: readonly Piece[]): Group[] {
  const groups: Group[] = [];
  for (const piece of pieces) {
    const last = groups.at(-1);
    if (last !== undefined && last.pieces[0]?.page === piece.page) last.pieces.push(piece);
    else groups.push({ pieces: [piece], table: false });
  }
  return groups;
}

/**
 * The pieces block by block, in the page's reading order: each piece goes to the block its span's
 * centre is in, or the nearest one; within a block they keep their run order.
 */
function byBlock(pieces: readonly Piece[], blocks: (page: number) => readonly ReadingBlock[] | undefined): Group[] {
  const keyed = new Map<string, { page: number; order: number; group: Group }>();
  pieces.forEach((piece, k) => {
    const found = blocks(piece.page);
    const index = found === undefined || found.length === 0 ? -1 : nearest(found, centreOf(piece));
    const key = `${piece.page}:${index}`;
    let entry = keyed.get(key);
    if (entry === undefined) {
      // A page without blocks keeps its place by its first piece; a block, by its reading order.
      const order = index === -1 ? k : index;
      entry = { page: piece.page, order, group: { pieces: [], table: found?.[index]?.type === "table" } };
      keyed.set(key, entry);
    }
    entry.group.pieces.push(piece);
  });
  return [...keyed.values()].sort((a, b) => a.page - b.page || a.order - b.order).map((e) => e.group);
}

/** The centre of `piece`'s span, in page space. */
function centreOf(piece: Piece): { x: number; y: number } {
  const { left, top, width, height, angle } = piece.span;
  const along = { x: Math.cos(angle), y: Math.sin(angle) };
  const down = { x: -along.y, y: along.x };
  return {
    x: left + (along.x * width) / 2 + (down.x * height) / 2,
    y: top + (along.y * width) / 2 + (down.y * height) / 2,
  };
}

/** The index of the block whose box holds `p`, or else the one nearest it. */
function nearest(blocks: readonly ReadingBlock[], p: { x: number; y: number }): number {
  let best = 0;
  let bestDistance = Infinity;
  blocks.forEach((block, k) => {
    const [x, y, w, h] = block.bbox;
    const dx = Math.max(x - p.x, 0, p.x - (x + w));
    const dy = Math.max(y - p.y, 0, p.y - (y + h));
    const distance = Math.hypot(dx, dy);
    if (distance < bestDistance) {
      best = k;
      bestDistance = distance;
    }
  });
  return best;
}

/** `pieces` as the lines they are laid out on (the text layer's rule: `separator`). */
function linesOf(pieces: readonly Piece[]): string[] {
  const lines: string[] = [];
  let line = "";
  let before: Piece | undefined;
  for (const piece of pieces) {
    const gap = before === undefined ? "" : separator(before, piece);
    if (gap === "\n") {
      lines.push(line);
      line = "";
    } else line += gap;
    line += piece.text;
    before = piece;
  }
  lines.push(line);
  return lines.map((l) => expandLigatures(l).trimEnd());
}

/**
 * `lines` as one: joined by a space, but a word broken at a line's end joined again: after a soft
 * hyphen, or after a hyphen when the next line starts with a lowercase letter (a hyphen before a
 * capital or a digit, as in "Jean-" / "Paul" or "pre-" / "2020", stays).
 */
export function joinLines(lines: readonly string[]): string {
  let out = "";
  for (const raw of lines) {
    const line = raw.trim();
    if (line === "") continue;
    if (out === "") out = line;
    else if (out.endsWith(SOFT_HYPHEN)) out = out.slice(0, -1) + line;
    else if (/\p{L}[-‐]$/u.test(out) && /^\p{Ll}/u.test(line)) out = out.slice(0, -1) + line;
    else out += ` ${line}`;
  }
  return out.replaceAll(SOFT_HYPHEN, "");
}

/** `text` safe in HTML. */
function escapeHtml(text: string): string {
  return text.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c] ?? c);
}
