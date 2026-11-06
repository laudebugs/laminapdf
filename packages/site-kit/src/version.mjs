// The version on the sites' install lines (todo P4). Pages write `{{version}}` inside code where a
// command names the fixed group's version (ADR 0017's one lockstep version), and each site fills it
// in at build time: the docs site in its pages and Markdown twins, the website in the install lines
// it copies from the docs site's install page. So no install line is ever typed with a version.

/** What a page writes where its install lines name the version. */
export const VERSION_PLACEHOLDER = "{{version}}";

/**
 * `text` with every `{{version}}` replaced by `version`.
 *
 * @param {string} text
 * @param {string} version
 * @returns {string}
 */
export function withVersion(text, version) {
  return text.replaceAll(VERSION_PLACEHOLDER, version);
}
