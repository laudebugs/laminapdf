// sitemap.xml: every page of the site but the 404 page, at its production address, dated by its
// sources' last commit (astro.config.mjs looks the dates up; a shallow checkout gives none).

import { routeOf } from "@laminapdf/site-kit/nav";
import type { APIRoute } from "astro";

export const GET: APIRoute = ({ site }) => {
  const urls = __SITE_PAGES__
    .filter((file) => !file.endsWith("/404.astro"))
    .map((file) => {
      const date = __SITE_LASTMOD__[file];
      const lastmod = date === undefined ? "" : `<lastmod>${date}</lastmod>`;
      return `  <url><loc>${new URL(routeOf(file), site).href}</loc>${lastmod}</url>`;
    });
  const body = `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls.join("\n")}\n</urlset>\n`;
  return new Response(body, { headers: { "Content-Type": "application/xml; charset=utf-8" } });
};
