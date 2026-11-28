# E6i — Newsreader, a monospace and Noto Sans Symbols 2 as bundled fonts

| Field | Value |
| --- | --- |
| ID | `E6i` |
| Status | `done` |
| Plan | [E6 plan §7](../../plans/editor/e6-adding-content.md#7-todos) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6c](./E6c-add-text.md) |
| ADRs | Follows [0016](../../decisions/0016-package-design.md) |
| Branch | `feat/e6i-bundled-fonts` |
| PR title | `feat(fonts): bundle Newsreader, a monospace and Noto Sans Symbols 2` |
| Size | M (200–400) |
| PR | [#836](https://github.com/laudebugs/laminapdf/pull/836) |

## Goal

The font picker offers the three OFL fonts the owner chose in [issue #532](https://github.com/laudebugs/laminapdf/issues/532) besides Lamina Sans: Newsreader, a monospace and Noto Sans Symbols 2. The symbol picker's dingbat, shape and most arrow and currency signs, which `fallbackChain` reports as missing today, draw in Noto Sans Symbols 2; the mathematical signs and ₿, which it lacks, draw in JetBrains Mono. Tracked in [#831](https://github.com/laudebugs/laminapdf/issues/831).

## Context

- [E6c](./E6c-add-text.md) built the registry (`apps/web/src/fonts/registry.ts`), which takes fonts as data (`PDF_FONTS` in `@laminapdf/fonts`), and the fallback chain and symbol data that use them
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

- Fonts on the computer in the desktop picker → [E6h](../ready/E6h-desktop-system-fonts.md)
- The picker's UI → [E6e](../done/E6e-content-tools-ui.md)

## Tasks

- [x] Choose and pin each source; add the licences
- [x] Extend `tools/fonts/subset.py` and `fonts.lock.json`; generate the TTFs
- [x] Add the `PDF_FONTS` rows and the registry's entries
- [x] Check the service worker caches the files; add a test beside the existing `/assets/` ones
- [x] Flip the symbol tests in `apps/web/test/fonts.test.ts` and measure the app's size

## Acceptance criteria

- [x] `fallbackChain` finds a font for every character of every symbol group
- [x] The three fonts embed as subsets in `AddText` (an example or test per font), with `fsType` that allows it
- [x] `cargo xtask check fonts` (or the lock check) passes; the lock lists each file's source and hash
- [x] The reader's startup JavaScript doesn't grow: the fonts are fetched when the picker opens

## Post-merge checks

None.

## Tests

- `packages/fonts` tests for the rows and the lock; `apps/web` fonts tests for the chain over the symbol groups; the service worker's precache test

## Docs to update

- `packages/fonts/README.md`, [E6 plan](../../plans/editor/e6-adding-content.md) §10 decision 3

## Risks and open questions

- Check each of the three fonts for a Reserved Font Name before the files are served. Source Sans 3's licence reserves "Source", which is why Lamina Sans is renamed; a renamed font is embedded under its served name.
- Noto Sans Symbols 2 is large (about 1 MB as TTF): it is fetched only when a symbol needs it, so the picker should name the font before it fetches.

## As built

- **Sources.** The npm packages hold only WOFF2 subsets by script (Noto Sans Symbols 2 in six), so the three files are Google Fonts' own builds from [google/fonts](https://github.com/google/fonts) at commit `5026800`, pinned in the lock's `sources` with each file's URL and SHA-256. `subset.py` still never touches the network; its docstring and the package README say how to fetch them.
- **Fonts and names.** Newsreader 1.003, JetBrains Mono 2.211 and Noto Sans Symbols 2 2.008, all OFL-1.1, none with a Reserved Font Name (each `OFL.txt` names none), so none is renamed. The monospace is JetBrains Mono, not Source Code Pro, which reserves "Source". Newsreader's TrueType file shares the licence text of its WOFF2 source (`licenceFrom` in the lock; google/fonts' own `OFL.txt` is pinned there too; the two differ only in the copyright line and trailing whitespace, checked by hand when it was pinned). The instance's `STAT` table is dropped, since it describes axes it no longer has. The Newsreader instance gets the name "Newsreader", where the variable file's default says "Newsreader 16pt".
- **Files.** `pdf/newsreader-regular.ttf` 110 KB, `pdf/jetbrains-mono-regular.ttf` 100 KB, `pdf/noto-sans-symbols-2-regular.ttf` 661 KB. Whole fonts (every mapped character), Regular instance only, hinting dropped (Noto Sans Symbols 2 is 1.2 MB with it). `fsType` is 0 in all three.
- **Startup and the service worker.** `PDF_FONTS` lives in its own module, `@laminapdf/fonts/pdf`, which only the registry imports, so the reader's entry (which imports `FONT_PRELOADS` from the main entry) carries none of it; nothing in the startup graph imports the registry yet (E6e wires it into a lazily loaded chunk). The three picker TrueType files (not Lamina Sans' and Noto Sans Arabic's, which the reader embeds offline) are left out of the service worker's install (about 0.9 MB would otherwise download with it) and are kept on first fetch like the other `/assets/` files, with a test in `apps/web/test/pwa.test.ts`. The startup size could not be measured here (the web build needs the wasm packages); CI's `cargo xtask web size` is the measure.
- **The picker names a font before fetching.** `PDF_FONTS` rows carry `bytes`, and `FontRegistry.toFetch(ids)` lists the bundled fonts a text still needs to download with their sizes, so the picker (E6e) can say "Noto Sans Symbols 2, 661 KB" first.
- **What Noto Sans Symbols 2 doesn't have.** The mathematical operators (∑ √ ≤ ≠ and the rest), among others. They come from JetBrains Mono, which has them, as the chain's last resort: the fallback order is now Lamina Sans, Noto Sans Symbols 2 and the scripts first (`preferred`), then the serif and the monospace, so a sans text doesn't take a serif's glyph for a symbol the symbol font has. Nine symbols no bundled font has (⇐ ⇒ ⇔ ↩ ↪ ∆ ₪ ⅓ ⅔) are removed from the symbol picker's data, so that every character of every group has a font; [#839](https://github.com/laudebugs/laminapdf/issues/839) restores them. Bold and italic Lamina Sans faces, which #831 also asked for, are [#838](https://github.com/laudebugs/laminapdf/issues/838).
- **Embedding.** `crates/laminapdf-ops/tests/added_text.rs` embeds a sample of each font as a subset with a `ToUnicode` map, and `apps/web/test/fonts.test.ts` reads each file with `sfnt.ts` and compares its `cmap` and `fsType` with the list. The Rust test was written without a local build (disk); CI runs it.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
