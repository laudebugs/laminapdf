# Q4r — Wrap long lines in code blocks, and format the code in "Edit and run"

| Field | Value |
| --- | --- |
| ID | `Q4r` |
| Status | `done` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | [Q4q](./Q4q-resizable-page-beside-the-code.md) |
| ADRs | [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (the CSP the buttons work under), [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (Prettier, MIT) |
| Branch | `claude/dazzling-sagan-s9woz0` |
| PR title | `feat(docs): wrap long lines in code blocks, and a Format button in "Edit and run"` |
| Size | S–M |
| PR | [laudebugs/laminapdf#549](https://github.com/laudebugs/laminapdf/pull/549) |

## Goal

The owner asked for two buttons. Long lines in the docs' code blocks scroll sideways, more so since Q4q lets the page beside an example take room from the code. A reader should be able to wrap them instead, from a button beside Copy. And a reader who edits an example in "Edit and run" should be able to tidy the code they typed, from a Format button at the right end of the runner's bar.

## Context

- Starlight's Expressive Code draws each block's Copy button (`.expressive-code .copy`) and has a style for wrapped lines (`pre.wrap`).
- `apps/docs/src/components/Head.astro` (on every page), `src/styles/paper.css`.
- `apps/docs/src/components/Example.astro` (the runner's bar), `runner/editor.ts` (the editor).
- The examples are formatted by Biome at 120 columns; Prettier at the same width leaves each runnable example as it is.

## Scope

**In scope**

- A "Wrap long lines" toggle beside Copy on every code block, for every block at once, remembered in the browser (`localStorage`, a per-reader convenience).
- A Format button in "Edit and run": the editor's TypeScript through Prettier (loaded on the first press, never before), the cursor kept; code that doesn't parse leaves the text alone and says why.
- Two Paper icons, `wrap-lines` and `format-code`, by the icon package's rules.

**Out of scope**

- Formatting Rust or shell blocks.
- A per-block wrap setting.

## Tasks

- [x] The two icons, their rows in the design language.
- [x] The wrap button and its style.
- [x] The Format button and `runner/format.ts`.
- [x] e2e tests: wrap in `example.e2e.test.ts`; Format in `runner.e2e.test.ts`.
- [x] The docs examples plan's notes.

## Acceptance criteria

- [x] Wrap toggles every block's long lines with no horizontal scroll left, and is kept after a reload.
- [x] Format restores an example whose indentation was removed, byte for byte; a syntax error leaves the code as typed.
- [x] Nothing new loads before Format is pressed, and no CSP violation is reported.

## Post-merge checks

None.

## Tests

The docs e2e suite in Chromium, Firefox and WebKit (`cargo xtask pdf-runtimes <browser>`).

## Docs to update

The docs examples plan (§2, §4).

## Risks and open questions

- Prettier's TypeScript parser is about 300 KB gzipped; it loads only when Format is first pressed.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
