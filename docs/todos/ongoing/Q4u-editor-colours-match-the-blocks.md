# Q4u — The editor's colours match the static code blocks

| Field | Value |
| --- | --- |
| ID | `Q4u` |
| Status | `ongoing` |
| Plan | [Docs examples §2](../../plans/docs-examples.md#completion-and-colours-q4t-q4u) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | [Q4i](../done/Q4i-code-colours-by-class.md) |
| ADRs | [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (no inline styles) |
| Branch | `feat/q4t-editor-type-completion` |
| PR title | `fix(docs): colour the editor's tokens as the static code blocks do` |
| Size | S |
| PR | — |

## Goal

The static code blocks colour tokens with Shiki at build time; the "Edit and run" editor colours them with CodeMirror's parser. Both use the same five `--paper-code-*` colours, but a token can fall in a different kind in each, so it changes colour when the editor replaces the block. This todo finds those tokens with a test and changes the editor's tag map until they agree, so no Shiki option is needed.

## Context

- [Completion and colours in "Edit and run"](../../research/docs-site/editor-completion.md#shiki-in-the-editor): why not Shiki in the editor, or a reader's toggle.
- `apps/docs/runner/theme.ts` (the editor's tag map), Q4i's classes in the docs build, `apps/site/src/data/highlight.ts` (the website's map from Shiki's scopes).

## Scope

**In scope**

- A Node test that tokenizes every runnable example with Shiki (the docs build's grammar and map) and with Lezer (`theme.ts`'s map), and lists each token whose kind differs.
- `theme.ts`'s map changed until the list is empty, or each remaining difference has a reason in the test.

**Out of scope**

- Shiki in the editor; a reader's choice of highlighter. If a difference can't be fixed in the map, the PR says so and proposes Shiki with an extension that writes classes.

## Tasks

- [ ] Write the test, sharing the build's scope-to-kind map rather than copying it.
- [ ] Fix the tag map; record the differences that stay.
- [ ] Check an example in a browser, both themes: the colours don't change when "Edit and run" opens.

## Acceptance criteria

- [ ] The test passes in `cargo xtask check ts` and lists no unexplained difference.
- [ ] Opening "Edit and run" on each runnable example changes no token's colour, in light and dark.
