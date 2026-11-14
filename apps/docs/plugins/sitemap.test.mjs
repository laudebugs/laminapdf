// Tests for sitemap.mjs: `node --test plugins/` (the docs site's `test` script).

import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { test } from "node:test";
import { contentFiles } from "./agent-docs.mjs";
import { routeSources } from "./sitemap.mjs";

const ROOT = new URL("../../../", import.meta.url);

test("a page is dated by its own file, a code page by its error page, the API reference by the SDK", async () => {
  const content = await contentFiles();
  /** @param {string} route */
  const sources = (route) => routeSources(route, content).map((url) => url.href.slice(ROOT.href.length));
  assert.deepEqual(sources(""), ["apps/docs/src/content/docs/index.mdx"]);
  assert.deepEqual(sources("guides/flowing-pages/"), ["apps/docs/src/content/docs/guides/flowing-pages.mdx"]);
  assert.deepEqual(sources("reference/codes/wrong-password/"), ["docs/errors/wrong-password.md"]);
  assert.deepEqual(sources("reference/sdk/classes/document/"), ["packages/sdk/src/"]);
  assert.deepEqual(sources("one-page/recipes/"), []);
  for (const route of ["", "guides/flowing-pages/", "reference/codes/wrong-password/"]) {
    for (const url of routeSources(route, content)) assert.ok(existsSync(url), `${url.href} exists`);
  }
});
