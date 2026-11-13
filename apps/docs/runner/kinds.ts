// The editor's code kinds (todo Q4u): which Lezer highlighting tags of TypeScript's grammar take
// each of the five kinds the docs' code blocks colour (plugins/code-kinds.mjs, by the website's
// rules in @laminapdf/site-kit/code-kinds). theme.ts colours them; colours.test.mjs tokenizes each
// runnable example both ways and fails where a token's kind differs, so a block keeps its colours
// when "Edit and run" replaces it with the editor.

import { javascript, typescriptLanguage } from "@codemirror/lang-javascript";
import { LanguageSupport } from "@codemirror/language";
import { styleTags, type Tag, tags as t } from "@lezer/highlight";

/** The five kinds, as the code blocks and the website name them. */
export type CodeKind = "keyword" | "string" | "comment" | "function" | "constant";

/** Each kind's tags; any other token is plain text. */
export const KIND_TAGS: Readonly<Record<CodeKind, readonly Tag[]>> = {
  keyword: [
    t.keyword,
    t.moduleKeyword,
    t.controlKeyword,
    t.operatorKeyword,
    t.definitionKeyword,
    t.modifier,
    t.self,
    // `=>`, which the blocks colour as a keyword (TextMate's storage.type.function.arrow).
    t.function(t.punctuation),
  ],
  string: [t.string, t.special(t.string), t.regexp, t.escape],
  comment: [t.comment, t.lineComment, t.blockComment, t.docComment],
  function: [t.function(t.variableName), t.function(t.propertyName), t.function(t.definition(t.variableName))],
  constant: [t.number, t.bool, t.null, t.atom, t.typeName, t.className, t.namespace, t.standard(t.variableName)],
};

/**
 * The editor's TypeScript: CodeMirror's, with `void` as a type coloured as the blocks colour every
 * primitive type, where the grammar would tag it as the `void` operator.
 */
export function typescript(): LanguageSupport {
  return new LanguageSupport(
    typescriptLanguage.configure({ props: [styleTags({ "VoidType/void": t.typeName })] }),
    javascript({ typescript: true }).support,
  );
}
