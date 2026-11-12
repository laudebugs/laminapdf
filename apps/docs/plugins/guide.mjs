// The guide offline: each sidebar area on one page (src/pages/one-page/[area].astro) and the whole
// guide as one PDF, which Lamina PDF builds from the pages' Markdown twins (agent-docs.mjs) with its
// own Markdown builder, the one behind `laminapdf build`. The PDF needs @laminapdf/wasm's text
// module, which only `cargo xtask wasm` builds: without it the build writes no PDF and the pages
// don't link one, as they show no "Edit and run" without the runner.
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { readFile, writeFile } from "node:fs/promises";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { routeOfTwin } from "./agent-docs.mjs";
import { outsideFences } from "./markdown.mjs";
import { sidebarRoutes } from "./sidebar.mjs";

const ROOT = new URL("../../../", import.meta.url);

/** Where the guide PDF is served, from the site's root. */
export const GUIDE_PDF = "/pdf/lamina-pdf-guide.pdf";

/** The one-page views' base route. */
export const ONE_PAGE = "one-page";

/** The sidebar groups the guide leaves out: the reference, which TypeDoc and rustdoc cover. */
const NOT_GUIDE = new Set(["Reference"]);

/** What the guide PDF needs built: the SDK and its text module. */
const NEEDS = ["packages/wasm/dist/lamina_pdf_text.wasm", "packages/sdk/dist/entry/node.js"];

/** The fonts the PDF is set in, from the docs' dev dependencies and the repository's Noto fixtures. */
const FONTS = {
  regular: "apps/docs/node_modules/source-sans/OTF/SourceSans3-Regular.otf",
  bold: "apps/docs/node_modules/source-sans/OTF/SourceSans3-Bold.otf",
  italic: "apps/docs/node_modules/source-sans/OTF/SourceSans3-It.otf",
  boldItalic: "apps/docs/node_modules/source-sans/OTF/SourceSans3-BoldIt.otf",
  mono: "apps/docs/node_modules/source-code-pro/OTF/SourceCodePro-Regular.otf",
  fallbacks: [
    "fixtures/fonts/NotoSans-Regular.ttf",
    "fixtures/fonts/NotoSansArabic-Regular.ttf",
    "fixtures/fonts/NotoSansDevanagari-Regular.ttf",
    "fixtures/fonts/NotoSansHebrew-Regular.ttf",
    "fixtures/fonts/NotoSansMath-Regular.ttf",
    "fixtures/fonts/NotoSansSC-Regular.otf",
  ],
};

/**
 * Whether this build can make the guide PDF: the SDK and its text module are built. Read once
 * by astro.config.mjs, so the pages link the PDF exactly when the build writes it.
 *
 * @returns {boolean}
 */
export function guidePdfAvailable() {
  return NEEDS.every((path) => existsSync(new URL(path, ROOT)));
}

/**
 * An area's route segment: its label in lower case, words joined by hyphens (`Start here` →
 * `start-here`).
 *
 * @param {string} label
 * @returns {string}
 */
