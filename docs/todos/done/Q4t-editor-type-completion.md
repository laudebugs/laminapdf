# Q4t — Completion, hover types and type errors in "Edit and run"

| Field | Value |
| --- | --- |
| ID | `Q4t` |
| Status | `done` |
| Plan | [Docs examples §2](../../plans/docs-examples.md#completion-and-colours-q4t-q4u) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | [Q4r](./Q4r-wrap-and-format-code.md) |
| ADRs | [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (the CSP the worker runs under), [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (TypeScript, Apache-2.0) |
| Branch | `feat/q4t-editor-type-completion` |
| PR title | `feat(docs): complete the SDK's API in "Edit and run", and colour the editor as the code blocks` |
| Size | M |
| PR | [laudebugs/laminapdf#563](https://github.com/laudebugs/laminapdf/pull/563) |

## Goal

The editor in "Edit and run" completed keywords and the names the file declares, but nothing of the SDK's API: `doc.` listed nothing. This todo runs TypeScript's own language service in a worker, over the SDK's `.d.ts` files, so the editor completes members, shows a value's type on hover or by key, and marks type errors. The worker loads only once the reader starts editing.

## Context

- [Completion and colours in "Edit and run"](../../research/docs-site/editor-completion.md): the measurements and why not a list generated from the SDK's types.
- `apps/docs/runner/editor.ts` (the editor), `apps/docs/scripts/build-runner.mjs` (the runner's build), `apps/docs/public/_headers` (`/run/*`'s policy).
- [Twoslash spike](Q4p-twoslash-spike.md): why hover types stay out of the static pages.

## Scope

**In scope**

- A module worker at `/run/types.js` (`runner/types-worker.ts`) holding `typescript` 6.0.3's language service over a file system of its own: the ES library declarations up to ES2022 with `using` (no DOM), `@laminapdf/sdk`'s and `@laminapdf/wasm`'s `package.json` and `.d.ts` files from the docs' own build (bundled as `virtual:lamina-types` by `build-runner.mjs`), and a hand-written declaration of what the Node quick start calls from `node:fs/promises` and `node:path`.
- The worker started on the reader's first edit, a completion they ask for or the type key, never on the "Edit and run" press; until then the editor completes as before.
- Completion with each item's signature and first doc sentence, hover types, and Ctrl+Shift+Space for the type at the cursor, named in a line under the editor.
- Type errors as lint marks with TypeScript's message; a run never waits for them or stops for them.

**Out of scope**

- Go to definition, rename and code fixes.
- The DOM's types and the examples that would need them (none of the runnable ones do).
- TypeScript 7.

## Decisions made while building it

- **Our own glue, not `@valtown/codemirror-ts` and `@typescript/vfs`.** The page side (`runner/types.ts`) and the worker speak a protocol of four requests (`runner/types-protocol.ts`) in the runner's existing style, about 300 lines in all. Val Town's package needs Comlink for a worker, and its tooltips are its own markup; a language service host over a `Map` is 30 lines, where `@typescript/vfs` brings `debug` and file-system code the docs don't need. TypeScript is the only new runtime dependency.
- **Ctrl+Shift+Space, not VS Code's Ctrl+K then I.** Ctrl+K (⌘K) opens the docs' search on every page.
- **Completions replace CodeMirror's language sources** (`autocompletion({ override })`): TypeScript's list already holds the file's names and keywords, so the file's own names come from CodeMirror only until TypeScript has started; TypeScript's snippets stay.

## Tasks

- [x] Add `@codemirror/autocomplete`, `@codemirror/lint` and `@codemirror/view` as the docs' direct dependencies (exact versions; TypeScript was one already) and check them with `cargo xtask npm-licenses`.
- [x] Build the worker and its declaration files in `build-runner.mjs`.
- [x] Start the worker on the first edit; join its completion, hover and lint extensions to the editor, styled by the Paper tokens.
- [x] Add the key that shows the type at the cursor, and the line that names it.
- [x] Measure the worker: 1.17 MB gzipped (4.5 MB raw); in Node its first answer took 0.3 s and later ones 20–45 ms.
- [x] Extend `e2e/runner.e2e.test.ts`: `b.` lists `page`, its information shows `Builder.page(`, the type key shows `Builder`, a misspelt method is marked and the run still fails with the SDK's error, an unchanged run loads no TypeScript, and every runnable example has no type error.

## Acceptance criteria

- [x] In Chromium, Firefox and WebKit, typing `b.` after `new Builder` lists the builder's methods with their types.
- [x] The key that shows the type works with no pointer.
- [x] Pressing "Edit and run" and running an unchanged example loads no part of TypeScript.
- [x] Every runnable example, unchanged, shows no type error.
- [x] No page outside `/run/*` changes its headers, and no CSP violation is reported.
