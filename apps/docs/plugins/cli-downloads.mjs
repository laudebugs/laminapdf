// The command-line tool's download table (cli/install.mdx) against the targets its release builds
// (dist-workspace.toml): a target added to the release fails the docs build until the page links
// its archive and checksum, and a row for a target the release dropped fails it too.

import { readFile } from "node:fs/promises";
import { VERSION_PLACEHOLDER } from "@laminapdf/site-kit/version";
import { REPOSITORY } from "./repository.mjs";

/** Where a release's files are, for the version the docs describe. */
const RELEASE = `${REPOSITORY}/releases/download/laminapdf-cli-v${VERSION_PLACEHOLDER}/`;

/** The page with the download table, from the repository's root. */
export const INSTALL_PAGE = "apps/docs/src/content/docs/cli/install.mdx";

/**
 * The `targets` list of dist-workspace.toml.
 *
 * @param {string} toml
 * @returns {string[]}
 */
export function distTargets(toml) {
  const list = /^targets\s*=\s*\[([^\]]*)\]/m.exec(toml);
  if (list === null) throw new Error("dist-workspace.toml: no `targets` list");
  return [...(list[1] ?? "").matchAll(/"([^"]+)"/g)].flatMap(([, target]) => (target === undefined ? [] : [target]));
}

/**
 * A target's archive: `.zip` on Windows, `.tar.xz` elsewhere, as dist writes them.
 *
 * @param {string} target
 * @returns {string}
 */
export function archiveName(target) {
  return `laminapdf-cli-${target}.${target.includes("-windows-") ? "zip" : "tar.xz"}`;
}

/**
 * What's wrong with the page's table: a target without its archive or checksum link, or a link
 * to a target the release doesn't build.
 *
 * @param {string} page the install page's MDX
 * @param {string[]} targets
 * @returns {string[]}
 */
export function downloadProblems(page, targets) {
  const problems = [];
  for (const target of targets) {
    const archive = RELEASE + archiveName(target);
    for (const url of [archive, `${archive}.sha256`]) {
      if (!page.includes(`(${url})`)) problems.push(`no link to ${url}`);
    }
  }
  for (const [, target] of page.matchAll(/\/laminapdf-cli-([\w-]+?)\.(?:tar\.xz|zip)\b/g)) {
    if (target !== undefined && !targets.includes(target))
      problems.push(`links ${target}, which the release doesn't build`);
  }
  return [...new Set(problems)];
}

/**
 * Fails the build when the install page's table and the release's targets differ.
 *
 * @param {URL} root the repository's root
 * @returns {import("astro").AstroIntegration}
 */
export function cliDownloadChecks(root) {
  return {
    name: "lamina-cli-downloads",
    hooks: {
      "astro:config:setup": async () => {
        const targets = distTargets(await readFile(new URL("dist-workspace.toml", root), "utf8"));
        const problems = downloadProblems(await readFile(new URL(INSTALL_PAGE, root), "utf8"), targets);
        if (problems.length > 0)
          throw new Error(
            `${INSTALL_PAGE}: the download table doesn't match dist-workspace.toml:\n${problems.join("\n")}`,
          );
      },
    },
  };
}
