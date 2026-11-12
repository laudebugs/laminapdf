// Tests for sidebar.mjs: `node --test plugins/` (the docs site's `test` script).

import assert from "node:assert/strict";
import { readdir } from "node:fs/promises";
import { test } from "node:test";
import { unlistedPages } from "./sidebar.mjs";

const PAGES = new URL("../src/content/docs/", import.meta.url);
const pages = (await readdir(PAGES, { recursive: true }))
  .filter((file) => /\.mdx?$/.test(file))
  .map((file) => file.split("\\").join("/"));

test("every hand-written page is in the sidebar", () => {
  assert.deepEqual(unlistedPages(pages), []);
});

test("a page the sidebar doesn't list fails the check", () => {
  assert.deepEqual(unlistedPages([...pages, "guides/new-guide.mdx"]), ["guides/new-guide.mdx"]);
});

test("index pages and the generated reference count as listed", () => {
  assert.deepEqual(unlistedPages(["index.mdx", "recipes/index.mdx", "reference/sdk/classes/Document.md"]), []);
});
