# E6i — Newsreader, a monospace and Noto Sans Symbols 2 as bundled fonts

| Field | Value |
| --- | --- |
| ID | `E6i` |
| Status | `ongoing` |
| Plan | [E6 plan §7](../../plans/editor/e6-adding-content.md#7-todos) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6c](../done/E6c-add-text.md) |
| ADRs | Follows [0016](../../decisions/0016-package-design.md) |
| Branch | `feat/e6i-bundled-fonts` |
| PR title | `feat(fonts): bundle Newsreader, a monospace and Noto Sans Symbols 2` |
| Size | M (200–400) |
| PR | — |

## Goal

The font picker offers the three OFL fonts the owner chose in [issue #532](https://github.com/laudebugs/laminapdf/issues/532) besides Lamina Sans: Newsreader, a monospace and Noto Sans Symbols 2. The symbol picker's mathematical, dingbat and currency signs, which `fallbackChain` reports as missing today, draw in Noto Sans Symbols 2. Tracked in [#831](https://github.com/laudebugs/laminapdf/issues/831).

## Context

- [E6c](../done/E6c-add-text.md) built the registry (`apps/web/src/fonts/registry.ts`), which takes fonts as data (`PDF_FONTS` in `@laminapdf/fonts`), and the fallback chain and symbol data that use them
- `packages/fonts/` and `tools/fonts/`: `subset.py` pins npm sources and refuses a TTF it didn't write, and `fonts.lock.json` records each file's source and hash, so a new family needs a pinned source and lock entries of its own
- [E6 plan §10](../../plans/editor/e6-adding-content.md#10-decisions-on-the-open-questions), decision 3: the fonts are fetched from the reader's own origin when the picker opens

## Scope

**In scope**

- A pinned source for each of the three families (an npm package or a release archive with a checksum), and their OFL licence texts in `packages/fonts/LICENSES`
- Full TTFs written by `tools/fonts/subset.py` (or its successor), with `fonts.lock.json` entries
- `PDF_FONTS` rows (id, family, file, `unicode-range`, `href`) for each, which the registry and the picker list
- The service worker keeps the three files like the other `/assets/` fonts, so a font used once works offline
- The symbol picker shows its maths, dingbat and currency groups as available; the test that says they are reported missing flips

**Out of scope** (and which todo covers it)

- Fonts on the computer in the desktop picker → [E6h](./E6h-desktop-system-fonts.md)
- The picker's UI → [E6e](./E6e-content-tools-ui.md)

## Tasks

- [ ] Choose and pin each source; add the licences
- [ ] Extend `tools/fonts/subset.py` and `fonts.lock.json`; generate the TTFs
- [ ] Add the `PDF_FONTS` rows and the registry's entries
- [ ] Check the service worker caches the files; add a test beside the existing `/assets/` ones
- [ ] Flip the symbol tests in `apps/web/test/fonts.test.ts` and measure the app's size

## Acceptance criteria

- [ ] `fallbackChain` finds a font for every character of every symbol group
- [ ] The three fonts embed as subsets in `AddText` (an example or test per font), with `fsType` that allows it
- [ ] `cargo xtask check fonts` (or the lock check) passes; the lock lists each file's source and hash
- [ ] The reader's startup JavaScript doesn't grow: the fonts are fetched when the picker opens

## Post-merge checks

None.

## Tests

- `packages/fonts` tests for the rows and the lock; `apps/web` fonts tests for the chain over the symbol groups; the service worker's precache test

## Docs to update

- `packages/fonts/README.md`, [E6 plan](../../plans/editor/e6-adding-content.md) §10 decision 3

## Risks and open questions

- Check each of the three fonts for a Reserved Font Name before the files are served. Source Sans 3's licence reserves "Source", which is why Lamina Sans is renamed; a renamed font is embedded under its served name.
- Noto Sans Symbols 2 is large (about 1 MB as TTF): it is fetched only when a symbol needs it, so the picker should name the font before it fetches.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
