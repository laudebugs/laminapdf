// Code colouring at build time, for both sites: Shiki tokenizes the code, and each token's TextMate
// scopes give it one of five kinds, which the sites colour with the --paper-code-* tokens. The
// pages get classes, not Shiki's inline styles, because both sites' style-src 'self' blocks inline
// styles. The website's code panel (apps/site/src/data/highlight.ts) and the docs' code blocks
// (apps/docs/plugins/code-kinds.mjs) use it; the docs' editor is held to it by a test
// (apps/docs/runner/colours.test.mjs). Node-only: it runs at build time, never in a page.
import { bundledLanguages, codeToTokens, createCssVariablesTheme } from "shiki";

/** The kinds of token the sites colour; everything else is plain text. */
export const CODE_KINDS = /** @type {const} */ (["keyword", "string", "comment", "function", "constant"]);

/** @typedef {(typeof CODE_KINDS)[number]} CodeKind */

/**
 * A run of text, coloured as `kind` when it has one.
 *
 * @typedef {{ readonly text: string, readonly kind?: CodeKind }} CodeToken
 */

/** Shiki needs a theme to tokenize; the kinds come from the scopes, not from its colours. */
const THEME = createCssVariablesTheme({ name: "paper-code", variablePrefix: "--shiki-" });

/**
 * Scope prefixes and the kind each gives, `null` for plain, in the order they are tried. A token
 * inside a comment is a comment. Otherwise the scopes are tried from the innermost out, and the
 * first one a rule matches decides: operators, punctuation, variables, namespaces, `undefined` (a
 * global, not a keyword) and a template string's `${…}` stay plain; types share the constants'
 * colour; a shell command is a function, its options are constants and its arguments are plain.
 *
 * @type {readonly (readonly [prefix: string, kind: CodeKind | null])[]}
 */
const RULES = [
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
  ["constant.language.undefined", null],
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
  ["meta.template.expression", null],
];

/**
 * Where the innermost scope means something else in an enclosing one, tried first: a function's
 * name after `new` is a class; a called name is a function; a variable or parameter that holds a
 * function, where it is declared, is a plain name.
 * CodeMirror's parser sees them the same way, so the docs' editor (apps/docs/runner/kinds.ts)
 * colours them as the blocks do.
 *
 * @type {readonly (readonly [innermost: string, enclosing: string, kind: CodeKind | null])[]}
 */
const IN_CONTEXT = [
  ["entity.name.function", "new.expr", "constant"],
  ["entity.name.function", "meta.function-call", "function"],
  ["entity.name.function", "variable.other", null],
  ["entity.name.function", "meta.parameters", null],
];

/**
 * Whether `scope` is `prefix` or one of its sub-scopes.
 *
 * @param {string} scope
 * @param {string} prefix
 */
function within(scope, prefix) {
  return scope === prefix || scope.startsWith(`${prefix}.`);
}

/**
 * The kind of a token whose scopes are `scopes`, outermost first; `undefined` for plain.
 *
 * @param {readonly string[]} scopes
 * @returns {CodeKind | undefined}
 */
export function kindOf(scopes) {
  if (scopes.some((scope) => within(scope, "comment"))) return "comment";
  const innermost = scopes.at(-1) ?? "";
  for (const [prefix, enclosing, kind] of IN_CONTEXT) {
    if (within(innermost, prefix) && scopes.some((scope) => within(scope, enclosing))) return kind ?? undefined;
  }
  for (const scope of [...scopes].reverse()) {
    const rule = RULES.find(([prefix]) => within(scope, prefix));
    if (rule !== undefined) return rule[1] ?? undefined;
  }
  return undefined;
}

/**
 * `runs` with neighbouring runs of the same kind joined.
 *
 * @param {readonly CodeToken[]} runs
 * @returns {CodeToken[]}
 */
export function joined(runs) {
  /** @type {CodeToken[]} */
  const out = [];
  for (const run of runs) {
    const last = out.at(-1);
    if (last !== undefined && last.kind === run.kind) out[out.length - 1] = { ...last, text: last.text + run.text };
    else out.push(run);
  }
  return out;
}

/**
 * Whether Shiki has a grammar for `lang` (a name or an alias, as a code fence writes it).
 *
 * @param {string} lang
 */
export function hasGrammar(lang) {
  return Object.hasOwn(bundledLanguages, lang);
}

/**
 * `code`'s lines in `lang`'s grammar, as runs of text; neighbouring runs of the same kind are joined.
 *
 * @param {string} code
 * @param {string} lang a language {@link hasGrammar} knows
 * @returns {Promise<CodeToken[][]>}
 */
export async function tokenize(code, lang) {
  const { tokens } = await codeToTokens(code, {
    lang: /** @type {import("shiki").BundledLanguage} */ (lang),
    theme: THEME,
    includeExplanation: "scopeName",
  });
  return tokens.map((line) =>
    joined(
      line.flatMap((token) =>
        (token.explanation ?? [{ content: token.content, scopes: [] }]).map(({ content, scopes }) => {
          const kind = kindOf(scopes.map(({ scopeName }) => scopeName));
          return kind === undefined ? { text: content } : { text: content, kind };
        }),
      ),
    ),
  );
}
