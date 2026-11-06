// Tests for release.mjs: `node --test plugins/` (the docs site's `test` script).

import assert from "node:assert/strict";
import { test } from "node:test";
import { headerProblems, releaseProblems, versionInPages } from "./release.mjs";

const HEADER = '<div class="site-header-row">';
const released = { version: "0.2.0-alpha.3", release: true };
const unreleased = { version: "0.2.0-alpha.3", release: false };

test("the pages' MDX gets the version, and other files are left alone", () => {
  const plugin = versionInPages("0.2.0-alpha.3");
  const transform = plugin.transform;
  assert.equal(
    transform("npm install @laminapdf/sdk@{{version}}", "/docs/install.mdx")?.code,
    "npm install @laminapdf/sdk@0.2.0-alpha.3",
  );
  assert.equal(
    transform("`{{version}}`", "/docs/install.mdx?astroContentCollectionEntry=true")?.code,
    "`0.2.0-alpha.3`",
  );
  assert.equal(transform("const x = '{{version}}';", "/docs/src/x.ts"), undefined);
  // `.md` pages are rendered without Vite, so the plugin says nothing about them.
  assert.equal(transform("{{version}}", "/docs/README.md"), undefined);
  assert.equal(transform("no placeholder", "/docs/install.mdx"), undefined);
});

test("a released page names the version in its header; an unreleased one says it is unreleased", () => {
  assert.deepEqual(releaseProblems(`${HEADER}<a data-docs-version="0.2.0-alpha.3">`, released), []);
  assert.deepEqual(releaseProblems(`${HEADER}<a data-docs-version="0.2.0-alpha.2">`, released), [
    "no version 0.2.0-alpha.3 in the header",
  ]);
  assert.deepEqual(releaseProblems(`${HEADER}<p data-docs-unreleased>`, unreleased), []);
  assert.deepEqual(releaseProblems(HEADER, unreleased), ["no notice that the page is unreleased"]);
  // A page without the site's header (a redirect) is held only to its install lines.
  assert.deepEqual(releaseProblems("<meta http-equiv=refresh>", released), []);
});

test("install lines name the version or the alpha tag, whatever the punctuation after them", () => {
  const page = (/** @type {string} */ lines) => `${HEADER}<p data-docs-unreleased>${lines}</p>`;
  const good = [
    "<code>npm install @laminapdf/sdk@0.2.0-alpha.3</code>.",
    "<span>pnpm add</span> <span>@laminapdf/sdk@0.2.0-alpha.3</span>",
    "<code>npm:@laminapdf/sdk@0.2.0-alpha.3</code>.",
    "<code>cargo add laminapdf@=0.2.0-alpha.3</code>, then",
    "<code>npm install @laminapdf/sdk@alpha</code>",
  ];
  assert.deepEqual(releaseProblems(page(good.join(" ")), unreleased), []);
  assert.deepEqual(releaseProblems(page("<code>cargo add laminapdf@0.2.0-alpha</code>."), unreleased), [
    '"cargo add laminapdf@0.2.0-alpha" names 0.2.0-alpha, not 0.2.0-alpha.3',
  ]);
  assert.deepEqual(releaseProblems(page("<code>bun add @laminapdf/sdk@{{version}}</code>"), unreleased), [
    "{{version}} left unfilled",
  ]);
});

test("install lines with flags, other package managers and Cargo.toml lines are checked too", () => {
  const page = (/** @type {string} */ lines) => `<p>${lines}</p>`;
  const stale = [
    "<code>npm install --save-exact @laminapdf/sdk@0.2.0-alpha.2</code>",
    "<code>npm i -E @laminapdf/sdk@0.2.0-alpha.2</code>",
    "<code>deno add npm:@laminapdf/sdk@0.2.0-alpha.2</code>",
    "<code>laminapdf = &quot;=0.2.0-alpha.2&quot;</code>",
    '<code>laminapdf = { version = "^0.2.0-alpha.2" }</code>',
  ];
  for (const line of stale) {
    assert.equal(releaseProblems(page(line), unreleased).length, 1, line);
    assert.deepEqual(releaseProblems(page(line.replaceAll("0.2.0-alpha.2", "0.2.0-alpha.3")), unreleased), [], line);
  }
  // History, the changelog, may name any version.
  assert.deepEqual(releaseProblems(page(stale[0] ?? ""), unreleased, { history: true }), []);
});

test("only the released docs may be indexed", () => {
  const noindex = "/*\n  X-Robots-Tag: noindex\n  Referrer-Policy: no-referrer\n";
  const indexed = "/*\n  Referrer-Policy: no-referrer\n";
  assert.deepEqual(headerProblems(indexed, { release: true }), []);
  assert.deepEqual(headerProblems(noindex, { release: false }), []);
  assert.deepEqual(headerProblems(noindex, { release: true }), ["_headers: the released docs send noindex"]);
  assert.deepEqual(headerProblems(indexed, { release: false }), ["_headers: unreleased docs without noindex"]);
});
