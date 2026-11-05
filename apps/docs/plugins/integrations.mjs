// The docs site's own build steps (astro.config.mjs): the example PDFs and the checks of links
// the links validator can't see. Each runs after the build, on the files in dist/. The steps both
// sites share, such as the CSP's inline-script hashes, are in @laminapdf/site-kit.
import { copyFile, mkdir, readdir, readFile } from "node:fs/promises";
import { filesIn, pageExists } from "@laminapdf/site-kit/integrations";

/**
 * Copies the examples' golden PDFs (`goldens`, a directory URL) into the site at /examples/pdf/.
 *
 * @param {URL} goldens
 * @returns {import("astro").AstroIntegration}
 */
export function examplePdfs(goldens) {
  return {
    name: "lamina-example-pdfs",
    hooks: {
      "astro:build:done": async ({ dir }) => {
        const out = new URL("examples/pdf/", dir);
        await mkdir(out, { recursive: true });
        for (const name of await readdir(goldens)) {
          if (name.endsWith(".pdf")) await copyFile(new URL(name, goldens), new URL(name, out));
        }
      },
    },
  };
}

/**
 * Fails the build on a link from any page to a file the build didn't write, for the paths the
 * links validator leaves out: the example PDFs (`/examples/pdf/…`, not pages) and the code pages
 * (`/reference/codes/<code>/`, written by src/pages/, which the validator doesn't read).
 *
 * @type {import("astro").AstroIntegration}
 */
export const checkedLinks = {
  name: "lamina-checked-links",
  hooks: {
    "astro:build:done": async ({ dir }) => {
      const broken = new Set();
      for (const file of await filesIn(dir, ".html")) {
        const html = await readFile(file, "utf8");
        for (const [, path = ""] of html.matchAll(/href="(\/(?:examples\/pdf|reference\/codes)\/[^"#?]+)/g)) {
          if (!(await pageExists(dir, path))) broken.add(path);
        }
      }
      if (broken.size > 0) throw new Error(`links to files the build didn't write: ${[...broken].sort().join(", ")}`);
    },
  },
};
