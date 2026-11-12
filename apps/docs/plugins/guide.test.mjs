// Tests for guide.mjs: `node --test plugins/` (the docs site's `test` script).

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import {
  areaSlug,
  deeperHeadings,
  guideAreas,
  guideMarkdown,
  ONE_PAGE,
  onePageAreas,
  onePageBody,
  onePageSections,
  siteLinks,
  twinBody,
} from "./guide.mjs";

const SITE = "https://docs.laminapdf.com";

const twinOf = (/** @type {string} */ title, /** @type {string} */ body) =>
  `---\ntitle: "${title}"\nversion: "1.0.0"\n---\n\n# ${title}\n\n${body}\n`;

test("an area's route segment is its label in lower case, joined by hyphens", () => {
  assert.equal(areaSlug("Read and edit PDFs"), "read-and-edit-pdfs");
  assert.equal(areaSlug("Start here"), "start-here");
});

test("the guide is every sidebar group but the reference, in sidebar order", () => {
  const areas = guideAreas([
    { label: "Start here", slugs: ["", "install/"] },
    { label: "Troubleshooting", slugs: ["troubleshooting/"] },
    { label: "Reference", slugs: ["reference/rust/"] },
  ]);
  assert.deepEqual(
    areas.map((a) => a.label),
    ["Start here", "Troubleshooting"],
  );
});

test("an area of one page gets no one-page view", () => {
  const areas = onePageAreas([
    { label: "Start here", slugs: ["", "install/"] },
    { label: "Troubleshooting", slugs: ["troubleshooting/"] },
  ]);
  assert.deepEqual(
    areas.map((a) => a.slug),
    ["start-here"],
  );
});

test("Read offline links every one-page view, and nothing else under it", async () => {
  const page = await readFile(new URL("../src/content/docs/start/offline.mdx", import.meta.url), "utf8");
  const linked = [...page.matchAll(new RegExp(`\\(/${ONE_PAGE}/([a-z-]+)/\\)`, "g"))].map((m) => m[1]);
  assert.deepEqual(
    linked,
    onePageAreas().map((a) => a.slug),
  );
});

test("a twin's body leaves out its front matter and its title", () => {
  assert.deepEqual(twinBody(twinOf("Save a PDF", "Text.\n\n## More")), {
    title: "Save a PDF",
    body: "Text.\n\n## More\n",
  });
});

test("headings go deeper, but not inside fenced code and never past six", () => {
  const markdown = "## A\n\n```md\n## not a heading\n```\n\n###### F";
  assert.equal(deeperHeadings(markdown, 1), "### A\n\n```md\n## not a heading\n```\n\n###### F");
});

test("links within the site point at the site, anchors kept, code left alone", () => {
  const markdown =
    "See [Save](/guides/save-a-pdf.md#refusals), [home](/index.md), [the PDF](/examples/pdf/hello.pdf) and [below](#more).\n\n```\n[x](/a.md)\n```";
  assert.equal(
    siteLinks(markdown, SITE, "recipes/hello/"),
    `See [Save](${SITE}/guides/save-a-pdf/#refusals), [home](${SITE}/), [the PDF](${SITE}/examples/pdf/hello.pdf) and [below](${SITE}/recipes/hello/#more).\n\n\`\`\`\n[x](/a.md)\n\`\`\``,
  );
});

test("a one-page section's ids and headings stand inside the whole page", () => {
  const html =
    '<section class="one-page-section" aria-labelledby="page-a"><h2 id="page-a" class="one-page-title">A</h2>' +
    '<div class="sl-heading-wrapper level-h2"><h2 id="how">How</h2><a href="#how">#</a></div>' +
    '<button aria-controls="tab-0 tab-1">x</button><section><h2 id="fn">Notes</h2></section>' +
    "<div data-one-page-end hidden></div></section>";
  assert.equal(
    onePageSections(html),
    '<section class="one-page-section" aria-labelledby="page-a"><h2 id="page-a" class="one-page-title">A</h2>' +
      '<div class="sl-heading-wrapper level-h3"><h3 id="page-a-how">How</h3><a href="#page-a-how">#</a></div>' +
      '<button aria-controls="page-a-tab-0 page-a-tab-1">x</button><section><h3 id="page-a-fn">Notes</h3></section>' +
      "<div data-one-page-end hidden></div></section>",
  );
});

test("the guide puts each area under a level-1 heading and its pages under level 2", () => {
  const markdown = guideMarkdown({
    version: "1.0.0",
    site: SITE,
    areas: [{ label: "Start here", routes: ["install/"] }],
    twins: new Map([["install/", twinOf("Install", "Run it.\n\n## npm")]]),
  });
  assert.match(markdown, /^---\ntitle: Lamina PDF 1\.0\.0: the guide\nlang: en\n/);
  assert.match(markdown, /\n# About this guide\n/);
  assert.match(markdown, /\n# Start here\n\n## Install\n\nRun it\.\n\n### npm\n$/);
});

test("the guide fails on a page with no twin", () => {
  assert.throws(
    () => guideMarkdown({ version: "1", site: SITE, areas: [{ label: "A", routes: ["x/"] }], twins: new Map() }),
    /no Markdown twin for x\//,
  );
});

test("a one-page twin is the area's pages in order, each under its title", async () => {
  const twins = new Map([
    ["a/", twinOf("A", "First.")],
    ["b/", twinOf("B", "Second.\n\n## Part")],
  ]);
  const body = await onePageBody({ label: "Area", routes: ["a/", "b/"] }, async (route) => twins.get(route));
  assert.equal(body, "## A\n\nFirst.\n\n## B\n\nSecond.\n\n### Part");
});
