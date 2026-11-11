// The command-line sessions the pages show (todo Q4a): every one is a transcript under
// fixtures/cli/transcripts/, which crates/laminapdf-cli/tests/transcripts.rs runs, never a
// `console` block typed into a page. `node --test test/` (the docs site's `test` script).

import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import { test } from "node:test";
import { mdxToMarkdown, shownFiles, transcriptFile } from "../plugins/markdown.mjs";

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

test("no page types a command-line session by hand", () => {
  const typed = pages.filter((page) => /^\s*```console\b/m.test(page.text)).map((page) => page.file);
  assert.deepEqual(typed, []);
});

test("every transcript a page shows exists", () => {
  const missing = pages.flatMap((page) =>
    [...page.text.matchAll(/<Transcript\s+name="([^"]+)"/g)]
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
