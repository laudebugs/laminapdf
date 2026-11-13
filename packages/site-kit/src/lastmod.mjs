// `<lastmod>` for the sitemaps: each page's date is the last commit that changed its sources, so
// search engines (and IndexNow's pings, W3e) can tell which pages changed. A shallow checkout
// would give every page its one commit's date, so a production build refuses one rather than
// write wrong dates; other builds, which name no sitemap in robots.txt, leave the dates out.
import { execFileSync } from "node:child_process";
import { relative } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Whether the git checkout at `root` is shallow (CI's default `fetch-depth: 1`).
 *
 * @param {URL} root the repository's root folder
 * @returns {boolean}
 */
export function isShallow(root) {
  const out = execFileSync("git", ["rev-parse", "--is-shallow-repository"], { cwd: fileURLToPath(root) });
  return out.toString().trim() === "true";
}

/**
 * The dates of the history at `root`, read in one `git log` pass: a function that gives the
 * committer date (ISO 8601) of the last commit that changed any of `sources` (files, or folders
 * for every file under them), or `undefined` when none is in the history (a file the build
 * generates).
 *
 * @param {URL} root the repository's root folder
 * @returns {(sources: URL[]) => string | undefined}
 */
export function commitDates(root) {
  const cwd = fileURLToPath(root);
  // Newest first: each commit is a NUL, its date, then the files it changed.
  const log = execFileSync("git", ["log", "--no-renames", "--name-only", "--format=%x00%cI"], {
    cwd,
    maxBuffer: 256 * 1024 * 1024,
  }).toString();
  /** @type {Map<string, string>} */
  const dates = new Map();
  for (const commit of log.split("\0").slice(1)) {
    const [date = "", ...files] = commit.split("\n").filter((line) => line !== "");
    for (const file of files) if (!dates.has(file)) dates.set(file, date);
  }
  return (sources) => {
    let newest;
    for (const source of sources) {
      const path = relative(cwd, fileURLToPath(source)).replaceAll("\\", "/").replace(/\/$/, "");
      for (const [file, date] of dates) {
        if (file !== path && !file.startsWith(`${path}/`)) continue;
        if (newest === undefined || Date.parse(date) > Date.parse(newest)) newest = date;
      }
    }
    return newest;
  };
}

/**
 * Whether this build may write dates, and fails when it must but can't: a production build from a
 * shallow checkout throws, any other shallow build writes none.
 *
 * @param {URL} root
 * @param {boolean} production
 * @returns {boolean}
 */
export function datesAvailable(root, production) {
  if (!isShallow(root)) return true;
  if (production) {
    throw new Error(
      "the sitemap's <lastmod> dates need the whole git history, and this checkout is shallow: " +
        "check out with fetch-depth: 0",
    );
  }
  return false;
}

/**
 * `xml`, a sitemap, with a `<lastmod>` after each `<loc>` that `dateOf` dates (and none where it
 * returns `undefined`, or where the entry already has one).
 *
 * @param {string} xml
 * @param {(loc: string) => string | undefined} dateOf
 * @returns {string}
 */
export function withLastmod(xml, dateOf) {
  return xml.replace(/<url>(\s*)<loc>([^<]+)<\/loc>(?!\s*<lastmod>)/g, (whole, space, loc) => {
    const date = dateOf(loc);
    return date === undefined ? whole : `<url>${space}<loc>${loc}</loc><lastmod>${date}</lastmod>`;
  });
}
