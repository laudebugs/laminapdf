// Starlight's docs collection: src/content/docs/ (the TypeDoc pages are generated into
// src/content/docs/reference/sdk/ at build time and gitignored). `errors` is docs/errors/: one
// page per code, rendered at /reference/codes/<code>/ by src/pages/reference/codes/[code].astro.
// `changelog` is CHANGELOG.md's releases, which the changelog page shows.

import { defineCollection } from "astro:content";
import { docsLoader } from "@astrojs/starlight/loaders";
import { docsSchema } from "@astrojs/starlight/schema";
import { changelogLoader } from "../plugins/changelog.mjs";
import { errorPagesLoader } from "../plugins/error-pages.mjs";

export const collections = {
  docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
  errors: defineCollection({ loader: errorPagesLoader(new URL("../../../docs/errors/", import.meta.url)) }),
  changelog: defineCollection({ loader: changelogLoader(new URL("../../../CHANGELOG.md", import.meta.url)) }),
};
