import { env } from "node:process";
import { pathToFileURL } from "node:url";
import { markNoindex } from "@laminapdf/site-kit/integrations";
import { writeRobotsTxt } from "@laminapdf/site-kit/robots";
import type { Plugin } from "vite";

/**
 * What the reader tells search engines, written into dist/ after the build (ADR 0011 §1): a
 * robots.txt that lets every crawler in, and `X-Robots-Tag: noindex` on every response unless
 * `LAMINA_PDF_ENV` is `production`. Staging and the previews are crawled but never indexed: a
 * crawler reads the header only on pages robots.txt lets it fetch. The production reader names no
 * sitemap: its one page to index is `/`. The two Astro sites do the same through the site kit.
 */
export function searchRules(): Plugin {
  let outDir = "";
  return {
    name: "lamina-search-rules",
    apply: "build",
    configResolved(config) {
      outDir = config.build.outDir;
    },
    closeBundle: () => writeSearchRules(pathToFileURL(`${outDir}/`), env.LAMINA_PDF_ENV),
  };
}

/** Writes `dir`'s robots.txt and, outside production, adds noindex to its `_headers`. */
export async function writeSearchRules(dir: URL, environment: string | undefined): Promise<void> {
  await writeRobotsTxt(dir, { production: environment === "production" });
  await markNoindex(dir, environment);
}
