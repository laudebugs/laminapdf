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
 * Whether the reader is live, which shows "Open the web app" on both sites (website plan §4). C7c
 * turns it on in the PR that deploys the reader to production.
 *
 * @type {boolean}
 */
export const APP_LAUNCHED = false;

/**
 * One link of the header, resolved for the site it renders on. `current` is `"page"` on the page it
 * names, `"true"` anywhere in the section it names (every page of the docs site for "Docs").
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
    const current =
      link.site !== here.site ? undefined : link.site === "docs" ? "true" : path === link.path ? "page" : undefined;
    return { label: link.label, href, current };
  });
}
