# U2d — Selection and find marks that show on every page appearance

| Field | Value |
| --- | --- |
| ID | `U2d` |
| Status | `ready` |
| Plan | [UI/UX §5.5](../../plans/ui-ux.md#55-page-appearance-and-themes-r8) |
| Round / milestone | Round U (UI/UX) — U2 Reading quality |
| Depends on | [U2c](../done/U2c-page-appearance-modes.md) |
| ADRs | Follows [0024 §5](../../decisions/0024-app-shell-ux.md) (page appearance) |
| Branch | `fix/u2d-selection-on-page-appearances` |
| PR title | `fix(viewer): selection and find marks show on Sepia and Dark pages` |
| Size | L (400–800) |
| PR | — |

## Goal

On a Dark page, selected text sits on a near-black box and the text itself dims, so a reader can't see what they selected. Find matches do the same. This todo makes the selection and the find marks land on their designed colours on the Original, Sepia and Dark pages, with the page's text readable through them and the selection still visible over pictures.

## Context

- [Bug: selection on dark pages](../../bugs/selection-on-dark-pages/detail.md): the reproduction and the numbers.
- [Bug: selection hides text](../../bugs/selection-hides-text/resolution.md): why the text layer and the find marks are `mix-blend-mode: multiply`. Its "Left for later" predicted this bug.
- [Design language §2.2, §2.6 and §2.8](../../design/paper/design-language.md): `selection-tint` is the accent at 22% over the paper (the dark accent at 35% on the Dark page); `find-match`, `find-current`, the find-match edge (1 px) and the find-current outline (2 px), with their contrast rules.
- `packages/viewer/src/layers/text-layer.ts` (the selection box and the column selection) and `highlight-layer.ts` (the find marks).
- `packages/tokens/scripts/emit.ts`: where each appearance's CSS block is written.

**The cause.** A multiplied fill shows as `paper × fill`. That is the fill itself on the Original page's white paper, but on the Dark page's paper (`#22211F`) Teal's tint `#425858` becomes `#090B0B`, and the light ink under it is multiplied down: ink on the selection measures 2.24:1, under the 4.5:1 that text needs. On Sepia it lands darker than designed.

**The fix, in two parts.** The plan review weighed one blend for everything and found it falls short both ways (the [bug's detail](../../bugs/selection-on-dark-pages/detail.md) has the numbers):

- **The selection is translucent, not blended.** The design defines the tint as the accent at 22% (Dark: 35%) over the paper, so the accent at that opacity lands on it exactly, with no calculation. The ink under it measures 9.6:1 (Original), 7.2:1 (Sepia) and 4.9:1 (Dark). It shows over pictures too, which a blended fill doesn't: screened over a white scan on a Dark page it disappears.
- **The find marks keep a blend, chosen with the paper, and a calculated fill.** Find colours are a strong amber; laid on translucently they tint the ink and `find-current` drops to 2.4:1. Blended, the ink stays as drawn. For a light paper the blend is `multiply` with fill = colour ÷ paper; for the Dark paper it is `screen` with fill = 1 − (1 − colour) ÷ (1 − paper), per channel. Every find colour lands on its token (Dark's `find-current`, `#7A5818`, on `#7A581F`: screen can't take blue below the paper's), with the ink at 5.3–12.8:1. The find-match edge and the find-current outline are drawn unblended, in their own colours, so the current match stays visible over a picture.

## Scope

**In scope**

- Tokens: for each appearance but High contrast, the selection overlay (the accent with its opacity, per accent), the find blend, and the fills for `find-match` and `find-current`, computed and emitted by the generator into `tokens.css` and `tokens.ts`.
- The viewer: the text layer's box unblended; the column selection's spans and its drag rectangle in the selection colour (`--lamina-selection`) instead of the hard-coded blue; find marks blended with `--lamina-find-blend`, with the 1 px edge and the 2 px outline drawn unblended.
- One stylesheet in `@laminapdf/ui` that maps the page tokens onto the viewer's `--lamina-*` properties and the text layer's `::selection`, on `:root`, for the web reader and the desktop app alike. The desktop app has no `::selection` rule or find colours today, so it gains both.

**Out of scope**

- The High contrast appearance and the system's forced-colours mode. Its colours are the system's (`Canvas`, `Highlight`, `Mark`), its paper is `Canvas` in the renderer but always black in the design language, and the design asks the renderer to redraw text under those fills. Checked by hand here, and written up as its own todo if it fails.
- Thumbnails, the link layer and reader mode, which draw no selection or find marks over a recoloured page.

## Tasks

- [ ] `packages/tokens/scripts`: a function that takes a paper and a colour and returns the blend and the fill, in 8-bit sRGB, rounded so the result lands on the colour where it can; and the selection overlay from the accent and the appearance's opacity.
- [ ] Emit them into `tokens.css` (per appearance, the selection per accent) and `tokens.ts` (`PAGE_APPEARANCES`); regenerate with `pnpm --filter @laminapdf/tokens generate`.
- [ ] `packages/viewer/src/layers/text-layer.ts`: drop the box's `multiply`; column selection and its rectangle use `var(--lamina-selection, …)`.
- [ ] `packages/viewer/src/layers/highlight-layer.ts`: each mark blended with `var(--lamina-find-blend, multiply)`; the edge and the outline in an unblended element above it.
- [ ] `packages/ui/src/styles`: the mapping stylesheet; take the same rules out of `apps/web/src/app.css`.
- [ ] Record the bug in `docs/bugs/selection-on-dark-pages/` (detail and resolution) and its line in `docs/bugs/README.md`.

## Acceptance criteria

- [ ] A tokens test, for Original, Sepia and Dark with each accent: the selection overlay lands within 1/255 per channel of `selection-tint`, `find-match` and `find-current` within 8/255 of their tokens; the ink under each measures ≥ 4.5:1 against it; the selection measures ≥ 1.1:1 against both white and black (it shows over any picture; multiply gives 1.0 over black today); the edge and the outline measure ≥ 3:1 against the paper.
- [ ] A viewer browser test, in Chromium, Firefox and WebKit, on a Dark page: a selected word's background is within tolerance of the tint and its ink ≥ 4.5:1, in a real screenshot; a word selected over a white image still differs from the image. The Original test keeps passing.
- [ ] A viewer browser test: on a Dark page, the find-current mark's fill is within tolerance of `find-current` and its outline is drawn.
- [ ] By hand, in the built web reader (Chromium and WebKit) and the desktop app: a selection, a column selection and a find match on Sepia and Dark pages, with screenshots in the PR.

## Post-merge checks

None.

## Tests

- `packages/tokens/test`: the numbers in the first criterion.
- `packages/viewer/test/browser/text-layer.browser.test.ts`: the Dark-page selection, over text and over an image.
- `packages/viewer/test/browser`: the find-current mark on a Dark page.

## Docs to update

- `packages/tokens/README.md` and the generated `CONTRAST.md`: the new tokens and what the ink measures under them.
- `docs/bugs/selection-hides-text/resolution.md`: point "Left for later" at the new bug.
- [UI/UX plan §9](../../plans/ui-ux.md#9-to-dos): a ticked U2 line.

## Risks and open questions

- `::selection` with `var()`: Chromium's highlight inheritance resolves custom properties differently from Firefox and WebKit. Defining the tokens on `:root`, where `data-page-appearance` sits, works in both models; the browser test covers all three engines, and the desktop webviews are checked by hand.
- A browser may draw an opaque `::selection` colour translucent on its own; the overlay is translucent already, and the screenshot test measures what is drawn.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
