import { resolve } from "node:path";
import { env } from "node:process";
import { pathToFileURL } from "node:url";
import { markNoindex } from "@laminapdf/site-kit/integrations";
import { isProduction, robotsTxt } from "@laminapdf/site-kit/robots";
import type { Plugin } from "vite";

/**
 * What the reader tells search engines (ADR 0011 §1): a robots.txt that lets every crawler in, and
 * `X-Robots-Tag: noindex` on every response unless `LAMINA_PDF_ENV` is `production`. Staging and
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
