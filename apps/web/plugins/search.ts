import { resolve } from "node:path";
import { env } from "node:process";
import { pathToFileURL } from "node:url";
import { markNoindex } from "@laminapdf/site-kit/integrations";
import { isProduction, robotsTxt } from "@laminapdf/site-kit/robots";
import type { HtmlTagDescriptor, Plugin } from "vite";
import { OPENING } from "../../site/src/data/copy.ts";

/** The production reader's one page to index: every path serves the same index.html. */
const READER_URL = "https://app.laminapdf.com/";

/** The website's share image, by its full address: a meta tag loads nothing, so the CSP is not involved. */
const SHARE_IMAGE = "https://laminapdf.com/og.png";

/** `text` from an HTML attribute or element, with the entities a page's text would use decoded. */
function decoded(text: string): string {
  return text
    .replaceAll("&quot;", '"')
    .replaceAll("&#39;", "'")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&amp;", "&");
}

/**
 * The production reader's canonical link and share tags, from index.html's own title and
 * description, so each is written once. The canonical link names `/` on every path: the
 * single-page fallback serves this page for any address, and the link keeps those addresses from
 * being indexed as copies of `/`.
 */
function shareTags(html: string): HtmlTagDescriptor[] {
  const title = html.match(/<title>([^<]+)<\/title>/)?.[1];
  const description = html.match(/<meta name="description" content="([^"]+)"/)?.[1];
  if (title === undefined || description === undefined) {
    throw new Error("index.html needs a <title> and a description for the reader's share tags");
  }
  const meta = (key: "name" | "property", name: string, content: string): HtmlTagDescriptor => ({
    tag: "meta",
    attrs: { [key]: name, content },
    injectTo: "head",
  });
  return [
    { tag: "link", attrs: { rel: "canonical", href: READER_URL }, injectTo: "head" },
    meta("property", "og:type", "website"),
    meta("property", "og:site_name", "Lamina PDF"),
    meta("property", "og:title", decoded(title)),
    meta("property", "og:description", decoded(description)),
    meta("property", "og:url", READER_URL),
    meta("property", "og:image", SHARE_IMAGE),
    meta("property", "og:image:width", "1200"),
    meta("property", "og:image:height", "630"),
    meta("property", "og:image:alt", `Lamina PDF: ${OPENING.headline}`),
    meta("name", "twitter:card", "summary_large_image"),
  ];
}

/**
 * What the reader tells search engines (ADR 0011 §1): a robots.txt that lets every crawler in, and
 * `X-Robots-Tag: noindex` on every response unless `LAMINA_PDF_ENV` is `production`; production's
 * page alone gets a canonical link and share tags (`shareTags`). Staging and
 * the previews are crawled but never indexed: a crawler reads the header only on pages robots.txt
 * lets it fetch. The production reader names no sitemap: its one page to index is `/`. The two
 * Astro sites do the same through the site kit, from the same `isProduction`.
 */
export function searchRules(environment: string | undefined = env.LAMINA_PDF_ENV): Plugin {
  let outDir = "dist";
  return {
    name: "lamina-search-rules",
    apply: "build",
    configResolved(config) {
      outDir = resolve(config.root, config.build.outDir);
    },
    transformIndexHtml: (html) => (isProduction(environment) ? shareTags(html) : []),
    // An emitted file, so it is written with the bundle, before any plugin reads dist/ in closeBundle.
    generateBundle() {
      this.emitFile({
        type: "asset",
        fileName: "robots.txt",
        source: robotsTxt({ production: isProduction(environment) }),
      });
    },
    // public/_headers is in dist/ only once the bundle is written.
    closeBundle: () => markNoindex(pathToFileURL(`${outDir}/`), environment),
  };
}
