// The repository's URL, in one place: the site's GitHub links (built with LAMINA_DOCS_GITHUB=1)
// and the error pages, which drop links into it otherwise (the repository is private).
import process from "node:process";

/** The repository on GitHub. */
export const REPOSITORY = "https://github.com/laudebugs/lamina";

/** Whether this build links to the repository (`LAMINA_DOCS_GITHUB=1`); off while it is private. */
export const GITHUB = process.env.LAMINA_DOCS_GITHUB === "1";
