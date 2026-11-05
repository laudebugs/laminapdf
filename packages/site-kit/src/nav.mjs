// The site header's links (design language §7.39), one list for the website and the docs site:
// Download · Docs · Pricing · Performance · Privacy, and "Open the web app" once the reader is live.
// A link to a website page appears on both sites once the page exists, so the todo that writes a
// page adds its link by adding the page.

/** The two sites' production origins. */
export const WEBSITE = "https://laminapdf.com";
export const DOCS = "https://docs.laminapdf.com";

/** The reader's address, for "Open the web app". */
export const APP_URL = "https://app.laminapdf.com/";

/**
 * Whether the reader is live, which shows "Open the web app" on both sites (website plan §4). It
 * turns on in the pull request that deploys the reader to production, so the link never points at
 * a reader that isn't there.
 *
 * @type {boolean}
 */
export const APP_LAUNCHED = false;

/**
 * One link of the header, resolved for the site it renders on. `current` is `"page"` on the page it
 * names, `"true"` on the pages below it (`/download/linux/` for "Download") and on every page of
 * the docs site for "Docs".
 *
 * @typedef {{ label: string, href: string, current: "page" | "true" | undefined }} NavLink
 */

/** §7.39's links, in order: each a page of the website or the docs site's home. */
const LINKS = /** @type {const} */ ([
  { label: "Download", site: "website", path: "/download/" },
  { label: "Docs", site: "docs", path: "/" },
  { label: "Pricing", site: "website", path: "/pricing/" },
  { label: "Performance", site: "website", path: "/performance/" },
  { label: "Privacy", site: "website", path: "/privacy/" },
]);

/**
 * The path a website page file is served at: `/src/pages/privacy.astro` and
 * `/src/pages/privacy/index.astro` are both `/privacy/`.
 *
 * @param {string} file one of the website's page files ({@link pageFiles} in integrations.mjs)
 * @returns {string}
 */
export function routeOf(file) {
  const path = file
    .replace(/^\/src\/pages/, "")
    .replace(/\.(astro|md)$/, "")
    .replace(/(^|\/)index$/, "$1");
  return path.endsWith("/") ? path : `${path}/`;
}

/**
 * How `link` marks the page at `path` on `site`: `"page"` on the page it names, `"true"` on the
 * pages below it (and on every docs page for "Docs"), else not at all.
 *
 * @param {(typeof LINKS)[number]} link
 * @param {"website" | "docs"} site
 * @param {string} path the page's path, ending in a slash
 * @returns {NavLink["current"]}
 */
function currentness(link, site, path) {
  if (link.site !== site) return undefined;
  if (link.site === "docs") return "true";
  if (path === link.path) return "page";
  return path.startsWith(link.path) ? "true" : undefined;
}

/**
 * The header's links on `here.site` at `here.path`: the docs link always, and each website page
 * that one of `pages` serves. A link to the other site is a full address; a link on the same site
 * is a path.
 *
 * @param {readonly string[]} pages the website's page files
 * @param {{ site: "website" | "docs", path: string }} here
 * @returns {NavLink[]}
 */
export function siteNav(pages, here) {
  const routes = new Set(pages.map(routeOf));
  const path = here.path.endsWith("/") ? here.path : `${here.path}/`;
  return LINKS.filter((link) => link.site === "docs" || routes.has(link.path)).map((link) => {
    const origin = link.site === "docs" ? DOCS : WEBSITE;
    const href = link.site === here.site ? link.path : `${origin}${link.path}`;
    return { label: link.label, href, current: currentness(link, here.site, path) };
  });
}
