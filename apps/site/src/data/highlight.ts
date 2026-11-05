// Code colouring for the home page's code panel, at build time. Shiki reads the code with its
// CSS-variables theme, and each token's variable becomes one of five kinds, which site.css colours
// with the --paper-code-* tokens. The panel gets classes, not Shiki's inline styles, because the
// website's style-src 'self' blocks inline styles. The page ships no script for this.

import { codeToTokens, createCssVariablesTheme } from "shiki";

/** The kinds of token the panel colours; everything else is plain text. */
export type CodeKind = "keyword" | "string" | "comment" | "function" | "constant";

/** The languages the panel shows: the two examples and the command-line session. */
export type CodeLanguage = "ts" | "rust" | "shellsession";

/** A run of text, coloured as `kind` when it has one. */
export interface CodeToken {
  readonly text: string;
  readonly kind?: CodeKind;
}

const THEME = createCssVariablesTheme({ name: "paper-code", variablePrefix: "--shiki-" });

/** Shiki's theme variables, by the kind each one colours. Punctuation and parameters stay plain. */
const KINDS: Readonly<Record<string, CodeKind>> = {
  "var(--shiki-token-keyword)": "keyword",
  "var(--shiki-token-string)": "string",
  "var(--shiki-token-string-expression)": "string",
  "var(--shiki-token-comment)": "comment",
  "var(--shiki-token-function)": "function",
  "var(--shiki-token-constant)": "constant",
};

/** `code`'s lines as runs of text; neighbouring runs of the same kind are joined. */
export async function highlight(code: string, lang: CodeLanguage): Promise<CodeToken[][]> {
  const { tokens } = await codeToTokens(code, { lang, theme: THEME });
  return tokens.map((line) => {
    const runs: CodeToken[] = [];
    for (const token of line) {
      const kind = token.color === undefined ? undefined : KINDS[token.color];
      const last = runs.at(-1);
      if (last !== undefined && last.kind === kind) {
        runs[runs.length - 1] = { ...last, text: last.text + token.content };
      } else {
        runs.push(kind === undefined ? { text: token.content } : { text: token.content, kind });
      }
    }
    return runs;
  });
}
