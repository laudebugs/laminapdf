// Adapts a docs/errors/<code>.md page (written for the repository) to the site's code page
// (src/pages/reference/codes/[code].astro), on the HTML Astro rendered it to: the page's own
// `<h1>` goes (the page title shows the code), a link to another code's page points at its page
// on the site, and other relative links (ADRs, todos: repository files the site doesn't have)
// keep their text without the link, as do links to the repository on GitHub unless
// LAMINA_DOCS_GITHUB=1 (astro.config.mjs: the repository is private for now).

/** A link to `href` from an error page, rewritten for the site, or `undefined` to drop the link. */
function siteHref(href) {
  if (href.startsWith("https://github.com/laudebugs/lamina") && process.env.LAMINA_DOCS_GITHUB !== "1")
    return undefined;
  if (/^[a-z][a-z0-9+.-]*:/i.test(href) || href.startsWith("#") || href.startsWith("/")) return href;
  const code = /^([a-z0-9-]+)\.md(#.*)?$/.exec(href);
  return code ? `/reference/codes/${code[1]}/${code[2] ?? ""}` : undefined;
}

/** The page's HTML for the site. */
export function errorPageHtml(html) {
  return html
    .replace(/<h1[^>]*>[\s\S]*?<\/h1>\s*/, "")
    .replace(/<a href="([^"]*)"([^>]*)>([\s\S]*?)<\/a>/g, (_, href, rest, text) => {
      const target = siteHref(href);
      return target === undefined ? text : `<a href="${target}"${rest}>${text}</a>`;
    });
}

/**
 * A content loader for docs/errors/ (`dir`, a directory URL): one entry per `<code>.md` page,
 * rendered with the site's Markdown pipeline. It renders without a file URL, so the links
 * validator (which reads only Starlight's pages) leaves these repository pages alone; their
 * links are rewritten by {@link errorPageHtml} and checked after the build (`checkedLinks`).
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
        store.set({ id, data: {}, body, digest: generateDigest(body), rendered: await renderMarkdown(body) });
      }
    },
  };
}
