// Tests for integrations.mjs: `node --test plugins/` (the docs site's `test` script).

import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { pathToFileURL } from "node:url";
import { copyWebsiteIcons, WEBSITE_ICON_FILES, WEBSITE_ICONS } from "./integrations.mjs";

const WEBSITE = new URL("../../site/", import.meta.url);

test("the docs site links the icons the website's pages link, and no others", async () => {
  const base = await readFile(new URL("src/layouts/Base.astro", WEBSITE), "utf8");
  const linked = [...base.matchAll(/<link rel="(?:icon|apple-touch-icon)" href="\/([^"]+)"/g)].map(([, file]) => file);
  assert.deepEqual([...WEBSITE_ICON_FILES].sort(), linked.sort());
  for (const { attrs } of WEBSITE_ICONS.head) {
    assert.ok(
      base.includes(`rel="${attrs.rel}" href="${attrs.href}"`),
      `Base.astro links ${attrs.href} as ${attrs.rel}`,
    );
  }
});

test("the build gets each icon, byte for byte, from the website's public folder", async () => {
  const dist = await mkdtemp(join(tmpdir(), "docs-icons-"));
  try {
    const dir = pathToFileURL(`${dist}/`);
    await copyWebsiteIcons(new URL("public/", WEBSITE), dir);
    for (const file of WEBSITE_ICON_FILES) {
      assert.deepEqual(await readFile(new URL(file, dir)), await readFile(new URL(`public/${file}`, WEBSITE)), file);
    }
  } finally {
    await rm(dist, { recursive: true, force: true });
  }
});