export function areaSlug(label) {
  return label
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

/**
 * The guide's areas in sidebar order, each with its pages' routes: every sidebar group but the
 * reference.
 *
 * @param {{ label: string, slugs: string[] }[]} [groups] the sidebar's groups as routes
 * @returns {{ label: string, slug: string, routes: string[] }[]}
 */
export function guideAreas(groups = sidebarRoutes()) {
  return groups
    .filter((group) => !NOT_GUIDE.has(group.label))
    .map((group) => ({ label: group.label, slug: areaSlug(group.label), routes: group.slugs }));
}

/**
 * The areas that get a one-page view: those with more than one page (Troubleshooting is one page
 * already).
 *
 * @param {{ label: string, slugs: string[] }[]} [groups]
 * @returns {{ label: string, slug: string, routes: string[] }[]}
 */
export function onePageAreas(groups) {
  return guideAreas(groups).filter((area) => area.routes.length > 1);
}

/**
 * The one-page views as a Markdown list, one link per area: what `<OnePageLinks />` shows, for the
 * Markdown twins.
 *
 * @returns {string}
 */
export function onePageList() {
  return onePageAreas()
    .map((area) => `- [${area.label}](/${ONE_PAGE}/${area.slug}/)`)
    .join("\n");
}

/**
 * A twin's body: its front matter and its own `# Title` line removed.
 *
 * @param {string} twin
 * @returns {{ title: string, body: string }}
 */
export function twinBody(twin) {
  const rest = twin.replace(/^---\n[\s\S]*?\n---\n+/, "");
  const match = /^# (.+)\n+/.exec(rest);
  return match ? { title: match[1] ?? "", body: rest.slice(match[0].length) } : { title: "", body: rest };
}

/**
 * `markdown` with every ATX heading `levels` deeper, at most six, outside fenced code.
 *
 * @param {string} markdown
 * @param {number} levels
 * @returns {string}
 */
export function deeperHeadings(markdown, levels) {
  return outsideFences(markdown, (text) =>
    text.replace(/^(#{1,6}) /gm, (_, hashes) => `${"#".repeat(Math.min(6, hashes.length + levels))} `),
  );
}

/**
 * `markdown`'s links within the site as addresses on `site`, outside code, since the PDF is read
 * away from the site: a twin's (`/guides/x.md#a` → `https://docs…/guides/x/#a`), any other path
 * (`/examples/pdf/hello.pdf`), and an anchor on the page itself (`#a`), which the joined guide
 * would otherwise resolve against the first heading of that name in any page.
 *
 * @param {string} markdown
 * @param {string} site
 * @param {string} route the page's own route (`guides/x/`)
 * @returns {string}
 */
export function siteLinks(markdown, site, route) {
  return outsideFences(markdown, (text) =>
    text.replace(/\]\(((?:\/|#)[^)\s]*)\)/g, (_, href) => {
      if (href.startsWith("#")) return `](${site}/${route}${href})`;
      if (href.startsWith("//")) return `](${href})`;
      const twin = /^\/([^#]*?)\.md(#.*)?$/.exec(href);
      if (twin) return `](${site}/${routeOfTwin(`${twin[1]}.md`)}${twin[2] ?? ""})`;
      return `](${site}${href})`;
    }),
  );
}

/**
 * The guide as one Markdown document: front matter for the PDF's metadata, "About this guide", then
 * each area as a level-1 heading with its pages as level-2 headings, their own headings below.
 * U+FFFD, the replacement character the extraction pages show, is written as its code point:
 * none of the guide's fonts has a glyph for it, and the builder refuses a character it can't draw.
 *
 * @param {{ version: string, site: string, areas: { label: string, routes: string[] }[], twins: Map<string, string> }} input
 *   `twins`: route → the twin's Markdown
 * @returns {string}
 */
export function guideMarkdown({ version, site, areas, twins }) {
  const parts = [
    "---",
    `title: Lamina PDF ${version}: the guide`,
    "lang: en",
    "author: Lamina PDF",
    `subject: The Lamina PDF docs from ${site}, as one file`,
    "---",
    "",
    "# About this guide",
    "",
    `This file holds the guide from ${site} for Lamina PDF ${version}. Lamina PDF built it from the pages'`,
    "Markdown with the Markdown builder behind `laminapdf build`. The API",
    `reference is on the site: ${site}/reference/sdk/readme/ and https://docs.rs/laminapdf.`,
  ];
  for (const area of areas) {
    parts.push("", `# ${area.label}`);
    for (const route of area.routes) {
      const twin = twins.get(route);
      if (twin === undefined) throw new Error(`guide: no Markdown twin for ${route || "/"}`);
      const { title, body } = twinBody(twin);
      parts.push("", `## ${title}`, "", siteLinks(deeperHeadings(body.trim(), 1), site, route));
    }
  }
  return `${parts.join("\n")}\n`.replaceAll("\uFFFD", "U+FFFD");
}

/**
 * The one-page view's Markdown twin: the area's pages in order, each under its title.
 *
 * @param {{ label: string, routes: string[] }} area
 * @param {(route: string) => Promise<string | undefined>} twinFor
 * @returns {Promise<string>}
 */
export async function onePageBody(area, twinFor) {
  const parts = [];
  for (const route of area.routes) {
    const twin = await twinFor(route);
    if (twin === undefined) throw new Error(`one page: no Markdown twin for ${route || "/"}`);
    const { title, body } = twinBody(twin);
    parts.push(`## ${title}\n\n${deeperHeadings(body.trim(), 1)}`);
  }
  return parts.join("\n\n");
}

/**
 * A one-page view's HTML with each page's section made to stand inside the whole: its ids and
 * the references to them (`href="#…"`, `aria-controls`, `aria-labelledby`, `for`) prefixed with
 * the section's id, so the pages' repeated headings ("How it works") and tabs stay distinct, and
 * its headings one level deeper (h2 → h3, at most h6), under the page's own title. The title (the
 * section's first h2, class `one-page-title`) keeps its id and level. A page's content ends at the
 * `data-one-page-end` marker the view writes after it, not at the first `</section>`: a page's
 * footnotes are a section of their own.
 *
 * @param {string} html
 * @returns {string}
 */
export function onePageSections(html) {
  return html.replace(
    /(<section class="one-page-section" aria-labelledby="([^"]+)">[\s\S]*?<\/h2>)([\s\S]*?)(<div data-one-page-end hidden><\/div>)/g,
    (_, head, id, body, end) => {
      const prefixed = body
        .replace(
          /\b(id|for|aria-controls|aria-labelledby)="([^"]+)"/g,
          (/** @type {string} */ _m, /** @type {string} */ attr, /** @type {string} */ value) => {
            const ids = value.split(/\s+/).map((v) => `${id}-${v}`);
            return `${attr}="${ids.join(" ")}"`;
          },
        )
        .replace(
          /\bhref="#([^"]+)"/g,
          (/** @type {string} */ _m, /** @type {string} */ anchor) => `href="#${id}-${anchor}"`,
        )
        .replace(
          /<(\/?)h([2-5])\b/g,
          (/** @type {string} */ _m, /** @type {string} */ close, /** @type {string} */ level) =>
            `<${close}h${Number(level) + 1}`,
        )
        .replace(
          /\blevel-h([2-5])\b/g,
          (/** @type {string} */ _m, /** @type {string} */ level) => `level-h${Number(level) + 1}`,
        );
      return head + prefixed + end;
    },
  );
}

/**
 * The build step for the one-page views: {@link onePageSections} on each one's HTML.
 *
 * @type {import("astro").AstroIntegration}
 */
export const onePageViews = {
  name: "lamina-one-page-views",
  hooks: {
    "astro:build:done": async ({ dir }) => {
      for (const area of onePageAreas()) {
        const file = new URL(`${ONE_PAGE}/${area.slug}/index.html`, dir);
        await writeFile(file, onePageSections(await readFile(file, "utf8")));
      }
    },
  },
};

/**
 * @laminapdf/sdk, loaded by name when the PDF is built. The docs typecheck runs before the SDK
 * is built, so a typed import would fail there; a name typed as `string` gives an untyped module.
 *
 * @returns {Promise<any>}
 */
export function loadSdk() {
  /** @type {string} */
  const name = "@laminapdf/sdk";
  return import(name);
}

/**
 * The guide PDF's bytes, built by @laminapdf/sdk's Markdown builder under PDF/UA-1 with the
 * guide's fonts. Fails on a warning but a missing image: the twins name the docs' diagrams by
 * file, which the PDF leaves out and describes in text.
 *
 * @param {string} markdown
 * @returns {Promise<Uint8Array>}
 */
export async function guidePdf(markdown) {
  const { Builder, Font, init } = await loadSdk();
  await init({ text: true });
  const read = async (/** @type {string} */ path) => new Uint8Array(await readFile(new URL(path, ROOT)));
  const fonts = {
    regular: Font.fromBytes(await read(FONTS.regular)),
    bold: Font.fromBytes(await read(FONTS.bold)),
    italic: Font.fromBytes(await read(FONTS.italic)),
    boldItalic: Font.fromBytes(await read(FONTS.boldItalic)),
    mono: Font.fromBytes(await read(FONTS.mono)),
    fallbacks: await Promise.all(FONTS.fallbacks.map(async (path) => Font.fromBytes(await read(path)))),
  };
  // `free()` rather than `using`: Node 22, which the docs build runs on, has no `using`.
  const builder = new Builder({ profiles: ["PDF/UA-1"] });
  try {
    builder.markdown(markdown, {
      fonts,
      page: {
        size: "A4",
        header: { end: "{title}" },
        footer: { start: "{section}", end: "Page {page} of {pages}" },
        furnitureSize: 8,
      },
      contents: 2,
    });
    const doc = builder.finish();
    try {
      const warnings = doc.warnings.filter((/** @type {{ code: string }} */ w) => w.code !== "markdown-image-missing");
      if (warnings.length > 0)
        throw new Error(`guide PDF: ${warnings.map((/** @type {{ code: string }} */ w) => w.code).join(", ")}`);
      return doc.save();
    } finally {
      doc.free();
    }
  } finally {
    builder.free();
  }
}

/**
 * The build step, after agentDocs (it reads the twins agentDocs writes): when `available`, runs
 * scripts/guide-pdf.mjs in a Node process of its own, which writes {@link GUIDE_PDF} and checks it.
 * A process of its own because Astro's module runner has closed by the time this hook runs, and the
 * SDK has to load its WebAssembly.
 *
 * @param {{ available: boolean, site: string }} options
 * @returns {import("astro").AstroIntegration}
 */
export function guidePdfStep({ available, site }) {
  return {
    name: "lamina-guide-pdf",
    hooks: {
      "astro:build:done": ({ dir }) => {
        if (!available) {
          console.log(`docs: no guide PDF (missing ${NEEDS.join(" or ")}; run \`cargo xtask wasm\` first)`);
          return;
        }
        const script = fileURLToPath(new URL("../scripts/guide-pdf.mjs", import.meta.url));
        execFileSync(process.execPath, [script, fileURLToPath(dir), site], { stdio: "inherit" });
      },
    },
  };
}
