# Completion and colours in "Edit and run"

The docs' "Edit and run" editor completes only what it can see in the file: keywords, a few TypeScript snippets and names declared above the cursor. A reader who types `doc.` gets nothing about `Document`, and `Builder`'s methods appear only once the example has used them. This note measures two ways to complete the SDK's own API, then asks whether the editor should colour code with Shiki, as the static code blocks do. It recommends TypeScript's own language service in a worker, loaded once the reader starts typing, and recommends against a Shiki option.

Measured on 2026-11-13 with esbuild 0.28.2 (minified ES module bundles, gzip -9) on the docs' pinned packages.

## What the editor completes today

The editor is CodeMirror 6's `basicSetup` with `javascript({ typescript: true })` (`apps/docs/runner/editor.ts`). `basicSetup` turns on `@codemirror/autocomplete`, and the language adds three sources: TypeScript keywords, TypeScript snippets (`for`, `if`, `class`, `interface`, …) and `localCompletionSource`, which lists the names the file declares in scope. None of the three reads a type. After `const doc = await Document.open(bytes)`, typing `doc.` lists nothing, and `Builder`'s options can't be completed at all.

## Two ways to complete the SDK

### A list generated from the SDK's types

The docs build already runs TypeDoc over `@laminapdf/sdk`, so it could write a JSON list of each export's members, with their signatures and first doc sentence. A completion source would then guess the type of the name before the dot. That guess works for `Document.` and `new Builder(`, where the type is in the text. But it fails for `doc.`, `page.` or `surface.`, whose types come from a call's return value or a callback's parameter, and most of the examples' code reads such names (`builder.page(size).surface()`, `s => …` in `tag`). To guess those, the source would have to follow return types and callbacks, which means writing part of a type checker. The list would weigh about 50 KB gzipped; the guesswork is its cost.

### TypeScript's language service in a worker

TypeScript's own `LanguageService` answers completions, hover types and errors from the real types. Val Town's `@valtown/codemirror-ts` (ISC, 2.3.1) joins it to CodeMirror: a worker holds the service over `@typescript/vfs` (MIT, 1.6.6), and the page gets three extensions: completion, hover tooltips and lint marks. It has no runtime dependencies; its peers are the CodeMirror packages the editor already uses. Its last release was in September 2025, and its glue is a few hundred lines, so the docs could keep a copy of their own if it stops being maintained.

The worker's weight is TypeScript itself:

| Part | gzip |
| --- | --- |
| `typescript` 6.0.3, the compiler and language service | 1,024 KB |
| The ES library declarations (`lib.es5` to `es2022`, disposable, decorators) | 51 KB |
| `@laminapdf/sdk`'s own `.d.ts` files | 52 KB |
| `@valtown/codemirror-ts` and `@typescript/vfs` | under 20 KB |
| **Total** | **about 1.15 MB** |

The runnable examples run in a worker and use no DOM, so the worker can leave out `lib.dom.d.ts` (356 KB). The one Node quick start imports `node:fs` and `node:path`; a hand-written declaration of the two functions it calls is enough. TypeScript's bundle calls neither `eval` nor `new Function` (none in the minified output), so the worker needs no `'unsafe-eval'` and can run under the docs' own CSP.

For scale, a reader's first run already loads about 3 MB gzipped: the SDK's text module and hayro. The language service would add about a third to that, and only for readers who type. The docs' pinned TypeScript is 6.0.3. TypeScript 7 is the native Go compiler, so the worker stays on TypeScript 6's JavaScript build; moving it needs a browser build of TypeScript 7's language service, which this note didn't look for. The Twoslash spike ([Q4p](../../todos/done/Q4p-twoslash-spike.md)) kept hover types out of the static pages because it added script to every page. The worker adds nothing to a page until its reader starts editing.

## Shiki in the editor

The static code blocks get their colours from Shiki at build time, through Expressive Code. Since [Q4i](../../todos/done/Q4i-code-colours-by-class.md), each token is a class coloured by one of five `--paper-code-*` tokens: keyword, string, comment, function and constant. The editor colours with CodeMirror's own parser (Lezer) and maps its tags to the same five tokens (`apps/docs/runner/theme.ts`). So both use the same five colours; they differ only where Shiki and Lezer put a token in different kinds, such as a property read or a type parameter.

Shiki in the editor would cost three things:

- **Weight.** Shiki's core, its JavaScript regex engine, the TypeScript grammar and two themes weigh 74 KB gzipped. The editor's whole CodeMirror bundle weighs 168 KB.
- **The CSP.** The existing bridge, `codemirror-shiki` (MIT, 0.3.0), colours each token with a `style` attribute, which the docs' `style-src 'self'` blocks. The docs would need their own extension that turns Shiki's tokens into the five classes, as the build does.
- **Typing speed.** TextMate grammars tokenize line by line from the top, while Lezer reparses only the edited part of its tree. A long example would retokenize from the edited line down on every keystroke.

An option to switch between them would leave the reader choosing between two shades of the same five colours. The difference a reader can see is narrower: a token that changes colour when "Edit and run" swaps the static block for the editor.

## Recommendations

1. **Complete with TypeScript's language service in a worker** (todo Q4t). The worker loads on the editor's first keystroke or focus, never on the button press, so readers who only run an example pay nothing. Completions, hover types and errors use TypeScript 6.0.3 over the SDK's own `.d.ts` files. Errors show as lint marks and never stop a run: Sucrase still strips the types, so the reader still sees the SDK's own error for a mistake the types miss. Hover types also need a keyboard path, a key that shows the type at the cursor, since a tooltip shown on mouse hover alone leaves keyboard users out.
2. **Make Lezer's colours match Shiki's instead of offering Shiki** (todo Q4u). A test tokenizes every runnable example with both, at build time, and lists each token that falls in a different one of the five kinds. `theme.ts`'s tag map then changes until the list is empty, or each remaining difference has a reason in the test. Shiki in the editor, with an extension of our own that writes classes, stays the fallback if a difference can't be fixed in the tag map. A reader-facing toggle is left out either way.
