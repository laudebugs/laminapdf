// The docs site's own build steps (astro.config.mjs): the example PDFs, the website's icons and
// the checks of links the links validator can't see. Each runs after the build, on the files in
// dist/. The steps both sites share, such as the CSP's inline-script hashes, are in
// @laminapdf/site-kit.
import { copyFile, mkdir, readdir, readFile } from "node:fs/promises";
import { filesIn, pageExists } from "@laminapdf/site-kit/integrations";

/**
 * The website's icons, linked the way its `src/layouts/Base.astro` links them. The files live in
 * the website's `public/` (its `scripts/share-image.mjs` draws the PNGs from the SVG), and
 * `websiteIcons` copies them into the docs site, so both sites show the same icon from one set of
 * files. `favicon` is Starlight's option of that name; `head` goes into Starlight's `head`.
 */
export const WEBSITE_ICONS = {
  favicon: "/favicon.svg",
  head: [
    { tag: "link", attrs: { rel: "icon", href: "/favicon-32.png", sizes: "32x32", type: "image/png" } },
    { tag: "link", attrs: { rel: "apple-touch-icon", href: "/apple-touch-icon.png" } },
  ],
};

/** The file names `WEBSITE_ICONS` links, at the root of the site. */
export const WEBSITE_ICON_FILES = [WEBSITE_ICONS.favicon, ...WEBSITE_ICONS.head.map(({ attrs }) => attrs.href)].map(
  (href) => href.slice(1),
);

/**
 * Copies `names` from the directory `from` into the directory `to`, which it creates.
 *
 * @param {URL} from
 * @param {string[]} names
 * @param {URL} to
 */
async function copyFiles(from, names, to) {
  await mkdir(to, { recursive: true });
  for (const name of names) await copyFile(new URL(name, from), new URL(name, to));
}

/**
 * Copies the examples' golden PDFs (`goldens`, a directory URL) into the site at /examples/pdf/.
 *
 * @param {URL} goldens
 * @returns {import("astro").AstroIntegration}
 */
export function examplePdfs(goldens) {
  return {
    name: "lamina-example-pdfs",
    hooks: {
      "astro:build:done": async ({ dir }) => {
        const pdfs = (await readdir(goldens)).filter((name) => name.endsWith(".pdf"));
        await copyFiles(goldens, pdfs, new URL("examples/pdf/", dir));
      },
    },
  };
}

/**
 * Copies the website's icons (`WEBSITE_ICON_FILES`, from `websitePublic`, the website's `public/`
 * as a directory URL) to the root of the built site `dir`, where `WEBSITE_ICONS` links them.
 *
 * @param {URL} websitePublic
 * @param {URL} dir
 */
export async function copyWebsiteIcons(websitePublic, dir) {
  await copyFiles(websitePublic, WEBSITE_ICON_FILES, dir);
}

/**
 * Copies the website's icons into the site after the build (`copyWebsiteIcons`).
 *
 * @param {URL} websitePublic
 * @returns {import("astro").AstroIntegration}
 */
export function websiteIcons(websitePublic) {
  return {
    name: "lamina-website-icons",
    hooks: {
      "astro:build:done": ({ dir }) => copyWebsiteIcons(websitePublic, dir),
    },
  };
}

/**
 * Fails the build on a link from any page to a file the build didn't write, for the paths the
 * links validator leaves out: the example PDFs (`/examples/pdf/…`, not pages), the guide PDF
 * (`/pdf/…`), the one-page views (`/one-page/<area>/`) and the code pages
 * (`/reference/codes/<code>/`, written by src/pages/, which the validator doesn't read). Each
 * page's link to its Markdown twin (`….md`) is checked by agent-docs.mjs, which writes the twins.
 *
 * @type {import("astro").AstroIntegration}
 */
export const checkedLinks = {
  name: "lamina-checked-links",
  hooks: {
    "astro:build:done": async ({ dir }) => {
      const broken = new Set();
      for (const file of await filesIn(dir, ".html")) {
        const html = await readFile(file, "utf8");
        for (const [, path = ""] of html.matchAll(
          /href="(\/(?:examples\/pdf|pdf|one-page|reference\/codes)\/[^"#?]+)/g,
        )) {
          if (path.endsWith(".md")) continue;
          if (!(await pageExists(dir, path))) broken.add(path);
        }
      }
      if (broken.size > 0) throw new Error(`links to files the build didn't write: ${[...broken].sort().join(", ")}`);
    },
  },
};

/**
 * What a built page lacks of the links into the repository every page carries: the header's link
 * to it and "Edit page" (`editPrefix` and the rest of the file's path).
 *
 * @param {string} html
 * @param {string} repository
 * @param {string} editPrefix
 * @returns {string[]}
 */
export function missingRepositoryLinks(html, repository, editPrefix) {
  const missing = [];
  if (!html.includes(`href="${repository}"`)) missing.push("the header's link to the repository");
  if (!html.includes(`href="${editPrefix}`)) missing.push('"Edit page"');
  return missing;
}

/**
 * Fails the build when a written page (`page`, a path under the site) lacks the repository's links:
 * the docs link to their source on GitHub in every build.
 *
 * @param {string} page
 * @param {string} repository
 * @param {string} editPrefix
 * @returns {import("astro").AstroIntegration}
 */
export function repositoryLinks(page, repository, editPrefix) {
  return {
    name: "lamina-repository-links",
    hooks: {
      "astro:build:done": async ({ dir }) => {
        const html = await readFile(new URL(`.${page}index.html`, dir), "utf8");
        const missing = missingRepositoryLinks(html, repository, editPrefix);
        if (missing.length > 0) throw new Error(`${page} has no ${missing.join(" and no ")}`);
      },
    },
  };
}

/**
 * The site path of each code's `docsUrl` (ADR 0026 §6: base + code + suffix), from `codes.json`'s
 * text: every item of its groups that has a `code`. The base must be on `site`, the docs site's
 * address, so every `docsUrl` the library hands out is a page this build writes.
 *
 * @param {string} codesJson
 * @param {string} site
 * @returns {string[]}
 */
export function codePagePaths(codesJson, site) {
  const codes = JSON.parse(codesJson);
  const { base, suffix } = codes.docsUrl;
  if (!base.startsWith(`${site}/`)) throw new Error(`codes.json's docsUrl base ${base} is not on ${site}`);
  const path = base.slice(site.length);
  return Object.values(codes)
    .filter(Array.isArray)
    .flat()
    .filter((item) => typeof item?.code === "string")
    .map((item) => `${path}${item.code}${suffix}`);
}

/**
 * Fails the build when a code in `codes.json` has no page at its `docsUrl`, which every error and
 * warning the library reports carries.
 *
 * @param {URL} codesJson
 * @param {string} site
 * @returns {import("astro").AstroIntegration}
 */
export function codePages(codesJson, site) {
  return {
    name: "lamina-code-pages",
    hooks: {
      "astro:build:done": async ({ dir }) => {
        const missing = [];
        for (const path of codePagePaths(await readFile(codesJson, "utf8"), site)) {
          if (!(await pageExists(dir, path))) missing.push(path);
        }
        if (missing.length > 0) throw new Error(`codes with no page at their docsUrl: ${missing.join(", ")}`);
      },
    },
  };
}
