// The editor's colours match the code blocks' (todo Q4u): every runnable example tokenized by the
// editor's Lezer grammar with kinds.ts's tags, and by Shiki with the blocks' rules
// (@laminapdf/site-kit/code-kinds), gives each character the same kind, so no token changes colour
// when "Edit and run" replaces a block with the editor. Run with `node --test`.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { tokenize } from "@laminapdf/site-kit/code-kinds";
import { highlightTree, tagHighlighter } from "@lezer/highlight";
import { RUNNABLE } from "./examples.ts";
import { KIND_TAGS, typescript } from "./kinds.ts";

const ROOT = new URL("../../../", import.meta.url);

/** Lezer's highlighter, naming each tag's kind as its class. */
const highlighter = tagHighlighter(
  Object.entries(KIND_TAGS).flatMap(([kind, tags]) => tags.map((tag) => ({ tag, class: kind }))),
);

/**
 * Each character's kind in the editor: "" for plain.
 *
 * @param {string} code
 */
function editorKinds(code) {
  const kinds = Array.from({ length: code.length }, () => "");
  highlightTree(typescript().language.parser.parse(code), highlighter, (from, to, kind) => {
    kinds.fill(kind, from, to);
  });
  return kinds;
}

/**
 * Each character's kind in a code block: "" for plain.
 *
 * @param {string} code
 */
async function blockKinds(code) {
  /** @type {string[]} */
  const kinds = [];
  for (const line of await tokenize(code, "ts")) {
    for (const run of line) kinds.push(...Array.from(run.text, () => run.kind ?? ""));
    kinds.push("");
  }
  return kinds.slice(0, code.length);
}

/**
 * The runs where the two differ: each its text, the block's kind and the editor's.
 * Whitespace takes no colour, so it never counts.
 *
 * @param {string} code
 */
async function differences(code) {
  const block = await blockKinds(code);
  const editor = editorKinds(code);
  /** @type {{ text: string, block: string, editor: string }[]} */
  const out = [];
  for (let i = 0; i < code.length; i++) {
    const char = code.charAt(i);
    if (/\s/.test(char) || block[i] === editor[i]) continue;
    const last = out.at(-1);
    if (
      last !== undefined &&
      last.block === block[i] &&
      last.editor === editor[i] &&
      i > 0 &&
      !/\s/.test(code.charAt(i - 1)) &&
      block[i - 1] !== editor[i - 1]
    )
      last.text += char;
    else out.push({ text: char, block: block[i] ?? "", editor: editor[i] ?? "" });
  }
  return out;
}

for (const file of Object.keys(RUNNABLE)) {
  test(`${file} has the same colours in the editor as in its code block`, async () => {
    const code = await readFile(new URL(file, ROOT), "utf8");
    assert.deepEqual(await differences(code), []);
  });
}
