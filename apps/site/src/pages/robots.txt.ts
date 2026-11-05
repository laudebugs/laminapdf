// robots.txt: production lets search engines in and names the sitemap; every other build (a PR
// preview, a local build) keeps them out, as its X-Robots-Tag does (ADR 0011 §1).
import type { APIRoute } from "astro";

export const GET: APIRoute = ({ site }) => {
  const sitemap = new URL("/sitemap.xml", site).href;
  const body = __PRODUCTION__ ? `User-agent: *\nAllow: /\n\nSitemap: ${sitemap}\n` : "User-agent: *\nDisallow: /\n";
  return new Response(body, { headers: { "Content-Type": "text/plain; charset=utf-8" } });
};
