// The command-line sessions the pages show (todo Q4a): every one is a transcript under
// fixtures/cli/transcripts/, which crates/laminapdf-cli/tests/transcripts.rs runs, never a
// `console` block typed into a page. `node --test test/` (the docs site's `test` script).

import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import { test } from "node:test";
import { mdxToMarkdown, shownFiles, TRANSCRIPT, transcriptFile } from "../plugins/markdown.mjs";

const ROOT = new URL("../../../", import.meta.url);
const PAGES = new URL("src/content/docs/", new URL("../", import.meta.url));
const pages = await Promise.all(
  (await readdir(PAGES, { recursive: true }))
    .filter((file) => file.endsWith(".mdx"))
    .map(async (file) => ({ file, text: await readFile(new URL(file, PAGES), "utf8") })),
);
const transcripts = new Set(
  (await readdir(new URL("fixtures/cli/transcripts/", ROOT)))
    .filter((file) => file.endsWith(".console"))
    .map((file) => file.replace(/\.console$/, "")),
);

/**
 * Whether `text` has a terminal session typed by hand: a `console` fence, or a shell fence with a
 * `$ ` prompt in it (a fence of commands alone, with no prompt, is fine).
 *
 * @param {string} text
 * @returns {boolean}
 */
export function typesASession(text) {
  for (const [, lang = "", body = ""] of text.matchAll(/^\s*(?:```|~~~)(\w*)[^\n]*\n([\s\S]*?)^\s*(?:```|~~~)/gm)) {
    if (lang === "console") return true;
    if (["sh", "bash", "shell", "zsh"].includes(lang) && /^\s*\$ /m.test(body)) return true;
  }
  return /<Code\b[^>]*\blang=["']console["']/.test(text);
}

test("no page types a command-line session by hand", () => {
  const typed = pages.filter((page) => typesASession(page.text)).map((page) => page.file);
  assert.deepEqual(typed, []);
});

test("a console fence, or a shell fence with a prompt, counts as a typed session", () => {
  assert.equal(typesASession("```console\n$ laminapdf --version\n```\n"), true);
  assert.equal(typesASession("~~~console\nlaminapdf 1\n~~~\n"), true);
  assert.equal(typesASession("```sh\n$ laminapdf info a.pdf\na.pdf  PDF 1.7\n```\n"), true);
  assert.equal(typesASession("```sh\nlaminapdf info a.pdf\n```\n"), false);
  assert.equal(typesASession('<Code code="$ laminapdf info a.pdf" lang="console" />'), true);
});

test("every Transcript tag is one the Markdown twins replace", () => {
  const odd = pages.flatMap((page) =>
    (page.text.match(/<Transcript\b[^>]*>/g) ?? [])
      .filter((tag) => [...tag.matchAll(TRANSCRIPT)].length !== 1)
      .map((tag) => `${page.file}: ${tag}`),
  );
  assert.deepEqual(odd, []);
});

test("every transcript a page shows exists", () => {
  const missing = pages.flatMap((page) =>
    [...page.text.matchAll(TRANSCRIPT)]
      .map((m) => m[1] ?? "")
      .filter((name) => !transcripts.has(name))
      .map((name) => `${page.file}: ${name}`),
  );
  assert.deepEqual(missing, []);
});

test("a twin shows a transcript as a console block", () => {
  const body = 'Run it:\n\n<Transcript name="version" />\n';
  assert.deepEqual(shownFiles(body), [transcriptFile("version")]);
  const markdown = mdxToMarkdown(body, {
    snippet: (file) => (file === transcriptFile("version") ? "$ laminapdf --version\nlaminapdf {{version}}\n" : ""),
    codeTable: () => "",
    changelog: () => "",
  });
  assert.equal(markdown, "Run it:\n\n```console\n$ laminapdf --version\nlaminapdf {{version}}\n```\n");
});

test("a Markdown file a page shows by name is the file the transcripts read", async () => {
  const notes = await readFile(new URL("fixtures/cli/notes.md", ROOT), "utf8");
  const shown = pages.flatMap((page) =>
    [...page.text.matchAll(/^```md title="notes\.md"\n([\s\S]*?)^```/gm)].map((m) => ({ file: page.file, text: m[1] })),
  );
  assert.ok(shown.length > 0);
  for (const block of shown) assert.equal(block.text, notes, block.file);
});
