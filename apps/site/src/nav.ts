// The site header's links (design language §7.39): Download · Docs · Pricing · Performance ·
// Privacy, in that order, and "Open the web app". A link to a page of this site appears once the
// page exists, so the todo that writes a page adds its link with it.

/** One link of the site header. */
export interface NavLink {
  readonly label: string;
  readonly href: string;
}

/** §7.39's links: `page` is a path of this site, `href` an address on another host. */
const LINKS: readonly ({ readonly label: string; readonly page: string } | NavLink)[] = [
  { label: "Download", page: "/download/" },
  { label: "Docs", href: "https://docs.laminapdf.com/" },
  { label: "Pricing", page: "/pricing/" },
  { label: "Performance", page: "/performance/" },
  { label: "Privacy", page: "/privacy/" },
];

/** The reader's address, for "Open the web app". */
export const APP_URL = "https://app.laminapdf.com/";

/**
 * Whether the reader is live, which shows "Open the web app" in the header (website plan §4).
 * C7c turns it on in the PR that deploys the reader to production.
 */
export const APP_LAUNCHED = false as boolean;

/**
 * The path a page file under src/pages is served at: `/src/pages/privacy.astro` and
 * `/src/pages/privacy/index.astro` are both `/privacy/`.
 *
 * @param file one of `__SITE_PAGES__` (astro.config.mjs)
 */
export function routeOf(file: string): string {
  const path = file
    .replace(/^\/src\/pages/, "")
    .replace(/\.(astro|md)$/, "")
    .replace(/(^|\/)index$/, "$1");
  return path.endsWith("/") ? path : `${path}/`;
}

/**
 * The header's links, given the site's page files: every link to another host, and each link to
 * a page of this site that one of `pages` serves.
 *
 * @param pages `__SITE_PAGES__`, the files in src/pages (astro.config.mjs)
 */
export function siteNav(pages: readonly string[]): NavLink[] {
  const routes = new Set(pages.map(routeOf));
  return LINKS.flatMap((link) => {
    if ("href" in link) return [link];
    return routes.has(link.page) ? [{ label: link.label, href: link.page }] : [];
  });
}
