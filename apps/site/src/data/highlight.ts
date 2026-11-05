// Code colouring for the home page's code panel, at build time. Shiki tokenizes the code, and each
// token's TextMate scopes give it one of five kinds, which site.css colours with the
// --paper-code-* tokens. The panel gets classes, not Shiki's inline styles, because the website's
// style-src 'self' blocks inline styles. The page ships no script for this.

import { codeToTokens, createCssVariablesTheme } from "shiki";

/** The kinds of token the panel colours; everything else is plain text. */
export const CODE_KINDS = ["keyword", "string", "comment", "function", "constant"] as const;

/** One of the kinds the panel colours. */
export type CodeKind = (typeof CODE_KINDS)[number];

/** The languages the panel shows: the two examples, and a console of `$ ` commands in bash. */
export type CodeLanguage = "ts" | "rust" | "console";

/** A run of text, coloured as `kind` when it has one. */
export interface CodeToken {
  readonly text: string;
  readonly kind?: CodeKind;
}

/** A console line's prompt, coloured as a comment: it is not part of the command. */
const PROMPT = "$ ";

/** Shiki needs a theme to tokenize; the kinds come from the scopes, not from its colours. */
const THEME = createCssVariablesTheme({ name: "paper-code", variablePrefix: "--shiki-" });

/**
 * Scope prefixes and the kind each gives, `null` for plain, in the order they are tried. A token
 * inside a comment is a comment. Otherwise its innermost scope that a rule matches decides, then
 * the next scope out: operators, punctuation, variables and namespaces stay plain; types share the
 * constants' colour; a shell command is a function, its options are constants and its arguments
 * are plain.
 */
const RULES: readonly (readonly [prefix: string, kind: CodeKind | null])[] = [
  ["keyword.operator.new", "keyword"],
  ["keyword.operator.expression", "keyword"],
  ["keyword.operator", null],
  ["keyword", "keyword"],
  ["storage", "keyword"],
  ["punctuation.definition.string", "string"],
  ["punctuation.separator.dot.decimal", "constant"],
  ["punctuation", null],
  ["string.unquoted.argument", null],
  ["string", "string"],
  ["constant.character.escape", "string"],
  ["entity.name.function", "function"],
  ["support.function", "function"],
  ["entity.name.command", "function"],
  ["entity.name.type", "constant"],
  ["entity.name.class", "constant"],
  ["support.type", "constant"],
  ["support.class", "constant"],
  ["constant", "constant"],
  ["variable", null],
  ["entity.name.namespace", null],
];

/** Whether `scope` is `prefix` or one of its sub-scopes. */
function within(scope: string, prefix: string): boolean {
  return scope === prefix || scope.startsWith(`${prefix}.`);
}

/** The kind of a token whose scopes are `scopes`, outermost first; `undefined` for plain. */
function kindOf(scopes: readonly string[]): CodeKind | undefined {
  if (scopes.some((scope) => within(scope, "comment"))) return "comment";
  for (const scope of [...scopes].reverse()) {
    const rule = RULES.find(([prefix]) => within(scope, prefix));
    if (rule !== undefined) return rule[1] ?? undefined;
  }
  return undefined;
}

/** `runs` with neighbouring runs of the same kind joined. */
function joined(runs: readonly CodeToken[]): CodeToken[] {
  const out: CodeToken[] = [];
  for (const run of runs) {
    const last = out.at(-1);
    if (last !== undefined && last.kind === run.kind) out[out.length - 1] = { ...last, text: last.text + run.text };
    else out.push(run);
  }
  return out;
}

/** `code`'s lines in `lang`'s grammar, as runs of text. */
async function tokenize(code: string, lang: "ts" | "rust" | "bash"): Promise<CodeToken[][]> {
  const { tokens } = await codeToTokens(code, { lang, theme: THEME, includeExplanation: "scopeName" });
  return tokens.map((line) =>
    joined(
      line.flatMap((token) =>
        (token.explanation ?? [{ content: token.content, scopes: [] }]).map(({ content, scopes }): CodeToken => {
          const kind = kindOf(scopes.map(({ scopeName }) => scopeName));
          return kind === undefined ? { text: content } : { text: content, kind };
        }),
      ),
    ),
  );
}

/**
 * `code`'s lines as runs of text; neighbouring runs of the same kind are joined. In a console, a
 * line that starts with `$ ` is a prompt and a bash command; any other line is output, left plain.
 */
export async function highlight(code: string, lang: CodeLanguage): Promise<CodeToken[][]> {
  if (lang !== "console") return tokenize(code, lang);
  const lines = code.split("\n");
  const commands = await tokenize(
    lines.map((line) => (line.startsWith(PROMPT) ? line.slice(PROMPT.length) : "")).join("\n"),
    "bash",
  );
  return lines.map((line, index) => {
    if (!line.startsWith(PROMPT)) return line === "" ? [] : [{ text: line }];
    return joined([{ text: PROMPT, kind: "comment" }, ...(commands[index] ?? [])]);
  });
}
