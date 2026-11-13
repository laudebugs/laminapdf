// The docs that ship inside the packages (package-docs.mjs): every file is stamped with the
// version, and every relative link resolves inside the folder, so an agent reading from
// node_modules or the cargo registry never follows a link to nothing. Run with `node --test`.
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdtemp, readdir, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, before, test } from "node:test";
import { pathToFileURL } from "node:url";
import { packageVersion } from "../plugins/agent-docs.mjs";
import { packageDocs, packageHref } from "./package-docs.mjs";

/** @type {string} */
let dir;

before(async () => {
  dir = await mkdtemp(join(tmpdir(), "lamina-package-docs-"));
  await packageDocs(pathToFileURL(`${dir}/`), { api: false });
});

after(async () => {
  await rm(dir, { recursive: true, force: true });
});

/** @returns {Promise<string[]>} the Markdown files, relative to the folder */
async function markdownFiles() {
  return (await readdir(dir, { recursive: true })).filter((name) => name.endsWith(".md"));
}

test("site links become links inside the folder", () => {
  assert.equal(packageHref("/guides/save-a-pdf/#incremental", 2), "../../pages/guides/save-a-pdf.md#incremental");
  assert.equal(packageHref("/", 1), "../pages/index.md");
  assert.equal(packageHref("/reference/codes/wrong-password/", 2), "../../errors/wrong-password.md");
  assert.equal(packageHref("/one-page/recipes/", 2), "https://docs.laminapdf.com/one-page/recipes/");
  assert.equal(packageHref("/examples/pdf/hello.pdf", 1), "https://docs.laminapdf.com/examples/pdf/hello.pdf");
  assert.equal(packageHref("https://example.com/", 1), "https://example.com/");
});

test("the folder has the index, the snippet, the renames and the schemas", () => {
  for (const name of ["llms.txt", "AGENTS.md", "renames.json", "errors/README.md", "pages/index.md"]) {
    assert.ok(existsSync(join(dir, name)), name);
  }
  assert.ok(existsSync(join(dir, "schemas/error.schema.json")));
  assert.ok(existsSync(join(dir, "schemas/renames.schema.json")));
});

test("every Markdown file is stamped with the package version", async () => {
  const version = await packageVersion();
  for (const name of await markdownFiles()) {
    const text = await readFile(join(dir, name), "utf8");
    assert.match(text, new RegExp(`^---\\n(?:.*\\n)*?version: "${version.replace(/\./g, "\\.")}"\\n`), name);
  }
});

/**
 * A heading's anchor as GitHub writes it (github-slugger, which `cargo xtask docs-links` checks the
 * released folder with): lower case, letters, marks, numbers, `_` and `-` kept, each space a `-`.
 *
 * @param {string} text
 * @returns {string}
 */
function slug(text) {
  return text
    .toLowerCase()
    .replace(/[^\p{L}\p{M}\p{N}\p{Pc}\- ]/gu, "")
    .replaceAll(" ", "-");
}

/**
 * The anchors of a Markdown file's headings, repeats numbered as GitHub numbers them.
 *
 * @param {string} text
 * @returns {Set<string>}
 */
function anchors(text) {
  const seen = new Map();
  const found = new Set();
  const withoutCode = text.replace(/^(`{3,})[^\n]*\n[\s\S]*?\n\1[ \t]*$/gm, "");
  for (const [, heading = ""] of withoutCode.matchAll(/^#{1,6}[ \t]+(.+?)[ \t#]*$/gm)) {
    const base = slug(heading.replace(/`([^`]*)`/g, "$1").replace(/\[([^\]]*)\]\([^)]*\)/g, "$1"));
    const count = seen.get(base) ?? 0;
    seen.set(base, count + 1);
    found.add(count === 0 ? base : `${base}-${count}`);
  }
  return found;
}

test("every relative link resolves inside the folder, anchors included", async () => {
  const files = [...(await markdownFiles()), "llms.txt"];
  const broken = [];
  for (const name of files) {
    const text = await readFile(join(dir, name), "utf8");
    const withoutCode = text.replace(/^(`{3,})[^\n]*\n[\s\S]*?\n\1[ \t]*$/gm, "").replace(/`[^`\n]+`/g, "");
    for (const [, href = ""] of withoutCode.matchAll(/\]\(([^)\s]+)\)/g)) {
      if (/^[a-z][a-z0-9+.-]*:/i.test(href) || href.startsWith("#")) continue;
      const [path = "", anchor] = href.split("#");
      const target = join(dir, name, "..", path);
      if (!existsSync(target)) broken.push(`${name}: ${href}`);
      else if (anchor !== undefined && target.endsWith(".md") && !anchors(await readFile(target, "utf8")).has(anchor)) {
        broken.push(`${name}: ${href} (no such heading)`);
      }
    }
  }
  assert.deepEqual(broken, []);
});

test("anchors follow GitHub's rule", () => {
  assert.deepEqual(
    [...anchors("# Page\n\n## React to a code\n\n## `kind`, again\n\n## Page\n")],
    ["page", "react-to-a-code", "kind-again", "page-1"],
  );
});

test("the code list and the API reference map to the folder's own copies", () => {
  assert.equal(packageHref("/reference/codes/", 2), "../../errors/README.md");
  assert.equal(packageHref("/reference/codes/#react-to-a-code", 2), "../../pages/reference/codes.md#react-to-a-code");
  assert.equal(packageHref("/reference/sdk/readme/", 1, true), "../api/README.md");
  assert.equal(packageHref("/reference/sdk/readme/", 1), "https://docs.laminapdf.com/reference/sdk/readme/");
});
