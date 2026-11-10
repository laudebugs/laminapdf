import pkg from "../package.json" with { type: "json" };

/**
 * The reader's version, which Settings › About shows: the release's when the build names one
 * (web-deploy.yml sets LAMINA_PDF_WEB_VERSION from its web-v tag), else package.json's base version.
 * The build and the e2e test that checks Settings › About both call this, so they can't disagree.
 */
export function appVersion(env: Readonly<Record<string, string | undefined>>): string {
  const release = env.LAMINA_PDF_WEB_VERSION;
  if (release === undefined || release === "") return pkg.version;
  if (!/^\d+\.\d+\.\d+$/.test(release)) throw new Error(`LAMINA_PDF_WEB_VERSION ${release} is not MAJOR.MINOR.PATCH`);
  return release;
}
