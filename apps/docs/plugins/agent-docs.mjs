// Docs for agents (todo L5a), written after the build: a Markdown twin of every page
// (`/guides/save-a-pdf/` is also `/guides/save-a-pdf.md`), `llms.txt` (llmstxt.org: an H1, a
// one-paragraph summary, then sections of links to the twins) and `llms-full.txt` (every twin in
// reading order). Twins come from the pages' sources, not their HTML: the MDX pages through
// markdown.mjs, the code pages from docs/errors/, the API reference from TypeDoc's Markdown. The
// build fails when an HTML page has no twin, doesn't link its twin (`<link rel="alternate"
// type="text/markdown">`, from src/components/Head.astro), or a link in `llms.txt` has no file.

import { realpathSync } from "node:fs";
import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import { isAbsolute, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { parseBenchmarks, sectionMarkdown } from "../src/lib/benchmarks.ts";
import { releaseNotes } from "./changelog.mjs";
import { readerMarkdown, siteHref } from "./error-pages.mjs";
import { GUIDE_PDF, ONE_PAGE, onePageAreas, onePageBody, onePageList } from "./guide.mjs";
import {
  frontMatter,
  mdxToMarkdown,
  pageTwin,
  rewriteLinks,
  routeOfTwin,
  shownFiles,
  twin,
  twinHref,
  twinPath,
} from "./markdown.mjs";

const ROOT = new URL("../../../", import.meta.url);
const CONTENT = new URL("src/content/docs/", new URL("../", import.meta.url));

/** What `llms.txt` says the docs are about (its blockquote). */
export const SUMMARY =
  "Lamina PDF creates, reads, edits and saves PDF files from TypeScript and Rust: one Rust core, " +
  "run as WebAssembly in browsers, Node, Deno, Bun and Cloudflare Workers (`@laminapdf/sdk`), or " +
  "natively from the `laminapdf` crate. Positions are top-left points of the displayed page and page " +
  "indexes start at 0; every error has a stable code with its own page.";

/**
 * The version every page describes: `@laminapdf/sdk`'s (one lockstep version, ADR 0017).
 *
 * @returns {Promise<string>}
 */
export async function packageVersion() {
  const manifest = JSON.parse(await readFile(new URL("packages/sdk/package.json", ROOT), "utf8"));
  return manifest.version;
}

/**
 * The benchmarks page's numbers (`src/data/benchmarks.json`), checked as the page checks them.
 *
 * @returns {Promise<import("../src/lib/benchmarks.ts").Benchmarks>}
 */
export async function readBenchmarks() {
  return parseBenchmarks(JSON.parse(await readFile(new URL("apps/docs/src/data/benchmarks.json", ROOT), "utf8")));
}

/**
 * The error and warning codes as Markdown tables, one per group of fixtures/codes.json (as the
 * site's code table shows them).
 *
 * @returns {Promise<string>}
 */
async function codeTable() {
  const codes = JSON.parse(await readFile(new URL("fixtures/codes.json", ROOT), "utf8"));
  const schema = JSON.parse(await readFile(new URL("fixtures/codes.schema.json", ROOT), "utf8"));
  const group = new RegExp(Object.keys(schema.patternProperties)[0] ?? "^$");
  /** @param {unknown} text */
  const cell = (text) => String(text).replaceAll("|", "\\|").replaceAll("\n", " ");
  return Object.entries(codes)
    .filter(([key, value]) => group.test(key) && Array.isArray(value))
    .map(([key, value]) => {
      const entries = /** @type {{ code: string, kind: string, summary: string, hint: string }[]} */ (value);
      const words = key.replace(/([A-Z])/g, " $1").toLowerCase();
      const title = words.charAt(0).toUpperCase() + words.slice(1);
      const rows = entries.map(
        (e) =>
          `| [\`${e.code}\`](/reference/codes/${e.code}.md) | \`${e.kind}\` | ${cell(e.summary)} | ${cell(e.hint)} |`,
      );
      return [
        `## ${title}`,
        "",
        "| Code | Kind | What happened | What to do |",
        "| --- | --- | --- | --- |",
        ...rows,
      ].join("\n");
    })
    .join("\n\n");
}

/**
 * A path `readdir` returned, with forward slashes: on Windows it joins folders with `\`, and
 * routes and the depth of a page are read from `/`.
 *
 * @param {string} name
 * @returns {string}
 */
function slashes(name) {
  return name.replaceAll("\\", "/");
}

/**
 * Every HTML page the build wrote, as routes (`guides/save-a-pdf/`, `` for the home page):
 * not the 404 page, nor the redirect pages Astro writes for moved URLs. Read in the same pass:
 * the routes whose page doesn't link its Markdown twin (`/` for the home page).
 *
 * @param {URL} dir
 * @returns {Promise<{ routes: string[], unlinked: string[] }>}
 */
async function pageRoutes(dir) {
  const routes = [];
  const unlinked = [];
  for (const name of (await readdir(dir, { recursive: true })).map(slashes)) {
    if (!name.endsWith(".html") || name === "404.html") continue;
    const html = await readFile(new URL(name, dir), "utf8");
    if (/<meta\s+http-equiv="refresh"/i.test(html)) continue;
    const route = name.replace(/index\.html$/, "").replace(/\.html$/, "/");
    routes.push(route);
    const twin = pageTwin(`/${route}`);
    if (twin === undefined || !linksTwin(html, twin)) unlinked.push(route || "/");
  }
  return { routes: routes.sort(), unlinked: unlinked.sort() };
}

/**
 * Reads `url`, or returns `undefined` when there is no such file.
 *
 * @param {URL} url
 * @returns {Promise<string | undefined>}
 */
async function readMaybe(url) {
  try {
    return await readFile(url, "utf8");
  } catch {
    return undefined;
  }
}

/**
 * A route's twin, from its source; `undefined` when the route has none.
 *
 * @param {string} route
 * @param {{ version: string, site: string, codes: Map<string, { summary: string }>, table: string, changelog: string, benchmarks: import("../src/lib/benchmarks.ts").Benchmarks, guidePdf: boolean }} context
 * @returns {Promise<{ title: string, description?: string | undefined, markdown: string } | undefined>}
 */
async function twinOf(route, context) {
  const url = `${context.site}/${route}`;
  const code = /^reference\/codes\/([a-z0-9-]+)\/$/.exec(route);
  if (code) {
    const id = code[1] ?? "";
    const source = await readMaybe(new URL(`docs/errors/${id}.md`, ROOT));
    if (source === undefined) return undefined;
    const body = rewriteLinks(readerMarkdown(source).replace(/^# .*\n+/, ""), (href) => {
      const target = siteHref(href);
      return target === undefined ? undefined : twinHref(target);
    });
    const description = context.codes.get(id)?.summary;
    return { title: id, description, markdown: twin({ title: id, description, version: context.version, url, body }) };
  }
  const base = route === "" ? "index" : route.replace(/\/$/, "");
  const name = (await contentFiles()).get(base);
  if (name !== undefined) {
    const source = await readFile(new URL(name, CONTENT), "utf8");
    const { data, body } = frontMatter(source);
    const markdown = mdxToMarkdown(body, {
      snippet: snippetText,
      codeTable: () => context.table,
      changelog: () => context.changelog,
      benchmarks: (section) =>
        sectionMarkdown(context.benchmarks, /** @type {import("../src/lib/benchmarks.ts").Section} */ (section)),
      guidePdf: () =>
        context.guidePdf
          ? `Download [the whole guide as one PDF](${context.site}${GUIDE_PDF}): every section above in order, with a contents page and bookmarks.`
          : "This build of the docs has no PDF of the guide; the published site has one.",
      onePageLinks: onePageList,
    });
    const title = data.title ?? base;
    return {
      title,
      description: data.description,
      markdown: twin({ title, description: data.description, version: context.version, url, body: markdown }),
    };
  }
  return undefined;
}

/** @type {Map<string, string> | undefined} */
let contentIndex;

/**
 * The pages under src/content/docs/ by route, as Starlight slugs them: lower case, without the
 * extension or a trailing `index` (`reference/sdk/classes/Document.md` → `reference/sdk/classes/document`,
 * `recipes/index.mdx` → `recipes`, TypeDoc's `README.md` → `reference/sdk/readme`).
 *
 * @returns {Promise<Map<string, string>>}
 */
export async function contentFiles() {
  if (contentIndex === undefined) {
    contentIndex = new Map();
    for (const name of (await readdir(CONTENT, { recursive: true })).map(slashes)) {
      if (!/\.mdx?$/.test(name)) continue;
      const route = name
        .replace(/\.mdx?$/, "")
        .replace(/(^|\/)index$/, "")
        .toLowerCase();
      contentIndex.set(route === "" ? "index" : route, name);
    }
  }
  return contentIndex;
}

/** The example files pages show, read once before the twins are written. */
const snippetCache = new Map();

/**
 * The folders a page may show files from: the ones `Snippet.astro`, `Example.astro` and
 * `Transcript.astro` glob (the TypeScript and Rust examples, the reading examples' outputs and the
 * command-line transcripts). A twin never reads elsewhere.
 */
const SHOWN_FOLDERS = [
  "packages/sdk/examples/",
  "packages/viewer/examples/",
  "crates/laminapdf/examples/",
  "fixtures/extract/examples/",
  "fixtures/reveal/examples/",
  "fixtures/cli/transcripts/",
];

/**
 * `file` (a path from the repository root, as a page names it) as a URL inside one of
 * {@link SHOWN_FOLDERS}; fails for anything else: an absolute path, `..`, a percent escape (which
 * reading the URL would decode, `%2e%2e` into `..`), another folder, a symbolic link out of them.
 *
 * @param {string} file
 * @returns {URL}
 */
export function shownFileUrl(file) {
  const safe =
    file !== "" &&
    !/^([/\\]|[A-Za-z]:|file:)/.test(file) &&
    !file.includes("%") &&
    !file.split(/[/\\]/).includes("..") &&
    SHOWN_FOLDERS.some((folder) => file.startsWith(folder));
  const outside = new Error(
    `agent docs: a page shows ${file}, outside the example folders (${SHOWN_FOLDERS.join(", ")})`,
  );
  if (!safe) throw outside;
  const url = new URL(file, ROOT);
  // The check that counts: the file read, decoded and with every symbolic link followed, lies
  // inside an allowed folder (its real path too).
  const path = realpathSync(fileURLToPath(url));
  const inside = SHOWN_FOLDERS.some((folder) => {
    const rest = relative(realpathSync(fileURLToPath(new URL(folder, ROOT))), path);
    return rest !== "" && !rest.startsWith("..") && !isAbsolute(rest);
  });
  if (!inside) throw outside;
  return url;
}

/**
 * The text of an example file a page shows, read by {@link readSnippets}; fails for any other.
 * The site's twins and the package docs both read snippets through it.
 *
 * @param {string} file the path from the repository root
 * @returns {string}
 */
export function snippetText(file) {
  const text = snippetCache.get(file);
  if (text === undefined) throw new Error(`agent docs: a page shows ${file}, which wasn't read`);
  return text;
}

/**
 * Reads every file a page's `<Snippet>`s and `<Example>`s show, whatever their attributes' order.
 *
 * @returns {Promise<void>}
 */
export async function readSnippets() {
  for (const name of await readdir(CONTENT, { recursive: true })) {
    if (!/\.mdx?$/.test(name)) continue;
    const source = await readFile(new URL(name, CONTENT), "utf8");
    for (const file of shownFiles(source)) {
      if (!snippetCache.has(file)) snippetCache.set(file, await readFile(shownFileUrl(file), "utf8"));
    }
  }
}

/**
 * `llms.txt`: the H1, the summary, the version, then one section per sidebar group with a link
 * to each page's twin, the API reference, and the codes under "Optional" (llmstxt.org: links an
 * agent may skip).
 *
 * @param {{ version: string, site: string, groups: { label: string, slugs: string[] }[], twins: Map<string, { title: string, description?: string | undefined }>, api: string[], codes: string[] }} input
 * @returns {string}
 */
export function llmsTxt(input) {
  /** @param {string} route */
  const link = (route) => {
    const page = input.twins.get(route);
    const title = page?.title ?? route;
    const description = page?.description ? `: ${page.description}` : "";
    return `- [${title}](${input.site}/${twinPath(route)})${description}`;
  };
  const sections = input.groups.map((g) => [`## ${g.label}`, "", ...g.slugs.map(link)].join("\n"));
  return [
    "# Lamina PDF",
    "",
    `> ${SUMMARY}`,
    "",
    `These docs describe version ${input.version}. Every link is a Markdown page; ` +
      `[llms-full.txt](${input.site}/llms-full.txt) holds them all in one file.`,
    "",
    ...sections.flatMap((s) => [s, ""]),
    "## API reference",
    "",
    ...input.api.map(link),
    "",
    "## Optional",
    "",
    ...input.codes.map(link),
    "",
  ].join("\n");
}

/** @typedef {{ title: string, description?: string | undefined, markdown: string }} Twin */

/**
 * Writes every built page's twin next to it and returns them by route. The one-page views'
 * twins are made last, from their pages' twins (plugins/guide.mjs). Fails on a page with none.
 *
 * @param {URL} dir
 * @param {Parameters<typeof twinOf>[1]} context
 * @returns {Promise<Map<string, Twin>>}
 */
async function writeTwins(dir, context) {
  /** @type {Map<string, Twin>} */
  const twins = new Map();
  const missing = [];
  const onePageRoutes = new Map(onePageAreas().map((area) => [`${ONE_PAGE}/${area.slug}/`, area]));
  const pages = await pageRoutes(dir);
  if (pages.unlinked.length > 0) {
    throw new Error(`agent docs: no link to the Markdown twin on ${pages.unlinked.join(", ")}`);
  }
  for (const route of pages.routes) {
    if (onePageRoutes.has(route)) continue;
    const page = await twinOf(route, context);
    if (page === undefined) missing.push(route || "/");
    else twins.set(route, page);
  }
  if (missing.length > 0) throw new Error(`agent docs: no Markdown source for ${missing.join(", ")}`);
  for (const [route, area] of onePageRoutes) {
    const title = `${area.label}, on one page`;
    const description = `Every page of the docs' ${area.label} section, in order, on one page.`;
    const body = await onePageBody(area, async (page) => twins.get(page)?.markdown);
    const url = `${context.site}/${route}`;
    twins.set(route, {
      title,
      description,
      markdown: twin({ title, description, version: context.version, url, body }),
    });
  }
  for (const [route, page] of twins) {
    const out = new URL(twinPath(route), dir);
    await mkdir(new URL(".", out), { recursive: true });
    await writeFile(out, page.markdown);
  }
  return twins;
}

/**
 * The integration: twins, `llms.txt` and `llms-full.txt` after the build, in sidebar order.
 *
 * @param {() => { label: string, slugs: string[] }[]} groups the sidebar's groups as routes
 * @param {{ guidePdf?: boolean }} [options] whether the build writes the guide PDF (plugins/guide.mjs)
 * @returns {import("astro").AstroIntegration}
 */
export function agentDocs(groups, { guidePdf = false } = {}) {
  /** @type {string} */
  let site = "";
  return {
    name: "lamina-agent-docs",
    hooks: {
      "astro:config:done": ({ config }) => {
        site = String(config.site ?? "").replace(/\/$/, "");
      },
      "astro:build:done": async ({ dir }) => {
        const version = await packageVersion();
        const codesJson = JSON.parse(await readFile(new URL("fixtures/codes.json", ROOT), "utf8"));
        const codes = new Map(
          Object.values(codesJson)
            .filter(Array.isArray)
            .flat()
            .map((e) => [e.code, e]),
        );
        const changelog = releaseNotes(await readFile(new URL("CHANGELOG.md", ROOT), "utf8"));
        const benchmarks = await readBenchmarks();
        const context = { version, site, codes, table: await codeTable(), changelog, benchmarks, guidePdf };
        await readSnippets();
        const twins = await writeTwins(dir, context);
        const sidebar = groups();
        const listed = new Set(sidebar.flatMap((g) => g.slugs));
        const routes = [...twins.keys()];
        const api = routes.filter((r) => r.startsWith("reference/sdk/"));
        const codeRoutes = routes.filter((r) => r.startsWith("reference/codes/") && r !== "reference/codes/");
        // The one-page views repeat their areas' pages, so llms.txt and llms-full.txt leave them out.
        const onePage = (/** @type {string} */ r) => r.startsWith(`${ONE_PAGE}/`);
        const rest = routes.filter((r) => !listed.has(r) && !api.includes(r) && !codeRoutes.includes(r) && !onePage(r));
        const all = [...sidebar, ...(rest.length > 0 ? [{ label: "More", slugs: rest }] : [])];
        await writeFile(
          new URL("llms.txt", dir),
          llmsTxt({ version, site, groups: all, twins, api, codes: codeRoutes }),
        );
        const order = [...all.flatMap((g) => g.slugs), ...api, ...codeRoutes].filter((r) => twins.has(r));
        const full = order.map((r) => (twins.get(r)?.markdown ?? "").replace(/^---\n[\s\S]*?\n---\n\n/, ""));
        await writeFile(
          new URL("llms-full.txt", dir),
          `# Lamina PDF ${version}\n\n> ${SUMMARY}\n\n${full.join("\n\n---\n\n")}`,
        );
        const llms = await readFile(new URL("llms.txt", dir), "utf8");
        const broken = [...llms.matchAll(/\]\((https?:\/\/[^)]+)\)/g)]
          .map((m) => m[1] ?? "")
          .filter((href) => href.startsWith(site) && href.endsWith(".md"))
          .filter((href) => !twins.has(routeOfTwin(href.slice(site.length + 1))));
        if (broken.length > 0) throw new Error(`agent docs: llms.txt links to missing twins: ${broken.join(", ")}`);
      },
    },
  };
}

/**
 * Whether the page `html` links `href` as its Markdown twin, with
 * `<link rel="alternate" type="text/markdown" href="…">` in any attribute order.
 *
 * @param {string} html
 * @param {string} href
 * @returns {boolean}
 */
export function linksTwin(html, href) {
  return [...html.matchAll(/<link\b[^>]*>/gi)].some(([tag]) => {
    /** @param {string} name */
    const attr = (name) => new RegExp(`\\s${name}="([^"]*)"`, "i").exec(tag)?.[1];
    return attr("rel") === "alternate" && attr("type") === "text/markdown" && attr("href") === href;
  });
}
