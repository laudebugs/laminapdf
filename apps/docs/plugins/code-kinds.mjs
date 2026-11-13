// The docs' code blocks coloured in the Paper look (todo Q4u): Expressive Code's Shiki plugin is off
// (astro.config.mjs), and this plugin colours each token as one of the five code kinds, by the
// rules the website's code panel uses (@laminapdf/site-kit/code-kinds), with the --paper-code-*
// tokens the "Edit and run" editor colours its own with (runner/theme.ts), so a block keeps its
// colours when the editor replaces it. Build time only.
import { InlineStyleAnnotation } from "@expressive-code/core";
import { hasGrammar, tokenize } from "@laminapdf/site-kit/code-kinds";

/**
 * A run of a line as one kind: Expressive Code's own inline style with the kind's --paper-code-*
 * colour for every theme, which @laminapdf/site-kit/inline-styles turns into a class after the
 * build, as it did Shiki's colours. A comment is italic, as in the editor.
 *
 * @param {import("@laminapdf/site-kit/code-kinds").CodeKind} kind
 * @param {number} columnStart
 * @param {number} columnEnd
 */
function kindAnnotation(kind, columnStart, columnEnd) {
  return new InlineStyleAnnotation({
    inlineRange: { columnStart, columnEnd },
    color: `var(--paper-code-${kind})`,
    italic: kind === "comment",
    renderPhase: "earliest",
  });
}

/**
 * The plugin: tokenizes each block whose language Shiki knows and annotates its coloured runs.
 * A language it doesn't know stays plain, as Shiki would leave it.
 *
 * @returns {import("@expressive-code/core").ExpressiveCodePlugin}
 */
export function codeKinds() {
  return {
    name: "lamina-code-kinds",
    hooks: {
      performSyntaxAnalysis: async ({ codeBlock }) => {
        if (!hasGrammar(codeBlock.language)) return;
        const lines = await tokenize(codeBlock.code, codeBlock.language);
        codeBlock.getLines().forEach((line, index) => {
          let column = 0;
          for (const run of lines[index] ?? []) {
            const end = column + run.text.length;
            if (run.kind !== undefined) line.addAnnotation(kindAnnotation(run.kind, column, end));
            column = end;
          }
        });
      },
    },
  };
}
