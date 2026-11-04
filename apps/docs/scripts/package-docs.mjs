// Writes the docs that ship inside a package (todo L5a): version-matched Markdown an agent can
// read from node_modules or the cargo registry instead of guessing. Run by @laminapdf/sdk's
// `prepack` (with the API reference) and by `cargo xtask publish crates` for the laminapdf crate:
//
//   node apps/docs/scripts/package-docs.mjs <out-dir> [--api]
//
// <out-dir> gets, each Markdown file stamped with the package version in its front matter:
//   llms.txt            the index (llmstxt.org), links relative to the folder
//   AGENTS.md           the snippet for a user's AGENTS.md or CLAUDE.md
//   pages/<route>.md    every page of the docs site, as on the site's Markdown twins
//   errors/<code>.md    every error and warning code's page
//   api/                @laminapdf/sdk's API reference in Markdown (TypeDoc), with --api
//   renames.json        what old names became (laminapdf.renames/1)
//   schemas/            laminapdf.error/1 and laminapdf.renames/1
import { copyFile, mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { packageVersion, SUMMARY } from "../plugins/agent-docs.mjs";
import { readerMarkdown } from "../plugins/error-pages.mjs";
import { frontMatter, mdxToMarkdown, rewriteLinks, twin } from "../plugins/markdown.mjs";
import { sidebarRoutes } from "../plugins/sidebar.mjs";

const ROOT = new URL("../../../", import.meta.url);
const CONTENT = new URL("../src/content/docs/", import.meta.url);
const SITE = "https://docs.laminapdf.com";

/**
 * A site link as a link inside the package folder, from a file `depth` folders below its root:
 * `/guides/x/#a` → `../pages/guides/x.md#a`, a code page → `../errors/<code>.md`. Links outside
 * the site stay; other site links point at the site. The API reference's start page is
 * `api/README.md` when the folder has one (`api`); its other pages, named differently by the
 * Markdown TypeDoc, point at the site.
 *
 * @param {string} href
 * @param {number} depth
 * @param {boolean} [api]
 * @returns {string}
 */
export function packageHref(href, depth, api = false) {
  const up = "../".repeat(depth);
  const match = /^\/([^#?]*?)\/?(#.*)?$/.exec(href);
  if (!match || href.startsWith("//")) return href;
  const [, path = "", anchor = ""] = match;
  const code = /^reference\/codes\/([a-z0-9-]+)$/.exec(path);
  if (code) return `${up}errors/${code[1]}.md${anchor}`;
  if (path === "reference/codes") return `${up}errors/README.md${anchor}`;
  if (path === "reference/sdk/readme" && api) return `${up}api/README.md`;
  if (path.startsWith("reference/sdk/")) return `${SITE}/${path}/${anchor}`;
  if (/\.[a-z0-9]+$/i.test(path)) return `${SITE}/${path}${anchor}`;
  return `${up}pages/${path === "" ? "index" : path}.md${anchor}`;
}

/**
 * Writes `text` to `url`, creating its folder.
 *
 * @param {URL} url
 * @param {string} text
 */
async function write(url, text) {
  await mkdir(new URL(".", url), { recursive: true });
  await writeFile(url, text);
}

/**
 * The pages, as `pages/<route>.md`, in the sidebar's order (routes not in it after).
 *
 * @param {URL} out
 * @param {string} version
 * @param {boolean} api whether the folder has the API reference
 * @returns {Promise<{ route: string, title: string, description?: string | undefined }[]>}
 */
async function writePages(out, version, api) {
  const files = new Map();
  for (const name of await readdir(CONTENT, { recursive: true })) {
    if (!/\.mdx?$/.test(name) || name.startsWith("reference/sdk/")) continue;
    const route = name.replace(/\.mdx?$/, "").replace(/(^|\/)index$/, "").toLowerCase();
    files.set(route === "" ? "" : `${route}/`, name);
  }
  const listed = sidebarRoutes().flatMap((g) => g.slugs);
  const order = [...listed.filter((r) => files.has(r)), ...[...files.keys()].filter((r) => !listed.includes(r)).sort()];
  const pages = [];
  for (const route of order) {
    const name = files.get(route);
    const source = await readFile(new URL(name, CONTENT), "utf8");
    const { data, body } = frontMatter(source);
    const path = route === "" ? "index" : route.replace(/\/$/, "");
    const depth = path.split("/").length;
    const markdown = mdxToMarkdown(body, {
      snippet: (file) => snippets.get(file) ?? "",
      codeTable: () => "See the [error and warning codes](/reference/codes/).",
    });
    const local = rewriteLinks(markdown, (href) =>
      href.startsWith("/") ? packageHref(href.replace(/\.md(#|$)/, "/$1"), depth, api) : href,
    );
    const title = data.title ?? path;
    await write(
      new URL(`pages/${path}.md`, out),
      twin({ title, description: data.description, version, url: `${SITE}/${route}`, body: local }),
    );
    pages.push({ route, title, description: data.description });
  }
  return pages;
}

/** The example files the pages show, by path from the repository root. */
const snippets = new Map();

async function readSnippets() {
  for (const name of await readdir(CONTENT, { recursive: true })) {
    if (!/\.mdx?$/.test(name)) continue;
    const source = await readFile(new URL(name, CONTENT), "utf8");
    for (const [, file = ""] of source.matchAll(/<Snippet\s+file="([^"]+)"/g)) {
      if (!snippets.has(file)) snippets.set(file, await readFile(new URL(file, ROOT), "utf8"));
    }
  }
}

/**
 * The code pages, as `errors/<code>.md`, and an index of them.
 *
 * @param {URL} out
 * @param {string} version
 * @returns {Promise<string[]>} the codes
 */
async function writeErrors(out, version) {
  const dir = new URL("docs/errors/", ROOT);
  const codes = [];
  for (const name of (await readdir(dir)).sort()) {
    if (!name.endsWith(".md") || name === "README.md") continue;
    const code = name.slice(0, -".md".length);
    const body = readerMarkdown(await readFile(new URL(name, dir), "utf8")).replace(/^# .*\n+/, "");
    const local = rewriteLinks(body, (href) => {
      if (/^[a-z0-9-]+\.md(#.*)?$/.test(href)) return href;
      if (/^[a-z][a-z0-9+.-]*:/i.test(href) || href.startsWith("#")) return href;
      return undefined;
    });
    await write(
      new URL(`errors/${name}`, out),
      twin({ title: code, version, url: `${SITE}/reference/codes/${code}/`, body: local }),
    );
    codes.push(code);
  }
  const index = codes.map((c) => `- [\`${c}\`](${c}.md)`).join("\n");
  await write(
    new URL("errors/README.md", out),
    twin({ title: "Error and warning codes", version, url: `${SITE}/reference/codes/`, body: index }),
  );
  return codes;
}

/**
 * `@laminapdf/sdk`'s API reference in Markdown (TypeDoc with typedoc-plugin-markdown), each file
 * stamped with the version.
 *
 * @param {URL} out
 * @param {string} version
 */
async function writeApi(out, version) {
  const { Application } = await import("typedoc");
  const target = new URL("api/", out);
  const app = await Application.bootstrapWithPlugins({
    entryPoints: [fileURLToPath(new URL("packages/sdk/src/index.ts", ROOT))],
    tsconfig: fileURLToPath(new URL("packages/sdk/tsconfig.build.json", ROOT)),
    plugin: ["typedoc-plugin-markdown"],
    out: fileURLToPath(target),
    readme: "none",
    skipErrorChecking: true,
    excludeInternal: true,
    excludePrivate: true,
    disableSources: true,
    hideGenerator: true,
  });
  const project = await app.convert();
  if (!project) throw new Error("package docs: TypeDoc could not read @laminapdf/sdk");
  await app.generateOutputs(project);
  for (const name of await readdir(target, { recursive: true })) {
    if (!name.endsWith(".md")) continue;
    const file = new URL(name, target);
    const text = await readFile(file, "utf8");
    await writeFile(file, `---\nversion: ${JSON.stringify(version)}\n---\n\n${text}`);
  }
}

/**
 * `llms.txt` for the folder: the same shape as the site's, links relative.
 *
 * @param {string} version
 * @param {{ route: string, title: string, description?: string | undefined }[]} pages
 * @param {boolean} api
 * @returns {string}
 */
function localLlms(version, pages, api) {
  const byRoute = new Map(pages.map((p) => [p.route, p]));
  const link = (/** @type {string} */ route) => {
    const page = byRoute.get(route);
    if (!page) return undefined;
    const path = route === "" ? "index" : route.replace(/\/$/, "");
    return `- [${page.title}](pages/${path}.md)${page.description ? `: ${page.description}` : ""}`;
  };
  const sections = sidebarRoutes().map((g) => [`## ${g.label}`, "", ...g.slugs.map(link).filter(Boolean)].join("\n"));
  return [
    "# Lamina PDF",
    "",
    `> ${SUMMARY}`,
    "",
    `These docs describe version ${version}, the version of this package. The same docs are online at ${SITE}/llms.txt.`,
    "",
    "- [Using Lamina PDF with coding agents](AGENTS.md)",
    "- [What old names became](renames.json)",
    "",
    ...sections.flatMap((s) => [s, ""]),
    ...(api ? ["## API reference", "", "- [@laminapdf/sdk](api/README.md)", ""] : []),
    "## Optional",
    "",
    "- [Error and warning codes](errors/README.md)",
    "",
  ].join("\n");
}

/**
 * Writes the package docs into `out` (emptied first).
 *
 * @param {URL} out
 * @param {{ api: boolean }} options
 */
export async function packageDocs(out, options) {
  const version = await packageVersion();
  await rm(out, { recursive: true, force: true });
  await mkdir(out, { recursive: true });
  await readSnippets();
  const pages = await writePages(out, version, options.api);
  await writeErrors(out, version);
  if (options.api) await writeApi(out, version);
  const agents = await readFile(new URL("packages/sdk/examples/AGENTS.md", ROOT), "utf8");
  await write(new URL("AGENTS.md", out), `---\nversion: ${JSON.stringify(version)}\n---\n\n${agents}`);
  await write(new URL("llms.txt", out), localLlms(version, pages, options.api));
  await copyFile(new URL("docs/renames.json", ROOT), new URL("renames.json", out));
  await mkdir(new URL("schemas/", out), { recursive: true });
  await copyFile(new URL("packages/sdk/schemas/error.schema.json", ROOT), new URL("schemas/error.schema.json", out));
  await copyFile(new URL("docs/renames.schema.json", ROOT), new URL("schemas/renames.schema.json", out));
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [dir, ...flags] = process.argv.slice(2);
  if (!dir) {
    console.error("usage: node apps/docs/scripts/package-docs.mjs <out-dir> [--api]");
    process.exit(2);
  }
  const out = new URL(`${dir.replace(/\/?$/, "/")}`, `file://${process.cwd()}/`);
  await packageDocs(out, { api: flags.includes("--api") });
}
