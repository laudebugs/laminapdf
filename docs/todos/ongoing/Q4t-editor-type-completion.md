# Q4t — Completion, hover types and type errors in "Edit and run"

| Field | Value |
| --- | --- |
| ID | `Q4t` |
| Status | `ongoing` |
| Plan | [Docs examples §2](../../plans/docs-examples.md#completion-and-colours-q4t-q4u) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | [Q4r](../done/Q4r-wrap-and-format-code.md) |
| ADRs | [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (the CSP the worker runs under), [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (TypeScript, Apache-2.0; `@valtown/codemirror-ts`, ISC; `@typescript/vfs`, MIT) |
| Branch | `feat/q4t-editor-type-completion` |
| PR title | `feat(docs): complete the SDK's API in "Edit and run" with TypeScript's language service` |
| Size | M |
| PR | — |

## Goal

The editor in "Edit and run" completes keywords and the names the file declares, but nothing of the SDK's API: `doc.` lists nothing. This todo runs TypeScript's own language service in a worker, over the SDK's `.d.ts` files, so the editor completes members, shows a value's type on hover or by key, and marks type errors. The worker loads only once the reader starts editing.

## Context

- [Completion and colours in "Edit and run"](../../research/docs-site/editor-completion.md): the measurements (about 1.15 MB gzipped) and why not a list generated from the SDK's types.
- `apps/docs/runner/editor.ts` (the editor), `apps/docs/scripts/build-runner.mjs` (the runner's build), `apps/docs/public/_headers` (`/run/*`'s policy).
- [Twoslash spike](../done/Q4p-twoslash-spike.md): why hover types stay out of the static pages.

## Scope

**In scope**

- A module worker at `/run/types.js` holding `typescript` 6.0.3's language service over `@typescript/vfs`: the ES library declarations up to ES2022 (no DOM), `@laminapdf/sdk`'s and `@laminapdf/render`'s `.d.ts` files from the docs' own build, and a hand-written declaration of what the Node quick start calls from `node:fs` and `node:path`.
- The worker started on the editor's first focus or keystroke, never on the "Edit and run" press; until it answers, the editor's current completion works as before.
- Completion with each item's type and first doc sentence, hover types, and a key that shows the type at the cursor (named in the bar's help or tooltip).
- Type errors as lint marks with TypeScript's message; a run never waits for them or stops for them.
- The worker's script, library files and declarations content-hashed under `/run/assets/`, like the runner's.

**Out of scope**

- Go to definition, rename and code fixes.
- The DOM's types and the examples that would need them (none of the runnable ones do).
- TypeScript 7.

## Tasks

- [ ] Add `typescript`, `@typescript/vfs` and `@valtown/codemirror-ts` to the docs' dependencies (exact versions), and check them with `cargo xtask npm-licenses`.
- [ ] Build the worker and its declaration files in `build-runner.mjs`; fail the build when a declaration it needs is missing.
- [ ] Start the worker on first focus or keystroke; join its completion, hover and lint extensions to the editor, styled by `theme.ts`.
- [ ] Add the key that shows the type at the cursor.
- [ ] Measure the worker's gzipped size and its first answer's time; write both into the plan.
- [ ] Extend `e2e/runner.e2e.test.ts`: `doc.` lists `save`, a hover shows `Document`, a misspelt method is marked and the run still reports the SDK's error, the button press alone loads no TypeScript, no CSP violation.

## Acceptance criteria

- [ ] In Chromium, Firefox and WebKit, typing `doc.` after `Document.open` lists the document's methods with their types.
- [ ] The key that shows the type works with no pointer.
- [ ] Pressing "Edit and run" and running an unchanged example loads no part of TypeScript.
- [ ] Every runnable example, unchanged, shows no type error.
- [ ] No page outside `/run/*` changes its headers, and no CSP violation is reported.
