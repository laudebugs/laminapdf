// The Result tabs' pictures (todo B7d): every <Example result="…"> on a page names an example in
// the manifest `cargo xtask docs-previews` writes, and every picture the manifest lists exists.
// (`cargo xtask check docs` checks the manifest against the example goldens themselves.)
import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";

const root = new URL("..", import.meta.url).pathname;
const previews = join(root, "public/examples/preview");
const manifest = JSON.parse(readFileSync(join(previews, "manifest.json"), "utf8"));

/** Every .mdx page under `dir`. */
function pages(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return pages(path);
    return entry.name.endsWith(".mdx") ? [path] : [];
  });
}

/** The example stems a page's <Example> tags name in `result`. */
export function results(page) {
  return [...page.matchAll(/<Example\s[^>]*?result=(?:"([^"]+)"|\{\[([^\]]*)\]\})/g)].flatMap((m) =>
    m[1] !== undefined ? [m[1]] : [...m[2].matchAll(/"([^"]+)"/g)].map((s) => s[1]),
  );
}

test("results are read from both forms of the attribute", () => {
  const page = '<Example ts="a.ts" result="hello" />\n<Example ts="b.ts" result={["x-2b", "x-2u"]} />';
  assert.deepEqual(results(page), ["hello", "x-2b", "x-2u"]);
});

test("every page's results name an example with pictures", () => {
  const named = pages(join(root, "src/content/docs")).flatMap((path) =>
    results(readFileSync(path, "utf8")).map((stem) => ({ path, stem })),
  );
  assert.ok(named.length > 10, "the recipe pages show their results");
  for (const { path, stem } of named) {
    assert.ok(stem in manifest.examples, `${path}: ${stem} has no pictures (cargo xtask docs-previews)`);
  }
});

test("every picture the manifest lists exists", () => {
  for (const [stem, example] of Object.entries(manifest.examples)) {
    example.pages.forEach((_, index) => {
      assert.ok(existsSync(join(previews, `${stem}-${index}.png`)), `${stem}-${index}.png`);
    });
  }
});
