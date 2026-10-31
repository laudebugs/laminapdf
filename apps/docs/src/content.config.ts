// Starlight's docs collection: src/content/docs/ (the TypeDoc pages are generated into
// src/content/docs/reference/pdf/ at build time and gitignored).

import { defineCollection } from "astro:content";
import { docsLoader } from "@astrojs/starlight/loaders";
import { docsSchema } from "@astrojs/starlight/schema";

export const collections = {
  docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
};
