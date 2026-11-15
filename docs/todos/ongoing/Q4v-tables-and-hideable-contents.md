# Q4v — Tables that read as tables, and "On this page" a reader can hide

| Field | Value |
| --- | --- |
| ID | `Q4v` |
| Status | `ongoing` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | [Q4q](../done/Q4q-resizable-page-beside-the-code.md) |
| ADRs | [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (the CSP the inline script runs under) |
| Branch | `claude/dazzling-sagan-s9woz0` |
| PR title | `feat(docs): boxed tables with a header band, and a button that hides "On this page"` |
| Size | S |
| PR | — |

## Goal

The owner asked for two changes, after the way Claude's docs do them. The docs' tables draw their rules in `hairline`, which is too faint on `surface` to follow a row across, and the header row looks like the body but for its weight. And the "On this page" column always takes its 14rem, even from a reader who reads one long page top to bottom and wants the article wider.

## Context

- Claude's docs (code.claude.com) put a round chevron before "On this page". Pressed, the column goes, the article widens, and a small pill at the right edge brings it back. The choice is kept in localStorage and applied before the first paint. Their tables have a darker rule under the header and a light rule between rows.
- Starlight draws the column in `PageSidebar` and the tables in its `markdown.css`; `apps/docs/src/styles/paper.css` sizes the column (`--docs-toc-width`) and colours the tables.
- `apps/docs/src/components/Example.astro` sizes an example beside its page from `--docs-toc-width` and the content width.

## Scope

**In scope**

- Tables: a 1 px `chip-border` box with radius 4, `chip-border` rules between rows, the header row on `gutter` (`raised` in the dark theme).
- A `PageSidebar` override with a button before "On this page" that hides the column and widens the article to 68rem; hidden, the column shrinks to the button, which shows the column again. The choice is kept in localStorage and applied by an inline script in the head before the first paint.
- Examples beside their pages grow with the article.

**Out of scope**

- The phone and tablet layouts, where "On this page" is already a dropdown under the header.
- Hiding the left sidebar.

## Tasks

- [x] Restyle the tables in `paper.css`.
- [x] Override `PageSidebar`, add the inline script to `Head.astro`, and style both states.
- [x] Let `Example.astro` use the content width instead of a fixed 45rem.
- [x] Test both in `e2e/example.e2e.test.ts`.
- [x] Check both in a browser: light and dark, keyboard focus, phone width, an example page.

## Acceptance criteria

- [x] A table's header row and its rows' rules are visible in light and dark.
- [x] Pressing the button hides the column and widens the article; pressing it again restores both; a reload keeps the choice without a CSP error.
- [x] The button is named for what it does and reports `aria-expanded`.
- [x] `example.e2e.test.ts`, `styles.e2e.test.ts` and `theme.e2e.test.ts` pass in Chromium, Firefox and WebKit.
- [ ] CI green.
