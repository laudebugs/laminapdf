// Which release the docs describe (todo P4, releases plan §4). A production build
// (`LAMINA_PDF_ENV=production`) is the released docs at docs.laminapdf.com: its header names the
// fixed group's version. Every other build (next.docs.laminapdf.com from `main`, the previews, a
// local build) describes changes that aren't released yet: every page says so and links to the
// released docs, and @laminapdf/site-kit marks it noindex. Pages write `{{version}}` where they name
// the version (install lines, docs.rs links), and the build fills it in, so they always name the
// version the docs describe. `main`'s version is a published one: the version pull request merges
// only after its release is published (releases plan §3).
import { readFile } from "node:fs/promises";
import { filesIn } from "@laminapdf/site-kit/integrations";
import { VERSION_PLACEHOLDER, withVersion } from "@laminapdf/site-kit/version";

/**
 * A Vite plugin that fills in `{{version}}` in the pages' MDX source before MDX parses it, in builds
 * and in the dev server, so it may stand anywhere on an `.mdx` page, prose and links included.
 * `.md` pages (TypeDoc's, the changelog's) are rendered without Vite, so they can't use it:
 * `releaseChecks` fails the build on one left there.
 *
 * @param {string} version
 * @returns {{ name: string, enforce: "pre", transform: (code: string, id: string) => { code: string, map: null } | undefined }}
 */
export function versionInPages(version) {
  return {
    name: "lamina-version-in-pages",
    enforce: "pre",
    transform(code, id) {
      if (!/\.mdx(?:$|\?)/.test(id) || !code.includes(VERSION_PLACEHOLDER)) return undefined;
      return { code: withVersion(code, version), map: null };
    },
  };
}

/** A version, without the punctuation that may follow it in a sentence. */
const V = String.raw`[\w.-]*\w`;

/**
 * The commands and manifest lines that name a version of a package: npm, pnpm, Yarn, Bun and Deno
 * (flags such as `--save-exact` allowed before the package; Deno's `npm:` specifier too), then
 * `cargo add` and a `Cargo.toml` dependency line, with or without `=`, `^` or `~`.
 */
const INSTALL = new RegExp(
  [
    String.raw`(?:npm (?:install|i|add)|pnpm (?:add|install|i)|yarn add|bun add|deno add)(?:\s+-{1,2}[\w-]+)*\s+(?:npm:)?@laminapdf\/sdk@(${V})`,
    String.raw`npm:@laminapdf\/sdk@(${V})`,
    String.raw`cargo add laminapdf@[=^~]?(${V})`,
    String.raw`\blaminapdf = (?:\{ version = )?"[=^~]?(${V})"`,
  ].join("|"),
  "g",
);

/**
 * What is wrong with one built page (its HTML) for the docs of `version`: a placeholder left, an
 * install line that names another version (the `alpha` tag aside; not checked on a page of
 * history, the changelog), and, on a page with the site's header, a released page without the
 * version in the header or an unreleased one without the notice.
 *
 * @param {string} html
 * @param {{ version: string, release: boolean }} docs
 * @param {{ history?: boolean }} [page]
 * @returns {string[]}
 */
export function releaseProblems(html, { version, release }, { history = false } = {}) {
  const problems = [];
  if (html.includes(VERSION_PLACEHOLDER)) problems.push(`${VERSION_PLACEHOLDER} left unfilled`);
  const text = html
    .replace(/<[^>]+>/g, "")
    .replaceAll("&#x3D;", "=")
    .replaceAll("&#61;", "=")
    .replaceAll("&quot;", '"')
    .replaceAll("&#34;", '"');
  for (const [line, ...versions] of history ? [] : text.matchAll(INSTALL)) {
    const named = versions.find((value) => value !== undefined);
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
 * What is wrong with the built `_headers` file: the released docs must not send
 * `X-Robots-Tag: noindex`, and every other build must.
 *
 * @param {string} headers
 * @param {{ release: boolean }} docs
 * @returns {string[]}
 */
export function headerProblems(headers, { release }) {
  const noindex = /^\s*X-Robots-Tag: noindex$/m.test(headers);
  if (release && noindex) return ["_headers: the released docs send noindex"];
  if (!release && !noindex) return ["_headers: unreleased docs without noindex"];
  return [];
}

/**
 * Fails the build on a built page with a problem `releaseProblems` finds, on headers
 * `headerProblems` refuses, and on a Markdown twin or text file with a placeholder left.
 *
 * @param {{ version: string, release: boolean }} docs
 * @returns {import("astro").AstroIntegration}
 */
export function releaseChecks(docs) {
  return {
    name: "lamina-release-checks",
    hooks: {
      "astro:build:done": async ({ dir }) => {
        const problems = headerProblems(await readFile(new URL("_headers", dir), "utf8"), docs);
        for (const file of await filesIn(dir, ".html")) {
          const name = file.pathname.slice(dir.pathname.length);
          const history = name.startsWith("reference/changelog/");
          for (const problem of releaseProblems(await readFile(file, "utf8"), docs, { history }))
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
