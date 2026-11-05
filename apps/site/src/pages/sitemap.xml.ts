// sitemap.xml: every page of the site but the 404 page, at its production address.

import { routeOf } from "@laminapdf/site-kit/nav";
import type { APIRoute } from "astro";

export const GET: APIRoute = ({ site }) => {
  const urls = __SITE_PAGES__
    .filter((file) => !file.endsWith("/404.astro"))
    .map((file) => `  <url><loc>${new URL(routeOf(file), site).href}</loc></url>`);
  const body = `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls.join("\n")}\n</urlset>\n`;
  return new Response(body, { headers: { "Content-Type": "application/xml; charset=utf-8" } });
};
