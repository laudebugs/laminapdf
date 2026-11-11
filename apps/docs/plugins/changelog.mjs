// The changelog page (src/content/docs/reference/changelog.mdx): CHANGELOG.md's releases, from
// its first `## ` heading on. Its title, its generator comment and its paragraph on the lockstep
// version stay in the repository (the page says the same in its own words), and each entry's
// leading commit hash becomes a link to that commit on GitHub.

import { REPOSITORY } from "./repository.mjs";

/**
 * CHANGELOG.md's releases as the site and the Markdown twins show them.
 *
 * @param {string} markdown
 * @returns {string}
 */
export function releaseNotes(markdown) {
  const start = markdown.search(/^## /m);
  if (start < 0) throw new Error("CHANGELOG.md has no release: no `## ` heading");
  return markdown
    .slice(start)
    .replace(
      /^([ \t]*- )([0-9a-f]{7,40}): /gm,
      (_, item, hash) => `${item}[\`${hash}\`](${REPOSITORY}/commit/${hash}): `,
    );
}

/**
 * A content loader for CHANGELOG.md (`file`, a file URL): one entry, `releases`, rendered with
 * the site's Markdown pipeline.
 *
 * @param {URL} file
 * @returns {import("astro/loaders").Loader}
 */
export function changelogLoader(file) {
  return {
    name: "lamina-changelog",
    load: async ({ store, renderMarkdown, generateDigest }) => {
      const { readFile } = await import("node:fs/promises");
      const body = await readFile(file, "utf8");
      store.clear();
      store.set({
        id: "releases",
        data: {},
        body,
        digest: generateDigest(body),
        rendered: await renderMarkdown(releaseNotes(body)),
      });
    },
  };
}
