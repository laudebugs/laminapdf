// A TypeDoc plugin for the @laminapdf/sdk reference (astro.config.mjs). It does three things:
// - "Defined in" links to the file on GitHub, but members inherited from TypeScript's own types
//   (Error's `message`, `cause`, `stack`) are declared in node_modules, which isn't in the
//   repository. Their source keeps its file and line and loses the link, so the member stays
//   documented without a link that would 404.
// - The types in SUMMARISED lose their members, since their description explains them.
// - The classes with a usage page (sidebar.mjs's USAGE_PAGES) end their summary with a link to it.
import { Converter } from "typedoc";
import { USAGE_PAGES } from "./sidebar.mjs";

/** Types whose members the reference leaves out, since their description explains them. */
export const SUMMARISED = ["FlowTemplate"];

/** Where TypeDoc writes the @laminapdf/sdk reference, under the site's pages (astro.config.mjs). */
export const TYPEDOC_OUTPUT = "reference/sdk";

/**
 * The git revision "Defined in" links name: the release's tag (`v<version>`) for the released docs,
 * whose code is that tag's, and `main` for every other build.
 *
 * @param {{ version: string, release: boolean }} docs
 * @returns {string}
 */
export function sourceRevision(docs) {
  return docs.release ? `v${docs.version}` : "main";
}

/**
 * Whether `fileName` is outside the repository's own files: a dependency's, in node_modules.
 *
 * @param {string} fileName
 * @returns {boolean}
 */
export function isDependencyFile(fileName) {
  return fileName.replaceAll("\\", "/").includes("/node_modules/");
}

/**
 * Leaves out the members of the types in {@link SUMMARISED}.
 *
 * @param {Pick<import("typedoc").ProjectReflection, "getChildByName" | "removeReflection">} project
 */
export function removeSummarisedMembers(project) {
  for (const name of SUMMARISED) {
    const reflection = /** @type {{ children?: import("typedoc").Reflection[] } | undefined} */ (
      project.getChildByName(name)
    );
    for (const child of [...(reflection?.children ?? [])]) project.removeReflection(child);
  }
}

/**
 * Ends the summary of each class in {@link USAGE_PAGES} with a link to its usage page. A class
 * that is missing or has no doc comment fails the build, so a rename can't drop the link unseen.
 *
 * @param {Pick<import("typedoc").ProjectReflection, "getChildByName">} project
 */
export function addUsageLinks(project) {
  for (const [name, slug] of Object.entries(USAGE_PAGES)) {
    const href = `/${slug}/`;
    const reflection = project.getChildByName(name);
    if (reflection?.comment === undefined) {
      throw new Error(`typedoc-sources: ${name} has a usage page (${href}) but no documented class to link it from`);
    }
    reflection.comment.summary.push({
      kind: "text",
      text: `\n\nCommon tasks with examples, and the mistakes to avoid: [using ${name}](${href}).`,
    });
  }
}

/**
 * TypeDoc's entry point for a plugin.
 *
 * @param {import("typedoc").Application} app
 */
export function load(app) {
  app.converter.on(Converter.EVENT_RESOLVE_END, (context) => {
    for (const reflection of Object.values(context.project.reflections)) {
      // Declarations and signatures have sources; other reflections have none.
      const { sources } = /** @type {{ sources?: import("typedoc").SourceReference[] }} */ (reflection);
      for (const source of sources ?? []) {
        if (isDependencyFile(source.fullFileName)) delete source.url;
      }
    }
    removeSummarisedMembers(context.project);
    addUsageLinks(context.project);
  });
}
