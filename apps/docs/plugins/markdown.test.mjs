// The MDX-to-Markdown conversion behind the pages' Markdown twins (markdown.mjs) and llms.txt's
// shape (agent-docs.mjs). Run with `node --test` (the package's `test` script).
import assert from "node:assert/strict";
import { test } from "node:test";
import { llmsTxt, routeOfTwin, twinPath } from "./agent-docs.mjs";
import { codeBlock, frontMatter, mdxToMarkdown, rewriteLinks, twin, twinHref } from "./markdown.mjs";

const components = {
  snippet: (/** @type {string} */ file) => `// ${file}\nconst a = 1;\n`,
  codeTable: () => "| Code |\n| --- |\n| `x` |",
  changelog: () => "## 1.0.0\n\n- A change.",
};

test("front matter is read and removed", () => {
  const { data, body } = frontMatter('---\ntitle: Save a PDF\ndescription: "Save it."\n---\n\nBody.\n');
  assert.deepEqual(data, { title: "Save a PDF", description: "Save it." });
  assert.equal(body, "\nBody.\n");
});

test("a snippet becomes a fenced block of the file, captioned with its path", () => {
  const out = mdxToMarkdown(
    'import Snippet from "x";\n\n<Snippet file="packages/sdk/examples/hello.ts" />\n',
    components,
  );
  assert.equal(
    out,
    "`packages/sdk/examples/hello.ts`:\n\n```ts\n// packages/sdk/examples/hello.ts\nconst a = 1;\n```\n",
  );
});

test("tabs become bold labels and their indentation goes", () => {
  const mdx = [
    '<Tabs syncKey="lang">',
    '  <TabItem label="TypeScript">',
    "    Some text.",
    "  </TabItem>",
    '  <TabItem label="Rust">',
    "    Other text.",
    "  </TabItem>",
    "</Tabs>",
  ].join("\n");
  assert.equal(mdxToMarkdown(mdx, components), "**TypeScript**\n\nSome text.\n\n**Rust**\n\nOther text.\n");
});

test("asides become block quotes with their title", () => {
  const out = mdxToMarkdown(":::note[Details]\nFirst line.\nSecond line.\n:::\n", components);
  assert.equal(out, "> **Details:** First line.\n> Second line.\n");
});

test("site links point at twins and other links stay", () => {
  assert.equal(twinHref("/guides/save-a-pdf/"), "/guides/save-a-pdf.md");
  assert.equal(twinHref("/reference/glossary/#incremental-save"), "/reference/glossary.md#incremental-save");
  assert.equal(twinHref("/"), "/index.md");
  assert.equal(twinHref("/examples/pdf/hello.pdf"), "/examples/pdf/hello.pdf");
  assert.equal(twinHref("https://example.com/"), "https://example.com/");
  assert.equal(twinHref("#anchor"), "#anchor");
});

test("links inside code are left alone", () => {
  const md = "[a](/x/) `[b](/y/)`\n\n```md\n[c](/z/)\n```\n";
  assert.equal(rewriteLinks(md, twinHref), "[a](/x.md) `[b](/y/)`\n\n```md\n[c](/z/)\n```\n");
});

test("a link the rewrite drops keeps its text", () => {
  assert.equal(
    rewriteLinks("see [ADR 0026](../decisions/0026.md)", () => undefined),
    "see ADR 0026",
  );
});

test("a code block's fence outlasts the backticks inside it", () => {
  assert.match(codeBlock("a.md", "````\nx\n````\n"), /^`a\.md`:\n\n`{5}md\n/);
});

test("the code table comes from the components", () => {
  assert.equal(mdxToMarkdown("<CodeTable />\n", components), "| Code |\n| --- |\n| `x` |\n");
});

test("the changelog component becomes the releases", () => {
  assert.equal(mdxToMarkdown("Intro.\n\n<Changelog />\n", components), "Intro.\n\n## 1.0.0\n\n- A change.\n");
});

test("a twin has front matter, the title, the description and the body", () => {
  const out = twin({ title: "T", description: "D.", version: "1.0.0", url: "https://s/t/", body: "Body.\n" });
  assert.equal(
    out,
    '---\ntitle: "T"\ndescription: "D."\nversion: "1.0.0"\nurl: "https://s/t/"\n---\n\n# T\n\nD.\n\nBody.\n',
  );
});

test("twin paths and routes are inverse", () => {
  for (const route of ["", "install/", "guides/save-a-pdf/", "reference/codes/wrong-password/"]) {
    assert.equal(routeOfTwin(twinPath(route)), route);
  }
  assert.equal(twinPath(""), "index.md");
});

test("llms.txt has an H1, a blockquote, the version and H2 link sections", () => {
  const twins = new Map([
    ["", { title: "Home", description: "Start." }],
    ["install/", { title: "Install" }],
    ["reference/sdk/readme/", { title: "API" }],
    ["reference/codes/x/", { title: "x", description: "an x" }],
  ]);
  const out = llmsTxt({
    version: "1.2.3",
    site: "https://docs.example",
    groups: [{ label: "Start here", slugs: ["", "install/"] }],
    twins,
    api: ["reference/sdk/readme/"],
    codes: ["reference/codes/x/"],
  });
  const lines = out.split("\n");
  assert.equal(lines[0], "# Lamina PDF");
  assert.match(lines[2] ?? "", /^> /);
  assert.match(out, /version 1\.2\.3/);
  assert.match(
    out,
    /^## Start here\n\n- \[Home\]\(https:\/\/docs\.example\/index\.md\): Start\.\n- \[Install\]\(https:\/\/docs\.example\/install\.md\)$/m,
  );
  assert.match(out, /^## API reference\n\n- \[API\]\(https:\/\/docs\.example\/reference\/sdk\/readme\.md\)$/m);
  assert.match(out, /^## Optional\n\n- \[x\]\(https:\/\/docs\.example\/reference\/codes\/x\.md\): an x$/m);
});

test("a link whose text is code is rewritten", () => {
  assert.equal(rewriteLinks("[`x`](/codes/x/)", twinHref), "[`x`](/codes/x.md)");
});

test("a page's own code blocks keep their imports and JSX", () => {
  const mdx = 'import Snippet from "x";\n\n```ts\nimport { init } from "@laminapdf/sdk";\n<Tabs>\n```\n';
  assert.equal(mdxToMarkdown(mdx, components), '```ts\nimport { init } from "@laminapdf/sdk";\n<Tabs>\n```\n');
});

test("a snippet's file is found whatever the attributes' order", () => {
  const out = mdxToMarkdown('<Snippet title="Hello" file="a.ts" />\n', components);
  assert.match(out, /^`a\.ts`:\n\n```ts\n/);
});
