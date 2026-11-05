# U6g — The docs site in the Paper design language (interim, before `@lamina/tokens` and `@lamina/fonts`)

| Field | Value |
| --- | --- |
| ID | `U6g` |
| Status | `done` |
| Plan | [Paper design §6 U6](../../plans/paper-design.md#u6-design-system) |
| Round / milestone | Round U (UI/UX) — U6 Paper design system |
| Depends on | [B7a](./B7a-docs-site.md) |
| ADRs | Follows [0011 §4](../../decisions/0011-cloudflare-topology-and-privacy.md) (CSP `font-src 'self'`), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md#fonts-icc-profiles-and-test-corpora) (bundled fonts OFL-1.1) |
| Branch | `feat/docs-site-paper-theme` |
| PR title | `feat(docs): the docs site in the Paper design language` |
| Size | S (≈ 300 lines of CSS and docs; the font files and licence texts don't count) |
| PR | [#121](https://github.com/laudebugs/lamina/pull/121) |

## Goal

The owner asked (2026-10-31) for the docs site to look like the app before it moves to `laminapdf.com`. Theme the Starlight site (`apps/docs`) in the Paper design language: Paper's light and proposed dark themes, Newsreader titles and Source Sans 3 text, self-hosted because the CSP allows fonts from `'self'` only. It is an interim copy: [U6a](../done/U6a-paper-tokens.md) and [U6b](../done/U6b-self-hosted-fonts.md) build the shared packages, and the docs site switches to them there.

## Context

- [Paper design language](../../design/paper/design-language.md) §2 (colour, light §2.1, dark §2.3, accents §2.8) and §3 (typography).
- [B7a](./B7a-docs-site.md) — the docs site and its `public/_headers` CSP.
- [U6a](../done/U6a-paper-tokens.md), [U6b](../done/U6b-self-hosted-fonts.md) — the packages that replace this copy.

## Scope

**In scope**

- `apps/docs/src/styles/paper.css` (Starlight `customCss`): `--paper-*` tokens mapped onto Starlight's `--sl-*` properties for light and dark (and `::backdrop`), asides, Newsreader titles, focus, tables, selection, the sidebar's current page as a selected row.
- `apps/docs/src/fonts/`: eight variable WOFF2 files (Newsreader and Source Sans 3, upright and italic, Latin and Latin Extended) from Fontsource, with their OFL texts, SHA-256s and provenance.
- The docs site's README ("Look") and CLAUDE.md.

**Out of scope** (and which todo covers it)

- The shared tokens and their contrast test → [U6a](../done/U6a-paper-tokens.md).
- Fonts subsetted from upstream sources, fallback metrics (`size-adjust`) and a preload → [U6b](../done/U6b-self-hosted-fonts.md).
- Moving the site to `laminapdf.com` → a separate PR.

## Tasks

- [x] Map the design language's tokens onto Starlight's custom properties, light and dark.
- [x] Self-host the fonts with provenance and licences.
- [x] Check light, dark and phone widths in Chromium; the fonts load from the site itself.

## Acceptance criteria

- [x] The site builds with every internal link valid; the CSP is unchanged (`font-src 'self'`).
- [x] Every text pair the theme adds has a recorded contrast ratio of at least 4.5:1.

## Post-merge checks

- [ ] The production deploy shows the Paper look.

## Tests

None beyond the docs site's build (`cargo xtask check ts`).

## Docs to update

`apps/docs/README.md`, `apps/docs/src/fonts/README.md`, CLAUDE.md, [Paper design §6](../../plans/paper-design.md#u6-design-system).

## Risks and open questions

- The tokens are copied by hand until U6a; a value changed in the design language before then must be changed here too.

## Definition of done

- [x] Claimed: the owner asked for the work directly (2026-10-31); it was added to the board in its own PR rather than claimed first
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file in `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
