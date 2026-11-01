// Adapts a docs/errors/<code>.md page (written for the repository) to the site's code page
// (src/pages/reference/codes/[code].astro), on the HTML Astro rendered it to: the page's own
// `<h1>` goes (the page title shows the code), a link to another code's page points at its page
// on the site, and other relative links (ADRs, todos: repository files the site doesn't have)
// keep their text without the link, as do links to the repository on GitHub unless
// LAMINA_DOCS_GITHUB=1 (astro.config.mjs: the repository is private for now). A page's last
// section, "For contributors" (decision records and todos), is left out.

import { GITHUB, REPOSITORY } from "./repository.mjs";

/**
 * A page's Markdown without its "For contributors" section, which is always last: everything
 * from the last such heading outside a code fence. Trailing spaces and CRLF line ends are fine.
 *
 * @param {string} markdown
 * @returns {string}
 */
function readerMarkdown(markdown) {
  const lines = markdown.split("\n");
  /** @type {string | undefined} */
  let fence;
  let cut = -1;
  lines.forEach((line, index) => {
    const marker = /^ {0,3}(`{3,}|~{3,})(.*)$/.exec(line);
    if (marker) {
      const [, run = "", after = ""] = marker;
      // A fence closes on the same character, at least as long, with nothing after it.
      if (fence === undefined) fence = run;
      else if (run[0] === fence[0] && run.length >= fence.length && after.trim() === "") fence = undefined;
      return;
    }
    if (fence === undefined && /^## For contributors[ \t]*\r?$/.test(line)) cut = index;
  });
  return cut < 0 ? markdown : lines.slice(0, cut).join("\n");
}

/**
 * A link to `href` from an error page, rewritten for the site, or `undefined` to drop the link.
 *
 * @param {string} href
 * @returns {string | undefined}
 */
function siteHref(href) {
  if (href.startsWith(REPOSITORY) && !GITHUB) return undefined;
  if (/^[a-z][a-z0-9+.-]*:/i.test(href) || href.startsWith("#") || href.startsWith("/")) return href;
  const code = /^([a-z0-9-]+)\.md(#.*)?$/.exec(href);
  return code ? `/reference/codes/${code[1]}/${code[2] ?? ""}` : undefined;
}

/**
 * The page's HTML for the site.
 *
 * @param {string} html
 * @returns {string}
 */
export function errorPageHtml(html) {
  return html.replace(/<h1[^>]*>[\s\S]*?<\/h1>\s*/, "").replace(/<a href="([^"]*)"([^>]*)>([\s\S]*?)<\/a>/g, link);
}

/**
 * One `<a>` of an error page, its target rewritten for the site, or its text alone.
 *
 * @param {string} _ the whole match
 * @param {string} href
 * @param {string} rest the tag's other attributes
 * @param {string} text
 * @returns {string}
 */
function link(_, href, rest, text) {
  const target = siteHref(href);
  return target === undefined ? text : `<a href="${target}"${rest}>${text}</a>`;
}

/**
 * A content loader for docs/errors/ (`dir`, a directory URL): one entry per `<code>.md` page,
 * rendered with the site's Markdown pipeline. It renders without a file URL, so the links
 * validator (which reads only Starlight's pages) leaves these repository pages alone; their
 * links are rewritten by {@link errorPageHtml} and checked after the build (`checkedLinks`).
 *
 * @param {URL} dir
 * @returns {import("astro/loaders").Loader}
 */
export function errorPagesLoader(dir) {
  return {
    name: "lamina-error-pages",
    load: async ({ store, renderMarkdown, generateDigest }) => {
      const { readdir, readFile } = await import("node:fs/promises");
      store.clear();
      for (const name of (await readdir(dir)).sort()) {
        if (!name.endsWith(".md") || name === "README.md") continue;
        const body = await readFile(new URL(name, dir), "utf8");
        const id = name.slice(0, -".md".length);
        store.set({
          id,
          data: {},
          body,
          digest: generateDigest(body),
          rendered: await renderMarkdown(readerMarkdown(body)),
        });
      }
    },
  };
}
