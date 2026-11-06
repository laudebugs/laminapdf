// The version the sites name (todo P4). Pages write `{{version}}` where they name the fixed group's
// version (ADR 0017's one lockstep version): install lines, docs.rs links. Each site fills it in at
// build time: the docs site in its pages and Markdown twins, the website in the install lines it
// copies from the docs site's install page. So no page is ever typed with a version.

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
