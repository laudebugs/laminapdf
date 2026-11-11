// The "Edit and run" editor in the Paper look: every colour and font is a `--paper-*` token, so the
// editor follows the site's light, dark and high-contrast themes (and the header's switch) with no
// code of its own. Syntax colours are the five code kinds the website's code panel uses
// (apps/site/src/data/highlight.ts): keyword, string, comment, function, constant; the rest is text.

import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";
import { EditorView } from "codemirror";

/** A token as a CSS value. */
const token = (name: string) => `var(--paper-${name})`;

/** The editor's surfaces: the page's raised surface, its gutter, the accent for the cursor and selection. */
const surfaces = EditorView.theme({
  "&": {
    color: token("text"),
    backgroundColor: token("raised"),
  },
  "&.cm-focused": {
    outline: `${token("focus-ring-width")} solid ${token("focus")}`,
    outlineOffset: token("focus-ring-offset-inset"),
  },
  ".cm-scroller": {
    fontFamily: token("font-mono"),
    lineHeight: "1.6",
  },
  ".cm-content": { caretColor: token("accent") },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: token("accent") },
  "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, ::selection": {
    backgroundColor: token("accent-tint"),
  },
  ".cm-gutters": {
    color: token("muted"),
    backgroundColor: token("gutter"),
    borderRight: `1px solid ${token("hairline")}`,
  },
  ".cm-activeLine": { backgroundColor: "transparent" },
  ".cm-activeLineGutter": { color: token("text"), backgroundColor: "transparent" },
  "&.cm-focused .cm-matchingBracket": {
    color: token("text"),
    backgroundColor: token("accent-tint"),
    outline: `1px solid ${token("accent")}`,
  },
  ".cm-searchMatch": { backgroundColor: token("warning-tint"), outline: `1px solid ${token("warning")}` },
  ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: token("accent-tint") },
  ".cm-foldPlaceholder": {
    color: token("muted"),
    backgroundColor: token("surface"),
    border: `1px solid ${token("hairline")}`,
  },
  ".cm-tooltip": {
    color: token("text"),
    backgroundColor: token("surface"),
    border: `1px solid ${token("chip-border")}`,
  },
  ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
    color: token("text"),
    backgroundColor: token("accent-tint"),
  },
  ".cm-panels": { color: token("text"), backgroundColor: token("surface") },
});

/** TypeScript's tokens in the five code kinds, as the website's panel colours its Shiki scopes. */
const syntax = HighlightStyle.define([
  {
    tag: [t.keyword, t.moduleKeyword, t.controlKeyword, t.operatorKeyword, t.definitionKeyword, t.modifier, t.self],
    color: token("code-keyword"),
  },
  { tag: [t.string, t.special(t.string), t.regexp, t.escape], color: token("code-string") },
  { tag: [t.comment, t.lineComment, t.blockComment, t.docComment], color: token("code-comment"), fontStyle: "italic" },
  {
    tag: [t.function(t.variableName), t.function(t.propertyName), t.function(t.definition(t.variableName))],
    color: token("code-function"),
  },
  {
    tag: [t.number, t.bool, t.null, t.atom, t.typeName, t.className, t.namespace, t.standard(t.variableName)],
    color: token("code-constant"),
  },
  { tag: t.invalid, color: token("danger") },
]);

/** The theme: surfaces and syntax, both from the tokens. */
export const paperTheme = [surfaces, syntaxHighlighting(syntax)];
