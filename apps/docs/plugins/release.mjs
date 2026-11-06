// Which release the docs describe (todo P4, releases plan §4). A production build
// (`LAMINA_PDF_ENV=production`) is the released docs at docs.laminapdf.com: its header names the
// fixed group's version. Every other build (next.docs.laminapdf.com from `main`, the previews, a
// local build) describes changes that aren't released yet: every page says so and links to the
// released docs, and @laminapdf/site-kit marks it noindex. Install lines write `{{version}}` in their
// code, which the build fills in with the version, so they always name the one the docs describe.
import { readFile } from "node:fs/promises";
import { filesIn } from "@laminapdf/site-kit/integrations";
import { VERSION_PLACEHOLDER, withVersion } from "@laminapdf/site-kit/version";

/**
 * A Vite plugin that fills in `{{version}}` in the pages' MDX source before it compiles, in builds
 * and in the dev server. Pages write it only in code: in MDX's prose, braces start an expression.
 *
 * @param {string} version
 * @returns {{ name: string, enforce: "pre", transform: (code: string, id: string) => { code: string, map: null } | undefined }}
 */
export function versionInPages(version) {
  return {
    name: "lamina-version-in-pages",
    enforce: "pre",
    transform(code, id) {
      if (!/\.mdx?(?:$|\?)/.test(id) || !code.includes(VERSION_PLACEHOLDER)) return undefined;
      return { code: withVersion(code, version), map: null };
    },
  };
}

/** An install command and the version it names: npm, pnpm, Yarn, Bun and Deno, then Cargo. */
const INSTALL =
  /(?:npm install|pnpm add|yarn add|bun add|npm:)\s*@laminapdf\/sdk@([\w.-]*\w)|cargo add laminapdf@=?([\w.-]*\w)/g;

/**
 * What is wrong with one built page (its HTML) for the docs of `version`: a placeholder left, an
 * install line that names another version (the `alpha` tag aside), and, on a page with the site's
 * header, a released page without the version in the header or an unreleased one without the
 * notice.
 *
 * @param {string} html
 * @param {{ version: string, release: boolean }} docs
 * @returns {string[]}
 */
export function releaseProblems(html, { version, release }) {
  const problems = [];
  if (html.includes(VERSION_PLACEHOLDER)) problems.push(`${VERSION_PLACEHOLDER} left unfilled`);
  const text = html
    .replace(/<[^>]+>/g, "")
    .replaceAll("&#x3D;", "=")
    .replaceAll("&#61;", "=");
  for (const [line, npm, cargo] of text.matchAll(INSTALL)) {
    const named = npm ?? cargo;
    if (named !== version && named !== "alpha") problems.push(`"${line}" names ${named}, not ${version}`);
  }
  if (html.includes('class="site-header-row"')) {
    if (release && !html.includes(`data-docs-version="${version}"`))
      problems.push(`no version ${version} in the header`);
    if (!release && !html.includes("data-docs-unreleased")) problems.push("no notice that the page is unreleased");
  }
  return problems;
}

/**
 * Fails the build on a built page with a problem `releaseProblems` finds, and on a Markdown twin
 * with a placeholder left.
 *
 * @param {{ version: string, release: boolean }} docs
 * @returns {import("astro").AstroIntegration}
 */
export function releaseChecks(docs) {
  return {
    name: "lamina-release-checks",
    hooks: {
      "astro:build:done": async ({ dir }) => {
        const problems = [];
        for (const file of await filesIn(dir, ".html")) {
          const name = file.pathname.slice(dir.pathname.length);
          for (const problem of releaseProblems(await readFile(file, "utf8"), docs))
            problems.push(`${name}: ${problem}`);
        }
        for (const file of [...(await filesIn(dir, ".md")), ...(await filesIn(dir, ".txt"))]) {
          if ((await readFile(file, "utf8")).includes(VERSION_PLACEHOLDER)) {
            problems.push(`${file.pathname.slice(dir.pathname.length)}: ${VERSION_PLACEHOLDER} left unfilled`);
          }
        }
        if (problems.length > 0)
          throw new Error(`the docs don't match version ${docs.version}:\n${problems.join("\n")}`);
      },
    },
  };
}
