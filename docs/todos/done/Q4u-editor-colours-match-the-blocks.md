# Q4u — The code blocks in the editor's colours

| Field | Value |
| --- | --- |
| ID | `Q4u` |
| Status | `done` |
| Plan | [Docs examples §2](../../plans/docs-examples.md#completion-and-colours-q4t-q4u) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | [Q4i](./Q4i-code-colours-by-class.md) |
| ADRs | [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (no inline styles) |
| Branch | `feat/q4t-editor-type-completion` (with Q4t, at the owner's request) |
| PR title | `feat(docs): complete the SDK's API in "Edit and run", and colour the editor as the code blocks` |
| Size | S–M |
| PR | [laudebugs/laminapdf#563](https://github.com/laudebugs/laminapdf/pull/563) |

## Goal

The docs' code blocks were coloured by Shiki in Starlight's default Night Owl themes, while the "Edit and run" editor uses the five Paper code colours (`--paper-code-*`) that the website's code panel uses. So a block changed colour when the editor replaced it. The plan first said both used the Paper colours; they didn't, and the owner chose to move the blocks to the Paper colours rather than the editor to Night Owl. This todo does that and holds the two to the same colours with a test, so no Shiki option is needed.

## Context

- [Completion and colours in "Edit and run"](../../research/docs-site/editor-completion.md#shiki-in-the-editor): why not Shiki in the editor, or a reader's toggle.
- `apps/docs/runner/theme.ts` (the editor's colours), `apps/site/src/data/highlight.ts` (the website's code panel).

## Scope

**In scope**

- The website's rules from a token's TextMate scopes to one of the five kinds moved into `@laminapdf/site-kit/code-kinds`, shared by the website and the docs.
- An Expressive Code plugin of the docs' own (`apps/docs/plugins/code-kinds.mjs`) colouring every block whose language Shiki knows by those rules, with Shiki's own colouring off, on the editor's surface and text colour (`apps/docs/ec.config.mjs`).
- The editor's tag map in its own file (`apps/docs/runner/kinds.ts`) and a Node test (`apps/docs/runner/colours.test.mjs`) that tokenizes every runnable example both ways and fails where a character's kind differs.

**Out of scope**

- Shiki in the editor; a reader's choice of highlighter.

## Decisions made while building it

- **Expressive Code's own inline style, not a class of our own.** The plugin colours each run with `InlineStyleAnnotation` and a `var(--paper-code-*)` colour, which `@laminapdf/site-kit/inline-styles` turns into a class after the build, as it did Shiki's colours. A span of our own made the Markdown pipeline fail.
- **Expressive Code's options moved to `ec.config.mjs`**, since a plugin can't be serialised into the `<Code>` component's settings.
- **Where the two disagreed, each side moved toward the meaning of the code:** in the editor, `=>` is a keyword and `void` in a type a type; in the shared rules, a name after `new` is a class, a called name a function, a variable or parameter that holds a function a plain name where it is declared, and `undefined` and a template string's `${…}` plain. The website's code panel follows the same rules.

## Tasks

- [x] Move the rules into the site kit; the website's `highlight.ts` keeps only its console prompts.
- [x] Write the plugin and the test; change the rules on both sides until the test passes.
- [x] Check the blocks in a browser, in light and dark.

## Acceptance criteria

- [x] The test passes in the docs' `pnpm test` (`cargo xtask check ts`).
- [x] Opening "Edit and run" on each runnable example changes no token's kind, in light and dark.
