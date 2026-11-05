# Paper: design language

- **Status:** Draft (the owner chose the Paper direction on 2026-10-29; second review and accent colours, 2026-10-29)
- **Source of truth:** [`packages/tokens/tokens.json`](../../../packages/tokens/tokens.json) (`@laminapdf/tokens`) holds the values of §2–§4 and the component sizes of §6; the tables here are a readable copy, and a change goes into that file first. Those values came from the reviewed Paper canvas (`Main.dc.html` web reader, `Desktop.dc.html` Comment mode with inspector and palette, `Mobile-read.dc.html`, `Mobile-select.dc.html`, `Style.dc.html` style tile), which stays the source for everything else. Every value marked *from the files* is copied from them; values marked **(proposed)** are new and are the defaults until a reviewed board replaces them.
- **Owner sign-off (2026-11-05):** the owner approved every value `@laminapdf/tokens` holds: §2 (but §2.5's mockup-only colours), §3, §4's shadows, §6's rail and panel widths, breakpoints, page pill fade and large sheet detent, and the styles of §7.18–§7.23. On the same day the owner approved every icon of §5 (U6c). Their **(proposed)** markers are gone. The markers left in this document still wait for a decision; the z-order table waits for [U6e](../../todos/ready/U6e-overlays-and-sheets.md).
- **Rules it serves:** the [UI/UX plan](../../plans/ui-ux.md), principles [§4](../../plans/ui-ux.md#4-principles) and layout and patterns §5.
- **Screens to design with it:** the [screen inventory](screens.md).

Designers: copy the snippets below rather than redrawing a component. Engineers: the token names in §2–§4 are the names `@laminapdf/tokens` gives its CSS custom properties (`--paper-<name>`) and TypeScript constants; read them from there, never copy a value. Its README says how the names map.

## 1. Personality

**Calm, editorial, book-like.** The chrome recedes like the margins of a well-set book, so the page is the only thing that speaks: warm paper grounds, hairline rules instead of boxes, a serif for titles and a quiet sans for controls, one deep teal accent used sparingly. Use it everywhere in lamina; when a screen feels busy, remove chrome rather than adding colour.

It serves principles 1 (open into reading: the page dominates, no ribbon), 2 (docked, not floating: hairline-separated panels, shadows only on short-lived layers), 7 (prove, don't claim: the network indicator is a first-class status item) and 9 (accessible by construction: every text pair ≥ 4.5:1, every control boundary ≥ 3:1). See [§4 Principles](../../plans/ui-ux.md#4-principles).

## 2. Colour

Contrast ratios are WCAG 2.x relative-luminance ratios. `@laminapdf/tokens` recomputes every pair in its tests and lists them in its [`CONTRAST.md`](../../../packages/tokens/CONTRAST.md). Thresholds: text ≥ 4.5:1; UI component boundaries, focus rings and state indicators ≥ 3:1. ⚠ marks a pair below the threshold, with the reason it is acceptable or the rule that avoids it.

### 2.1 Light UI theme (default)

| Token | Hex | Role | Contrast |
| --- | --- | --- | --- |
| `ground` | `#F7F4EE` | Chrome: top bar, rail, status bar, mobile bars, app background | — |
| `surface` | `#FBF9F5` | Raised panels: secondary toolbar, inspector, palette, sheet, selection toolbar, inactive appearance chips | — |
| `field` | `#FFFFFF` | Inputs, textarea, secondary buttons, shortcut chips | — |
| `gutter` | `#ECE7DE` | Canvas behind pages (its own token, per plan §5.5) | white page on gutter 1.23 (the page shadow separates them) |
| `text` | `#1F1D1A` | Primary text, active labels | ground 15.32 · surface 15.99 · white 16.81 · gutter 13.65 · accent-tint 13.32 |
| `text-2` | `#3A3631` | Strong secondary: window title, quotes, mobile bar labels | ground 10.92 · surface 11.40 |
| `text-3` | `#4A453E` | Icons, field labels, chip text | ground 8.65 · surface 9.03 · white 9.49 · accent-tint 7.52 |
| `muted` | `#6B645A` | Muted text: metadata, inactive modes and tabs, status bar | ground 5.32 · surface 5.55 · white 5.84 · gutter 4.74 · accent-tint 4.62 |
| `hairline` | `#E4DED3` | Dividers and panel edges. **Decorative only** | ⚠ ground 1.22 (never the only boundary of a control) |
| `chip-border` | `#D8D1C5` | Shortcut chips, floating-panel edges, inactive appearance chips | ⚠ white 1.52 (decorative; those elements are identified by their text or shadow) |
| `control-border` | `#8A8174` | Borders of inputs, secondary buttons, swatches; sheet grabber | ground 3.50 · surface 3.65 · white 3.84 · gutter 3.12 · accent-tint 3.04 |
| `accent` | `#1F5C63` | Primary button, selected state, active-mode underline, links, network shield | ground 6.91 · surface 7.21 · white 7.58 · gutter 6.16 · accent-tint 6.00 |
| `accent-strong` | `#16454B` | Link hover; primary hover | ground 9.63 · white on it 10.57 |
| `on-accent` | `#FFFFFF` | Text on `accent` | 7.58 |
| `accent-tint` | `#E1E6E3` | Selected rows and pressed toggles (= `accent` 12% over `surface`) | text 13.32 · accent 6.00 |
| `selection-tint` | `#CEDBDD` | Text selection on an Original page (= `accent` 22% over white) | page ink 10.91 · ⚠ vs white page 1.42 (see note 1) |
| `focus` | `#1F5C63` | Focus ring: same as `accent` | ground 6.91 · surface 7.21 · gutter 6.16 · accent-tint 6.00 |
| `danger` | `#9B2335` | Destructive actions ("Delete highlight"), errors | ground 7.12 · surface 7.44 · white 7.82 |
| `positive` | `#2F6A45` | Success, valid signature | ground 5.85 · surface 6.11 · white 6.43 |
| `warning` | `#8A5300` | Warnings: approximate reflow, signature that saving would break | ground 5.77 · surface 6.02 · white 6.33 · gutter 5.14 |
| `danger-tint` / `positive-tint` / `warning-tint` | `#F1E4E2` / `#E7EBE3` / `#F0E8DC` | Backgrounds of badges and notices (10% over `surface`) | own colour on tint 6.31 / 5.32 / 5.21; text ≥ 13.5 |
| `field-tint` | `#EDF2F3` | Fill of fillable form fields while Highlight fields is on, and of design-mode fields (= `accent` 8% over white). Decoration only: the boundary is the 1 px `control-border` edge | page ink 13.71 · `control-border` on it 3.40 |
| `hover` | `rgba(31,29,26,0.06)` | Hover fill of ghost and icon buttons (= `#EAE7E1` on ground) | text 13.62 · muted 4.73 |
| `pressed` | `rgba(31,29,26,0.10)` | Pressed fill (= `#E1DFD9` on ground) | text 12.62 · `text-3` 7.12 · ⚠ muted 4.38: a pressed control's label switches to `text-3` |
| `disabled` | `#A39C91` text, `hairline` border | Disabled controls (exempt from contrast; also set `disabled`/`aria-disabled`) | surface 2.59 |
| `scrim` | `rgba(31,29,26,0.30)` | Dims the app behind the palette and dialogs | — |
| `pill` | `rgba(31,29,26,0.78)` | Page-indicator pill on the phone | `on-pill` on it (over gutter) 8.95 |
| `on-pill` | `#FFFFFF` | Text on the page pill | (see `pill`) |
| `raised` | = `surface` | Palette, menus and dialogs. The light theme draws them on `surface`; the token exists so components can use one name in both themes | as `surface` |
| `window` | `#DCD6CB` bg, `#C9C2B6` border, `#EFEBE3` title bar, `#E0D9CD` title-bar rule | Drawn macOS window frame in mockups only | title text `text-2` on title bar 10.08 |

Notes:

1. **Selection tint and highlights on white are fills, not boundaries.** `selection-tint` (1.42) and the highlight colours (1.43–1.77) are low-contrast against the page by design, so text stays readable through them (ink on them ≥ 8.77). The *selected* state never relies on the fill alone: a selected annotation gets a 2 px `focus` outline (5.29 against yellow), the mini toolbar or the action bar appears, and touch selections draw `accent` handles (7.58 on white).
2. The `Commands` button in the top bar has a `hairline` border (1.22); its visible text identifies it, so WCAG 1.4.11 does not require the border. Don't copy that border onto icon-only controls.
3. Teal `#1F5C63` is the default accent. The user can choose Blue, Plum or Green instead; every accent-derived token above has a value per accent in [§2.8](#28-accent-colours).

### 2.2 Highlight and annotation colours

From the files. Annotation colours carry meaning, so they never change with UI theme or page appearance (plan §5.5).

| Name | Hex | Page ink `#262421` on it | Swatch edge |
| --- | --- | --- | --- |
| Yellow (default) | `#F2D675` | 10.80 | `control-border` 1 px (3.65 on surface) |
| Green | `#B7D9A8` | 9.95 | same |
| Blue | `#A9C8E8` | 8.93 | same |
| Pink | `#EBB4C8` | 8.77 | same |

The selected swatch adds a 2 px `text` outline (`#1F1D1A` in the light theme; offset 1 px in the toolbar, 2 px in the inspector). The outline uses `text` rather than a fixed colour, so it follows the theme: a fixed `#1F1D1A` would measure 1.1:1 on the dark theme's `surface`.

### 2.3 Dark UI theme

The same warm family, inverted: chrome is warm charcoal, never pure black, and the accent lightens to keep its hue. The page stays Original (white) unless the user picks a page appearance.

| Token | Hex | Contrast |
| --- | --- | --- |
| `ground` | `#1E1D1A` | — |
| `surface` | `#262421` | — |
| `raised` (palette, menus, dialogs) | `#2E2C28` | — |
| `field` | `#1E1D1A` | — |
| `gutter` | `#141311` | white page on it 18.57 |
| `text` | `#ECE7DE` | ground 13.69 · surface 12.57 · raised 11.31 |
| `text-2` / `text-3` | `#CFC8BC` | ground 10.15 · surface 9.32 · raised 8.39 |
| `muted` | `#A39B8E` | ground 6.13 · surface 5.63 · raised 5.07 · on hover fill 5.01 |
| `hairline` (decorative) | `#3A3732` | ⚠ ground 1.42 |
| `chip-border` (decorative) | `#4A463F` | ⚠ raised 1.49 |
| `control-border` | `#7E766A` | ground 3.76 · surface 3.45 · raised 3.11 |
| `accent` / `focus` | `#7DBDC2` (Teal; other accents in [§2.8](#28-accent-colours)) | ground 7.97 · surface 7.32 · raised 6.59 |
| `accent-strong` (hover) | `#9BCFD3` | — |
| `on-accent` | `#1E1D1A` | on accent 7.97 |
| `accent-tint` | `#343C3B` (16% over surface) | text 9.19 · accent 5.35 |
| `danger` | `#EE8C98` | ground 7.10 · surface 6.52 · raised 5.87 |
| `positive` | `#86C49A` | ground 8.33 · surface 7.65 |
| `warning` | `#E0AE5A` | ground 8.32 · surface 7.64 |
| `danger-tint` / `positive-tint` / `warning-tint` | `#463534` / `#353E34` / `#443A2A` (16% over surface, the dark `accent-tint` rule) | own colour on tint 4.86 / 5.49 / 5.50; text ≥ 9.0 |
| `hover` | `rgba(236,231,222,0.08)` (= `#2E2D2A` on ground) | text ≥ 11 |
| `pressed` | `rgba(236,231,222,0.14)` (hover's light-theme ratio) | text 9.36 · `text-3` 6.94 |
| `disabled` | `#6A645A` | exempt (2.64) |
| `scrim` | `rgba(0,0,0,0.50)` | — |
| `pill` / `on-pill` | as in the light theme | 8.95 |

Dark-theme shadows: `shadow-page-dark` `0 1px 2px rgba(0,0,0,0.40), 0 8px 28px rgba(0,0,0,0.45)` (page sheet) and `shadow-float-dark` `0 4px 14px rgba(0,0,0,0.45)` (selection toolbar, menus, tooltips, toasts; palette and dialogs use `0 18px 48px rgba(0,0,0,0.55)` over the dark `scrim`). Thumbnails on the dark rail: `0 1px 3px rgba(0,0,0,0.50)`. The phone page sheet uses `shadow-page-dark` too, and the bottom sheet `0 -8px 28px rgba(0,0,0,0.45)`. Floating layers in dark use `raised` with a `chip-border` edge.

A high-contrast UI theme follows `forced-colors` system colours (plan §5.5), not a hand-made palette. The mapping keeps every pair readable with the OS palette under forced colours and with each browser's default system colours, which the browser tests check in three engines:

| Tokens | System colour |
| --- | --- |
| `ground`, `surface`, `raised`, `field`, `gutter`, the three tints, `accent-tint`, `field-tint`, `hover`, `pressed`, `pill`, `scrim`, `on-accent` | `Canvas` |
| `text`, `text-2`, `text-3`, `muted`, `hairline`, `chip-border`, `control-border`, `danger`, `positive`, `warning`, `focus`, `on-pill` | `CanvasText` |
| `accent`, `accent-strong`, primary pressed, the outline Blue | `LinkText` |
| `disabled` | `GrayText` |

Shadows are `none` in that theme. `accent` maps to `LinkText` rather than `Highlight`, and `focus` and `control-border` map to `CanvasText` rather than `Highlight` and `ButtonBorder`. System palettes mean `LinkText` and `CanvasText` to be read on `Canvas`; the other two differ more between engines, and Chromium's default `Highlight` is translucent.

### 2.4 Page appearance

Separate from the UI theme; chosen per document or globally. From the style tile (`Aa` chips).

| Appearance | Paper | Ink | Ink on paper | Selection tint |
| --- | --- | --- | --- | --- |
| Original | `#FFFFFF` | `#262421` | 15.48 | `#CEDBDD` (ink 10.91) |
| Sepia | `#F4ECD8` | `#3B2F20` | 11.05 | `#C5CCBE` (ink 7.91) |
| Dark | `#22211F` | `#E6E1D8` | 12.35 | `#425858` (ink 5.83) |
| High contrast | `#000000` | `#FFFFFF` (chip label weight 600) | 21.00 | system `Highlight` |

Photos are never inverted and annotation colours never change.

**Page content colours per appearance** for drawing mockup pages and thumbnails in each appearance (ratios on that paper; rules are decorative):

| Role | Original | Sepia | Dark |
| --- | --- | --- | --- |
| Heading | `#1C1B19` (17.21) | `#2E2418` (12.91) | `#F2EEE7` (13.91) |
| Body ink | `#262421` (15.48) | `#3B2F20` (11.05) | `#E6E1D8` (12.35) |
| Muted (running header, footer, captions) | `#6E6A64` (5.37) | `#6B5B45` (5.56) | `#ADA597` (6.59) |
| Rule | `#E6E3DE` | `#DCD0B6` | `#3A3833` |
| Chart bars | `#4E5A63` (7.08) | `#4E5A63` (6.01) | `#A9B4BC` (7.62) |
| Chart labels | `#3A3733` (11.84) | `#5A4C38` (7.06) | `#C9C2B6` (9.10) |

On a High contrast page, every page content colour is the ink, `#FFFFFF` (21.00).

### 2.5 Mockup-only colours (document content)

For drawing the sample page and thumbnails; not UI tokens. Page heading `#1C1B19` (17.21 on white), page muted `#6E6A64` (5.37), page rule `#E6E3DE`, chart bars `#4E5A63` (7.08), chart labels `#3A3733` (11.84). Thumbnail lines: heading `#B9B2A7`, text `#DDD7CD`, chart `#C9CFD3` (Original); **per page appearance (proposed, Reader B R10, R11)** Sepia heading `#B5A68C`, text `#DCD0B6`; Dark heading `#8D877D`, text `#55514A`, chart `#6E777D` (decorative only). Phone page-4 placeholders `#CFC9BF` / `#E3DED6`; phone page rules `rgba(0,0,0,0.1)` and header/footer `opacity: 0.75`. Links drawn in page content use the page ink with an underline, as the PDF draws them; there is no page-link colour. A signature field drawn on a page: `#EEF2F3` fill, 1 px `control-border` bottom rule, 12 px "Sign here" in `#4E5A63`, 72 px high on the desktop page.

More mockup-only colours **(proposed, Convert and produce)**: scanned paper `#F5F2EC` and `#F3F0EA`, scan ink and smudges `#B8B2A8` and `#A9A296` (X01); photo blocks `#4E5A63`, flat, no gradients (X10, X12); the fictional spot ink “Northwind Teal” `#1F7A80` (X10 separations, the annual review's buttons in X11).

### 2.6 Find marks

Used for find results and for every other search hit on the page (Find sensitive content). Ratios are page ink on the fill, and the edge or outline against the paper. The bottom edge separates other matches from a yellow highlight (`#F2D675`), which has no edge; the current match is never shown by fill alone.

| Appearance | `find-match` | Match edge | `find-current` | Current outline |
| --- | --- | --- | --- | --- |
| Original (paper `#FFFFFF`, ink `#262421`) | `#F6E3BC` (ink 12.26) | 1 px bottom `#8A5300` (6.33 on paper, 5.01 on fill) | `#EDBF6A` (ink 9.04) | 2 px `#1F5C63` (7.58 on paper) |
| Sepia (paper `#F4ECD8`, ink `#3B2F20`) | `#E9CF95` (ink 8.57) | 1 px bottom `#8A5300` (5.38 on paper, 4.17 on fill) | `#D9A851` (ink 5.99) | 2 px `#1F5C63` (6.44 on paper) |
| Dark (paper `#22211F`, ink `#E6E1D8`) | `#5A4520` (ink 6.99) | 1 px bottom `#E0AE5A` (7.95 on paper, 4.50 on fill) | `#7A5818` (ink 4.98) | 2 px `#7DBDC2` (7.61 on paper) |
| High contrast | system `Mark` / `MarkText` | 1 px bottom: the ink `#FFFFFF` | system `Highlight` / `HighlightText` | 2 px: the ink `#FFFFFF` |

On a High contrast page, the edge and the outline are the ink rather than `CanvasText`: the paper there is always `#000000`, and outside forced colours `CanvasText` is black too. The page's white ink under a `Mark` or `Highlight` fill can't be measured in advance, so the renderer draws the text under those fills in `MarkText` or `HighlightText` ([U2c](../../todos/ready/U2c-page-appearance-modes.md)); the tokens report those pairs as exempt.

### 2.7 Fixed content palettes

Colours written into the file or drawn on the page. Like the highlight colours (§2.2) they never change with UI theme or page appearance, and each swatch has a name. Each palette belongs to one job; don't mix them.

| Palette | Colours | Used for |
| --- | --- | --- |
| Annotation line colours | Black `#262421` (15.48 on white) · Red `#B3261E` (≈ 6.5) · Blue `#2D5DA8` (≈ 6.4) · Green `#2F6A45` (6.43) | Underline, strike, caret, ink, shapes, lines and annotation text. The Comment toolbar swaps between these and the highlight (fill) colours with the tool |
| Page content colours | Ink `#262421` · Grey `#6E6A64` · Teal `#1F5C63` · Red `#9B2335` · Custom…; "No fill" is a white swatch with a red diagonal. An existing object's colour comes first, named "(original)" | Text, shapes and fills added or edited in Edit mode |
| Outline colours (`/C`) | Default (`text`) · Grey (`muted`) · Red (`danger`) · Blue `#2D4A6B` · Green (`positive`). In the dark theme the rail shows Blue as `#91B3DC`, the Blue accent's dark value (7.77 on ground), since `#2D4A6B` measures 1.85 there; the file always gets `#2D4A6B` | Bookmark rows |
| Signature ink | Black `#1F1D1A` · Dark blue `#1F2F5C` (12.97 on white) | Drawn and typed signatures |
| Presence | Violet `#5B4B8A` (white on it 7.45) · Blue `#1F5C8A` (7.10) · Plum `#8A3B5C` (7.34) · Teal-green `#3B6B5C` (6.10); the same ratios as an outline on a white page | Reviewer avatars and remote selections, assigned in joining order, stable per person per document; a fifth reviewer reuses violet. Names always show, so colour is never the only cue. A remote selection is a 2 px **dashed** outline with its name tab; your own selection stays solid `focus`, so the two differ whatever the accent (§2.8) |
| Diff marks | Removed: `danger-tint` + strikethrough · Added: `positive-tint` + underline · Overlay: orange `#B4540F` (4.98 on white) and blue `#2D4A6B`, with a legend | Compare views; the current change gets the 2 px `focus` outline at a 2 px offset **and** its number chip (“3 of 14”), so it never relies on the accent hue alone (the Blue accent is the overlay blue, §2.8) |
| Measurement | Selected shape: `accent` at 12% with a 2 px `accent` edge, value chip filled `accent`; other measurements in `warning` with a white chip | Measure tool |

### 2.8 Accent colours

The owner chose on 2026-10-29 to let people pick the accent: **Teal** (default), **Blue**, **Plum** or **Green**, in both UI themes. The accent is a third setting beside Theme and Page appearance (§8.6). High contrast ignores it and uses system colours.

Each set is derived from its base colour with the same rules as Teal, and those rules reproduce every documented Teal value exactly:

- `accent-tint` is the accent at 12% over `surface` (dark: 16% over dark `surface`).
- `selection-tint` is the accent at 22% over the paper (Sepia 22%; on the Dark page, the dark accent at 35%).
- The dark accent keeps the hue and takes Teal's dark lightness (OKLCH L 0.757).
- `accent-strong` and primary pressed keep Teal's lightness ratios.

Green is a moss green, `#495533` (OKLCH chroma 0.055, hue 124°). It replaces the canvas tweak's `#3E4A3C`, which had half Teal's chroma, read as dark text and sat next to `positive` (154°).

| Accent | `accent` | `accent-strong` | primary pressed | `accent-tint` | `selection-tint` Original | Sepia | Dark `accent` / `focus` | Dark `accent-strong` | Dark `accent-tint` | Dark-page selection |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Teal (default) | `#1F5C63` | `#16454B` | `#123A3F` | `#E1E6E3` | `#CEDBDD` | `#C5CCBE` | `#7DBDC2` | `#9BCFD3` | `#343C3B` | `#425858` |
| Blue | `#2D4A6B` | `#1A3757` | `#112E4D` | `#E2E4E4` | `#D1D7DE` | `#C8C8C0` | `#91B3DC` | `#AAC7E9` | `#373B3F` | `#495461` |
| Plum | `#5B3B6B` | `#472856` | `#3D1E4C` | `#E8E2E4` | `#DBD4DE` | `#D2C5C0` | `#C59FD9` | `#D6B6E7` | `#3F383E` | `#5B4D60` |
| Green | `#495533` | `#35401F` | `#2C3615` | `#E6E5DE` | `#D7DAD2` | `#CECBB4` | `#A8B78F` | `#BDCAA8` | `#3B3C33` | `#515646` |

Dark primary pressed is the dark `accent-strong` 30% toward white: Teal `#B9DDE0`, Blue `#C4D8F0`, Plum `#E2CCEE`, Green `#D1DAC2` (`on-accent` on it 11.62 / 11.58 / 11.32 / 11.65).

`field-tint` (fillable fields, §2.1) is the accent at 8% over white in light; in dark it is the dark accent at 12% over dark `field`, with `control-border` on it ≥ 3.01 and text ≥ 10.9: Teal `#EDF2F3` / `#29302E`, Blue `#EEF1F3` / `#2C2F31`, Plum `#F2EFF3` / `#322D31`, Green `#F0F1EF` / `#2F2F28`. `on-accent` stays `#FFFFFF` in light and `#1E1D1A` in dark for every accent. Contrast, recomputed (WCAG 2.x):

| Accent | Light: on ground · on surface · white on it · white on pressed · on its tint · text on tint · page ink on selection · Sepia ink on selection | Dark: on ground · on surface · on raised · `on-accent` on it · on its tint · text on tint · page ink on selection |
| --- | --- | --- |
| Teal | 6.91 · 7.21 · 7.58 · 12.33 · 6.00 · 13.32 · 10.91 · 7.91 | 7.97 · 7.32 · 6.59 · 7.97 · 5.35 · 9.19 · 5.83 |
| Blue | 8.31 · 8.67 · 9.12 · 13.80 · 7.14 · 13.17 · 10.68 · 7.73 | 7.77 · 7.14 · 6.43 · 7.77 · 5.21 · 9.17 · 5.92 |
| Plum | 8.38 · 8.75 · 9.20 · 14.11 · 7.20 · 13.16 · 10.68 · 7.74 | 7.49 · 6.88 · 6.19 · 7.49 · 5.06 · 9.23 · 6.03 |
| Green | 7.27 · 7.59 · 7.98 · 12.76 · 6.32 · 13.31 · 10.94 · 7.95 | 7.89 · 7.25 · 6.53 · 7.89 · 5.23 · 9.07 · 5.82 |

**What follows the accent:**

- `accent`, `accent-strong`, primary pressed, `accent-tint`, `focus` and `field-tint`, the last at 8% over white.
- Links, the active-mode underline, selected rows and tabs, pressed toggles, target fields, progress fills, the network shield, the Measure tool's selected shape and the find-current outline.
- The text selection on the page, in every page appearance. lamina draws it; the PDF doesn't.

**What never follows it:**

- Annotation and highlight colours (§2.2) and page paper and ink (§2.4).
- `danger`, `warning`, `positive` and their tints.
- The fixed content palettes (§2.7). The Edit-mode "Teal" swatch is content `#1F5C63` whatever the accent.
- The find-match fill (§2.6).

**Collisions and how they are resolved:**

- **Blue** is the hex of the outline `/C` Blue and of Compare's overlay blue (§2.7). A selected bookmark row is marked by `accent-tint` and weight, not by its text colour. The current change in Compare carries its number chip as well as the outline.
- **Presence** blue `#1F5C8A` sits near the Blue accent and presence violet `#5B4B8A` (20° of hue) near the Plum accent; presence plum `#8A3B5C` is 42° away. Remote selections in every presence colour are dashed and named (§2.7), so they never look like your own.
- **Green** stays 30° of hue away from `positive`. A selected state is never drawn as green text alone; it always has the tint or the underline.

**Setting.** View settings (R10) and Settings › Appearance (S07) show an **Accent** row under Theme.

- **Control:** a radio group of four 32 px swatches named Teal, Blue, Plum, Green, with the selected ring from §7.12.
- **High contrast:** in that theme the row is disabled with the help line "High contrast uses your system colours".
- **Command:** `view.accent` (`teal`, `blue`, `plum`, `green`).
- **Applying it:** switching swaps CSS custom properties under `[data-accent]`, with no re-render and no transition under reduced motion. Each accent's dark values must apply wherever the dark theme does: under `[data-theme="dark"][data-accent=…]` **and** under `@media (prefers-color-scheme: dark)` for `:root:not([data-theme])[data-accent=…]` (Match system), so a light accent never lands on dark chrome.
- **Storage and sync:**
  - Every device always caches the last applied Theme and Accent locally (`localStorage` on the web, the settings file on desktop), signed in or not, and applies that cache before first paint, so there is no flash of teal.
  - Signed in, Theme and Accent are also saved to the account, so every device that signs in gets the same look (H02 shows it; there is no switch). After load, if the account's value differs from the cache, the app applies it without animation and updates the cache.
  - **First sign-in on a device:** if the account already has an appearance, the account's wins and replaces the device's choice; if it has none, the device's choice is uploaded and becomes the account's. After that, the latest change on any signed-in device wins.

## 3. Typography

| Family | Use |
| --- | --- |
| **Newsreader** (serif, optical sizes 6–72) | Titles: file name, inspector and sheet titles, section headings, outline rows, quoted text |
| **Source Sans 3** | Every control, label and message; fallbacks `'Segoe UI', system-ui, sans-serif` (Newsreader falls back to `Georgia, serif`) |
| **Source Serif 4** | Page mockups only (the sample document's body). Never in UI |
| `mono` | System stack, no webfont: `ui-monospace, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace`. Code blocks, object trees, payloads, licence keys, fingerprints, raw XML, JavaScript shown as text (12.5 / 1.6); field names in design mode (11) |
| **Caveat** 500 | Typed signature style only (OFL; self-hosted in the product, a second link line on the boards that use it). The other typed styles are Newsreader italic and Newsreader with letter spacing |

The link line used in every file (keep it byte-identical, inside `<helmet>`):

```html
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Newsreader:opsz,wght@6..72,400;6..72,500;6..72,600&amp;family=Source+Sans+3:wght@400;500;600&amp;family=Source+Serif+4:opsz,wght@8..60,400;8..60,600&amp;display=swap">
```

The product will self-host these fonts (no third-party requests: principle 7); the link is for mockups.

### Type scale

| Name | Family | Size / line height / weight | Usage |
| --- | --- | --- | --- |
| Site display | Source Sans 3 | 48 / 1.1 / 700 (section headings 30–34 / 700) | Docs-site pages: pricing, benchmark, download |
| Display | Newsreader | 40 / 44 / 400 | Empty states, home screen heading |
| Page heading | Newsreader | 28 / 34 / 500 | `h1` of settings and list pages (Account, Cloud files, Telemetry) |
| Title | Newsreader | 20 / 24–26 / 500 | Inspector title, sheet title, dialog title |
| Heading | Newsreader | 18 / 24 / 500 | Section headings in panels and settings |
| Document name | Newsreader | 16 / 20 / 500 (phone 17) | File name in the top bar |
| Outline | Newsreader | 15 / 21 / 400 (level 2: 14, `text-3`; current: 600) | Outline rows |
| Quote | Newsreader italic | 15 / 1.5 / 400, `text-2` | Quoted selection in the inspector |
| Input large | Source Sans 3 | 16 / 20 / 400 | Palette input; every text input on touch (stops iOS zoom) |
| Body | Source Sans 3 | 14 / 20 / 400 (emphasis 600) | Default UI text, inputs, palette rows, mode switcher |
| Control | Source Sans 3 | 13 / 18 / 400 (selected 600) | Rail tabs, secondary toolbar, selection toolbar, small buttons |
| Label | Source Sans 3 | 12 / 16 / 600, `text-3` | Field labels in the inspector and forms |
| Caption | Source Sans 3 | 12 / 16 / 400, `muted` | Metadata, status bar, palette group names, thumbnail numbers |
| Overline | Source Sans 3 | 11 / 14 / 400, uppercase, `letter-spacing: 0.06em`, `muted` | Palette section headers ("Recent", "Commands"); 12 px with 0.08em on the style tile |
| Chip | Source Sans 3 | 11 / 14 / 400, `text-3` (12 in the palette) | Shortcut chips |
| Tab label (phone) | Source Sans 3 | 11.5 / 14 / 400 (selected 600) | Bottom bar and action bar labels |

The docs site uses Source Sans 3 for all text, titles included ([quality plan §5](../../plans/quality.md#5-the-font)); the apps keep Newsreader for titles.

Rules:

- **Tabular numerals** (`font-variant-numeric: tabular-nums`) on every number that changes or lines up: page input and `/ 24`, zoom %, counts ("0 network requests", "24 pages"), dates, thumbnail and outline page numbers, shortcut chips, the page pill.
- **Minimum sizes:** 12 px for UI text; 11 px only for shortcut chips, uppercase overlines and, as an explicit exception, the page-overlay labels and tags of §7.32 (design-mode field name, size chip, tab-stop badge, suggestion type tag, snap-guide and destination tags): they sit on the page at the page's zoom, are short, and repeat at 12 px or more in the inspector or a rail tab; 11.5 px only for phone bar labels under a 22 px icon. 16 px for any text input on touch.
- Sentence case everywhere; uppercase only for overlines.

## 4. Space, shape, depth

**Spacing scale** (from the style tile): `4 · 8 · 12 · 16 · 24 · 32 · 48`. Layout gaps and paddings use the scale. Inside components the files also use `2, 6, 10, 14, 18, 20, 22, 28` (for example 22 px between mode buttons, 20 px inspector padding); reuse those exact values for the same components, don't invent new ones.

**Radius:** `3` shortcut chips, appearance-chip previews · `4` buttons, inputs, rows, swatches tiles · `6` floating panels (selection toolbar, palette, dialog), appearance chips, phone bar buttons · `12` bottom sheet top corners · `height / 2` pills and circles (colour swatches, avatar, page pill). `10` is only the drawn macOS window; the drawn Windows and Linux frames use `8` **(proposed)**.

**Borders:** 1 px everywhere. `hairline` separates regions (top bar bottom, rail right, inspector left, status bar top, secondary toolbar bottom, toolbar dividers 1×18 px). `control-border` on anything you type into or press that has no other boundary. Selected state: 2 px `accent` underline (modes, tabs) or 1 px `accent` border + `accent-tint` fill (toggles).

**Shadows** only on the page sheet and on short-lived floating layers. Panels never have shadows.

| Element | Shadow |
| --- | --- |
| Page sheet (web, desktop) | `0 1px 2px rgba(60,48,30,0.08), 0 8px 28px rgba(60,48,30,0.10)` |
| Page sheet (phone) | `0 1px 2px rgba(60,48,30,0.08), 0 6px 20px rgba(60,48,30,0.10)` |
| Thumbnail | `0 1px 3px rgba(60,48,30,0.12)` |
| Selection mini toolbar; toast, menu, tooltip | `0 4px 14px rgba(60,48,30,0.14)` |
| Bottom sheet | `0 -8px 28px rgba(60,48,30,0.12)` |
| Command palette; dialog | `0 18px 48px rgba(31,29,26,0.22)` over `scrim` |

**Z-order (proposed):**

| Layer | z-index |
| --- | --- |
| Pages, then page overlays (selection, highlights, handles) | 0, 1 |
| Docked chrome (bars, rail, inspector) | 10 |
| Selection mini toolbar, page pill | 20 |
| Bottom sheet | 30 |
| Scrim + palette / dialog | 40 |
| Menus, popovers, tooltips (portalled above whatever opened them, so a menu in a sheet or dialog draws on top) | 50 |
| Toasts | 60 |

## 5. Iconography

- Inline SVG, `viewBox="0 0 24 24"`, `fill="none"`, `stroke="currentColor"`, `stroke-width="1.5"`, default (butt) caps and miter joins, no fills. No icon fonts, no emoji.
- Drawn on the 24 grid with ~3.5–4.5 px of margin; rendered at **14/15** (status bar), **16** (buttons with a text label, zoom, close in the inspector, selection toolbar), **18** (top-bar icon buttons, palette search), **20** (sheet close), **22** (phone bars and top bar).
- Colour comes from the button (`currentColor`): `text-3` in desktop chrome, `text`/`text-2` on the phone, `accent` when selected. The network shield is the one icon stroked in `accent` while idle.
- Always `aria-hidden="true"` on the SVG. Icon-only buttons carry `aria-label` (a verb phrase: "Find in document", "Close inspector"); buttons with a visible label need nothing extra.
- Hit area is the button, not the glyph: ≥ 24 px desktop (files use 26–32), ≥ 44 px touch.

Each icon's markup lives in `packages/icons/svg/<file>.svg` (`@laminapdf/icons`), which is the source of truth. The tables below are a readable copy, and a test compares them with the files:

| Name | File | Markup |
| --- | --- | --- |
| Sidebar | `sidebar` | `<rect x="3.5" y="4.5" width="17" height="15" rx="1.5"></rect><path d="M9 4.5v15"></path>` |
| Find / Search | `find` | `<circle cx="10.5" cy="10.5" r="6"></circle><path d="M15 15l5 5"></path>` |
| Zoom out | `zoom-out` | `<path d="M6 12h12"></path>` |
| Zoom in | `zoom-in` | `<path d="M6 12h12M12 6v12"></path>` |
| More (⋯) | `more` | `<circle cx="5.5" cy="12" r="0.9"></circle><circle cx="12" cy="12" r="0.9"></circle><circle cx="18.5" cy="12" r="0.9"></circle>` |
| Close | `close` | `<path d="M6 6l12 12M18 6L6 18"></path>` |
| Back | `back` | `<path d="M15 5l-7 7 7 7"></path>` |
| Copy | `copy` | `<rect x="8.5" y="8.5" width="11" height="11" rx="1.5"></rect><path d="M15.5 8.5V6a1.5 1.5 0 0 0-1.5-1.5H6A1.5 1.5 0 0 0 4.5 6v8A1.5 1.5 0 0 0 6 15.5h2.5"></path>` |
| Highlight / Annotate | `highlight` | `<path d="M14.5 4.5l5 5L11 18H6v-5z"></path><path d="M4 20.5h9"></path>` |
| Comment / Note | `comment` | `<path d="M5 5.5h14v10H10l-4 3.5v-3.5H5z"></path>` |
| Underline | `underline` | `<path d="M7 4.5v6a5 5 0 0 0 10 0v-6"></path><path d="M5 20h14"></path>` |
| Ink | `ink` | `<path d="M4 17c3-6 5-9 7-9s1 6 3 6 3-4 6-6"></path>` |
| Shapes | `shapes` | `<rect x="4" y="4" width="10" height="10" rx="1"></rect><circle cx="15.5" cy="15.5" r="4.5"></circle>` |
| Share | `share` | `<path d="M12 15V4.5M8 8l4-3.5L16 8"></path><path d="M7 11H5.5v8.5h13V11H17"></path>` |
| Pages | `pages` | `<rect x="4.5" y="4.5" width="6" height="7.5" rx="1"></rect><rect x="13.5" y="4.5" width="6" height="7.5" rx="1"></rect><rect x="4.5" y="14" width="6" height="5.5" rx="1"></rect><rect x="13.5" y="14" width="6" height="5.5" rx="1"></rect>` |
| Reader (book) | `reader` | `<path d="M12 6.5c-2-1.3-4.5-2-8-2v13c3.5 0 6 .7 8 2 2-1.3 4.5-2 8-2v-13c-3.5 0-6 .7-8 2z"></path><path d="M12 6.5v13"></path>` |
| Network shield | `network-shield` | `<path d="M12 3.5l7 2.5v5.5c0 4.3-3 7.6-7 9-4-1.4-7-4.7-7-9V6z"></path><path d="M9 12l2.2 2.2L15.5 10"></path>` |
| Repaired (wrench) | `repaired` | `<path d="M14.5 5.5a4 4 0 0 0-5 5L4.5 15.5l4 4 5-5a4 4 0 0 0 5-5l-2.5 2.5-2.5-.5-.5-2.5z"></path>` |
| Signed file | `signed-file` | `<path d="M4 19c2-1 3-4 5-4s1 3 3 3 2-2 4-2"></path><path d="M14 4.5l5 5-7 7h-2v-2z"></path>` |
| Warning | `warning` | `<path d="M12 4l9 16H3z"></path><path d="M12 10v4M12 17v.5"></path>` |
| Chevron down | `chevron-down` | `<path d="M7 10l5 5 5-5"></path>` |
| Check (stroke 2 inside checkboxes) | `check` | `<path d="M6 12.5l4 4 8-9"></path>` |

Wrap each in:

```html
<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">…</svg>
```

`iconSvg(name, { size })` and the React `<Icon name size />` from `@laminapdf/icons` write this wrapper. New icons follow the same grid and stroke: add the SVG to `@laminapdf/icons`, whose lint enforces the rules, and a row to a table here.

**Icons from the area boards.** One drawing per name; where two areas drew the same name differently, the version below won (see [Decisions on area proposals](#decisions-on-area-proposals)). All stroke 1.5, no fills.

| Name | File | Markup | Used for |
| --- | --- | --- | --- |
| File | `file` | `<path d="M6.5 3.5h7l4 4v13h-11z"></path><path d="M13.5 3.5v4h4"></path>` | Files, attachments, “View on page”, the Windows title-bar document glyph |
| Folder | `folder` | `<path d="M3.5 6.5h6l2 2h9v10h-17z"></path>` | Folders, Save as |
| Link | `link` | `<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"></path><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"></path>` | Links, review links, “Keep proportions” |
| External link | `external-link` | `<path d="M13.5 4.5h6v6"></path><path d="M19.5 4.5L11 13"></path><path d="M17 13.5v6H4.5V7h6"></path>` | Opening a website |
| Lock | `lock` | `<rect x="5.5" y="10.5" width="13" height="9" rx="1.5"></rect><path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5"></path>` | Password, Protect, read-only fields, layer lock |
| Paperclip | `paperclip` | `<path d="M16.5 7.5l-7 7a2 2 0 0 0 2.8 2.8l7.2-7.2a4 4 0 0 0-5.7-5.7l-7.2 7.2a6 6 0 0 0 8.5 8.5l5.4-5.4"></path>` | Attachments, Attach file |
| Download | `download` | `<path d="M12 4.5V15M8 11l4 4 4-4"></path><path d="M5 19.5h14"></path>` | Download, Import, Update ready, drop hints |
| Upload | `upload` | the `share` icon | Upload to cloud, upload a signature image |
| Printer | `printer` | `<path d="M7 9V4.5h10V9"></path><rect x="4.5" y="9" width="15" height="7" rx="1"></rect><path d="M7 14h10v5.5H7z"></path>` | Print, print production |
| Info | `info` | `<circle cx="12" cy="12" r="8"></circle><path d="M12 11v5M12 8v.5"></path>` | Properties, info notes |
| Help | `help` | `<circle cx="12" cy="12" r="8"></circle><path d="M9.8 9.5a2.3 2.3 0 1 1 3.2 2.1c-.6.3-1 .8-1 1.4v.5M12 16.5v.5"></path>` | Help, shortcuts |
| Forward | `forward` | `<path d="M9 5l7 7-7 7"></path>` | History forward, Next field (mirror of Back) |
| Chevron up | `chevron-up` | `<path d="M7 14l5-5 5 5"></path>` | Previous match, collapse |
| Chevron right | `chevron-right` | `<path d="M10 7l5 5-5 5"></path>` | Collapsed tree rows, submenus (mirror of Chevron down); Chevron left is Back |
| Chevron left | `chevron-left` | the `back` icon | Previous, where a chevron pairs with Chevron right |
| Arrow up | `arrow-up` | `<path d="M12 18.5V5.5M7 10.5l5-5 5 5"></path>` | Move up in ordered lists |
| Arrow down | `arrow-down` | `<path d="M12 5.5v13M7 13.5l5 5 5-5"></path>` | Move down in ordered lists |
| Plus | `plus` | the `zoom-in` icon | Add tab, Add option, Add bookmark |
| List | `list` | `<path d="M5 7h14M5 12h14M5 17h9"></path>` | Results, contents |
| Comments list | `comments-list` | `<path d="M9 7h11M9 12h11M9 17h11"></path><path d="M4.5 7h1M4.5 12h1M4.5 17h1"></path>` | The Comments panel toggle |
| Filter | `filter` | `<path d="M4 6h16M7 12h10M10 18h4"></path>` | Filters |
| Undo | `undo` | `<path d="M9 7L4.5 11.5 9 16"></path><path d="M4.5 11.5H15a4.5 4.5 0 0 1 0 9h-3"></path>` | Top bar, toasts |
| Redo | `redo` | `<path d="M15 7l4.5 4.5L15 16"></path><path d="M19.5 11.5H9a4.5 4.5 0 0 0 0 9h3"></path>` | Top bar, toasts |
| Reset | `reset` | `<path d="M5 8.5A7.5 7.5 0 1 1 4.5 13"></path><path d="M5 4v4.5h4.5"></path>` | Reset form fields |
| Reload / update | `reload` | `<path d="M19 12a7 7 0 1 1-2.05-4.95"></path><path d="M19 4.5v4h-4"></path>` | Reload from disk, updates |
| Sync | `sync` | `<path d="M5 11a7 7 0 0 1 12.5-3.5M19 13a7 7 0 0 1-12.5 3.5"></path><path d="M17.5 4v3.5H14M6.5 20v-3.5H10"></path>` | Cloud sync status |
| Cloud | `cloud` | `<path d="M7.5 18.5h9.5a3.5 3.5 0 0 0 .4-7A5.5 5.5 0 0 0 6.8 10 4.3 4.3 0 0 0 7.5 18.5z"></path>` | Cloud files |
| Cloud off | `cloud-off` | `<path d="M7.5 18.5h9.5a3.5 3.5 0 0 0 .4-7A5.5 5.5 0 0 0 6.8 10 4.3 4.3 0 0 0 7.5 18.5z"></path><path d="M4 4l16 16"></path>` | Offline edits |
| Offline | `offline` | `<path d="M7 17.5h10a3.5 3.5 0 0 0 .5-7 5.5 5.5 0 0 0-10.6-1A4 4 0 0 0 7 17.5z"></path><path d="M4 4l16 16"></path>` | Offline strip (web app) |
| Server | `server` | `<rect x="4.5" y="5" width="15" height="6" rx="1"></rect><rect x="4.5" y="13" width="15" height="6" rx="1"></rect><path d="M8 8h.5M8 16h.5"></path>` | Server jobs |
| Bell | `bell` | `<path d="M6.5 16.5V11a5.5 5.5 0 0 1 11 0v5.5l1.5 1.5H5z"></path><path d="M10 20.5h4"></path>` | Notifications |
| Laptop | `laptop` | `<rect x="5" y="5.5" width="14" height="10" rx="1"></rect><path d="M3 18.5h18"></path>` | Signed-in devices |
| Phone | `phone` | `<rect x="7.5" y="3.5" width="9" height="17" rx="1.5"></rect><path d="M11 17.5h2"></path>` | Signed-in devices |
| Add to home screen | `add-to-home-screen` | `<rect x="7" y="3.5" width="10" height="17" rx="2"></rect><path d="M12 7.5v6M9.5 11l2.5 2.5 2.5-2.5"></path>` | Install the web app |
| View settings | `view-settings` | `<circle cx="12" cy="12" r="7.5"></circle><path d="M12 4.5v15M12 8.5h5.5M12 12h7.5M12 15.5h5.5"></path>` | View settings popover |
| Single page | `single-page` | `<rect x="7" y="4.5" width="10" height="15" rx="1"></rect>` | Page layout |
| Two pages | `two-pages` | `<rect x="3.5" y="5.5" width="8" height="13" rx="1"></rect><rect x="12.5" y="5.5" width="8" height="13" rx="1"></rect>` | Page layout |
| Focus mode | `focus-mode` | `<path d="M4.5 9V4.5H9M15 4.5h4.5V9M19.5 15v4.5H15M9 19.5H4.5V15"></path>` | Focus mode |
| Clock | `clock` | `<circle cx="12" cy="12" r="7.5"></circle><path d="M12 8v4l3 2"></path>` | Revisions |
| Replace / Swap | `replace` | `<path d="M5 9h12l-3-3M19 15H7l3 3"></path>` | Replace…, swap the two sides of a compare |
| Eye | `eye` | `<path d="M3 12c2.5-4 5.5-6 9-6s6.5 2 9 6c-2.5 4-5.5 6-9 6s-6.5-2-9-6z"></path><circle cx="12" cy="12" r="2.5"></circle>` | Preview result, show a layer, show a password |
| Eye off | `eye-off` | `<path d="M3 12c2.5-4 5.5-6 9-6s6.5 2 9 6c-2.5 4-5.5 6-9 6s-6.5-2-9-6z"></path><circle cx="12" cy="12" r="2.5"></circle><path d="M4 4l16 16"></path>` | Hide a layer, hide a password |
| Layers | `layers` | `<path d="M12 4.5l8 4-8 4-8-4z"></path><path d="M4 12.5l8 4 8-4M4 16.5l8 4 8-4"></path>` | Layers |
| Flatten | `flatten` | `<path d="M12 4.5l8 4-8 4-8-4z"></path><path d="M4 15.5l8 4 8-4"></path>` | Flatten fields or comments |
| Tag | `tag` | `<path d="M4.5 4.5h7l8 8-7 7-8-8z"></path><circle cx="8.5" cy="8.5" r="1"></circle>` | Tags |
| Structure | `structure` | `<path d="M5 5h6v4H5zM13 15h6v4h-6zM5 15h6v4H5z"></path><path d="M8 9v6M8 12h8v3"></path>` | Structure and conformance |
| Accessibility | `accessibility` | `<circle cx="12" cy="5.5" r="1.5"></circle><path d="M5 9l7 1.5L19 9M12 10.5v4l-3 5.5M12 14.5l3 5.5"></path>` | Accessibility tool |
| Action | `action` | `<path d="M13 3.5L6 13h5l-1 7.5L17 11h-5z"></path>` | Actions |
| Play | `play` | `<path d="M8 5.5v13l10-6.5z"></path>` | Media |
| 3D cube | `cube` | `<path d="M12 3.5l7.5 4v9L12 20.5l-7.5-4v-9z"></path><path d="M4.5 7.5L12 11.5l7.5-4M12 11.5v9"></path>` | 3D content |
| Ruler | `ruler` | `<path d="M3.5 15.5l12-12 5 5-12 12z"></path><path d="M7 12l2 2M10 9l2 2M13 6l2 2"></path>` | Measure |
| Distance | `distance` | `<path d="M4 12h16M4 9v6M20 9v6"></path>` | Measure tools |
| Perimeter | `perimeter` | `<path d="M5 6l12-1 2 12-13 2z"></path>` | Measure tools |
| Area | `area` | `<path d="M5 6l12-1 2 12-13 2z"></path><path d="M9 10l6 6M8 14l3 3M12 8l5 5"></path>` | Measure tools |
| Rotate left | `rotate-left` | `<path d="M4.5 12a7.5 7.5 0 1 0 2.2-5.3"></path><path d="M4.5 4.5v3h3"></path>` | Organise |
| Rotate right | `rotate-right` | `<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3"></path><path d="M19.5 4.5v3h-3"></path>` | Organise, rotate the view clockwise |
| Insert page | `insert-page` | `<path d="M6.5 3.5h7l4 4v13h-11z"></path><path d="M12 10.5v6M9 13.5h6"></path>` | Organise |
| Extract | `extract` | `<path d="M6.5 3.5h7l4 4v4"></path><path d="M6.5 3.5v17h6"></path><path d="M15 17.5h6M18 14.5l3 3-3 3"></path>` | Organise |
| Delete | `delete` | `<path d="M4.5 6.5h15"></path><path d="M9.5 6.5V4.5h5v2"></path><path d="M6.5 6.5l1 13h9l1-13"></path>` | Delete pages, files, marks |
| Crop | `crop` | `<path d="M7 3.5V17h13.5"></path><path d="M3.5 7H17v13.5"></path>` | Crop |
| Resize | `resize` | `<path d="M4.5 9.5v-5h5M19.5 14.5v5h-5"></path><path d="M4.5 4.5l6 6M19.5 19.5l-6-6"></path>` | Resize pages |
| Move to page | `move-to-page` | `<path d="M4.5 12h11M12 8l4 4-4 4"></path><path d="M19.5 5v14"></path>` | Move to page… |
| Move | `move` | `<path d="M12 4v16M4 12h16M12 4l-2.5 2.5M12 4l2.5 2.5M12 20l-2.5-2.5M12 20l2.5-2.5M4 12l2.5-2.5M4 12l2.5 2.5M20 12l-2.5-2.5M20 12l-2.5 2.5"></path>` | Move a signature or object |
| Bookmark | `bookmark` | `<path d="M7 4.5h10v15l-5-4-5 4z"></path>` | Bookmarks |
| Rename | `rename` | `<path d="M14.5 4.5l5 5L9 20H4v-5z"></path>` | Rename |
| Drag grip | `drag-grip` | `<circle cx="9" cy="7" r="0.9"></circle><circle cx="15" cy="7" r="0.9"></circle><circle cx="9" cy="12" r="0.9"></circle><circle cx="15" cy="12" r="0.9"></circle><circle cx="9" cy="17" r="0.9"></circle><circle cx="15" cy="17" r="0.9"></circle>` | Reorderable rows |
| Select | `select` | `<path d="M6 4l12 7-5.5 1.5L10 18z"></path>` | Select tools |
| Edit text | `edit-text` | `<path d="M4 7.5V5.5h10v2M9 5.5V18.5M7 18.5h4"></path><path d="M18 8.5v11M16.5 8.5h3M16.5 19.5h3"></path>` | Edit text tool |
| Text not editable | `text-not-editable` | `<path d="M4 7.5V5.5h10v2M9 5.5V18.5M7 18.5h4"></path><path d="M18 8.5v11M16.5 8.5h3M16.5 19.5h3"></path><path d="M15 14l5 5M20 14l-5 5"></path>` | Text that can't be edited in place |
| Add text | `add-text` | `<path d="M5 7V5h14v2M12 5v14M9.5 19h5"></path>` | Add text (Edit) and Text (Fill & Sign) |
| Image | `image` | `<rect x="4" y="5" width="16" height="14" rx="1.5"></rect><circle cx="9" cy="10" r="1.5"></circle><path d="M4.5 17l5-5 4 4 2.5-2.5 3.5 3.5"></path>` | Image tool |
| Page marks | `page-marks` | `<path d="M6 3.5h8.5L18 7v13.5H6z"></path><path d="M9 16.5l6-6"></path>` | Watermark, header and footer |
| Tools | `tools` | `<rect x="4.5" y="4.5" width="6" height="6" rx="1"></rect><rect x="13.5" y="4.5" width="6" height="6" rx="1"></rect><rect x="4.5" y="13.5" width="6" height="6" rx="1"></rect><path d="M16.5 13.5v6M13.5 16.5h6"></path>` | Tools ▾ in the Edit toolbar |
| Align left | `align-left` | `<path d="M5 6h14M5 10h9M5 14h14M5 18h9"></path>` | Text alignment (Edit and Comment) |
| Align centre | `align-centre` | `<path d="M5 6h14M7.5 10h9M5 14h14M7.5 18h9"></path>` | Text alignment (Edit and Comment) |
| Align right | `align-right` | `<path d="M5 6h14M10 10h9M5 14h14M10 18h9"></path>` | Text alignment (Edit and Comment) |
| Strike | `strike` | `<path d="M5 12h14"></path><path d="M16 7.5c-.6-1.8-2.2-3-4.2-3-2.4 0-4.1 1.3-4.1 3.2 0 1.4.9 2.4 2.6 3"></path><path d="M8 16.5c.6 1.8 2.2 3 4.3 3 2.5 0 4.2-1.3 4.2-3.3 0-.9-.4-1.7-1.1-2.3"></path>` | Strike |
| Caret | `caret` | `<path d="M6 17l6-10 6 10"></path><path d="M4 20.5h16"></path>` | Caret |
| Text box | `text-box` | `<rect x="3.5" y="5.5" width="17" height="13" rx="1"></rect><path d="M8 9.5h8M12 9.5v6"></path>` | Text box |
| Callout | `callout` | `<path d="M4.5 4.5h15v10h-9l-4 5v-5h-2z"></path><path d="M8 8.5h8M8 11.5h5"></path>` | Callout |
| Stamp | `stamp` | `<path d="M9.5 4.5h5l-1 6h-3z"></path><path d="M5 14a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v2.5H5z"></path><path d="M6 20h12"></path>` | Stamps |
| Rectangle | `rectangle` | `<rect x="4.5" y="6.5" width="15" height="11" rx="1"></rect>` | Shapes |
| Ellipse | `ellipse` | `<ellipse cx="12" cy="12" rx="8" ry="6"></ellipse>` | Shapes |
| Line | `line` | `<path d="M5 19L19 5"></path>` | Shapes |
| Arrow | `arrow` | `<path d="M5 19L19 5"></path><path d="M11 5h8v8"></path>` | Shapes |
| Polygon | `polygon` | `<path d="M12 4l8 6-3 9H7l-3-9z"></path>` | Shapes |
| Polyline | `polyline` | `<path d="M4 18l5-8 5 5 6-10"></path>` | Shapes |
| Pen | `pen` | `<path d="M15 4.5l4.5 4.5L9 19.5H4.5V15z"></path><path d="M13 6.5l4.5 4.5"></path>` | Stylus tools |
| Eraser | `eraser` | `<path d="M14 4.5l6 6-9 9H6l-2.5-2.5z"></path><path d="M9 9.5l6 6"></path><path d="M11 19.5h9"></path>` | Stylus tools |
| Lasso | `lasso` | `<ellipse cx="12" cy="10" rx="8" ry="5.5"></ellipse><path d="M8 14.5c-1 1.5-1 3.5 1 4.5"></path>` | Stylus tools |
| Text field | `text-field` | `<rect x="3.5" y="7" width="17" height="10" rx="1"></rect><path d="M7 10v4"></path>` | Form design |
| Check box | `check-box` | `<rect x="4.5" y="4.5" width="15" height="15" rx="1.5"></rect><path d="M8 12.5l3 3 5-6"></path>` | Form design |
| Radio group | `radio-group` | `<circle cx="12" cy="12" r="7.5"></circle><circle cx="12" cy="12" r="3"></circle>` | Form design |
| Dropdown | `dropdown` | `<rect x="3.5" y="7" width="17" height="10" rx="1"></rect><path d="M13 11l2 2 2-2"></path>` | Form design |
| List box | `list-box` | `<rect x="4.5" y="4.5" width="15" height="15" rx="1"></rect><path d="M8 9h8M8 12h8M8 15h5"></path>` | Form design |
| Push button | `push-button` | `<path d="M8 7.5h8a4.5 4.5 0 0 1 0 9H8a4.5 4.5 0 0 1 0-9z"></path>` | Form design |
| Date | `date` | `<rect x="4.5" y="5.5" width="15" height="14" rx="1.5"></rect><path d="M4.5 9.5h15M8.5 3.5v4M15.5 3.5v4"></path>` | Date fields |
| Calculated | `calculated` | `<path d="M17 5.5H7l5 6.5-5 6.5h10"></path>` | Calculated fields |
| Tab order | `tab-order` | `<path d="M10 7h10M10 12h10M10 17h10M5 5.5v3M4 17h2"></path>` | Tab order |
| Detect fields | `detect-fields` | `<path d="M4.5 9V4.5H9M15 4.5h4.5V9M19.5 15v4.5H15M9 19.5H4.5V15"></path><path d="M8 12h8"></path>` | Detect fields |
| Certificate | `certificate` | `<rect x="4" y="4.5" width="16" height="11" rx="1"></rect><path d="M7.5 8.5h9M7.5 11.5h4"></path><circle cx="16" cy="16" r="2.8"></circle><path d="M14.6 18.5l-.8 3 2.2-1 2.2 1-.8-3"></path>` | Sign with a certificate, chain of trust |
| Signature image | `signature-image` | the `ink` icon | Add a signature image; signature fields use Signed file |
| Shield cross | `shield-cross` | `<path d="M12 3.5l7 2.5v5.5c0 4.3-3 7.6-7 9-4-1.4-7-4.7-7-9V6z"></path><path d="M9.5 9.5l5 5M14.5 9.5l-5 5"></path>` | Signature not trusted |
| Redact | `redact` | `<rect x="4" y="9" width="16" height="6"></rect><path d="M4 5.5h10M4 18.5h7"></path>` | Redact |
| Mark text | `mark-text` | `<path d="M6 5h12M12 5v11"></path><path d="M5 20h14"></path>` | Redact tools |
| Mark area | `mark-area` | `<rect x="4.5" y="5.5" width="12" height="10" stroke-dasharray="2 2"></rect><rect x="15" y="14" width="4.5" height="4.5"></rect>` | Redact tools (a dashed rectangle with a square corner handle, so it no longer looks like Focus mode) |
| Keyboard | `keyboard` | `<rect x="3" y="6.5" width="18" height="11" rx="1.5"></rect><path d="M7 10.5h1M11 10.5h1M15 10.5h1M8 14h8"></path>` | Type a signature, keyboard shortcuts |
| Key | `key` | `<circle cx="8" cy="12" r="3.5"></circle><path d="M11.5 12H20M17 12v3M20 12v2"></path>` | Keychain |
| Smart card | `smart-card` | `<rect x="3.5" y="6" width="17" height="12" rx="1.5"></rect><rect x="6.5" y="9.5" width="4" height="3.5" rx="0.5"></rect>` | Smart card |
| Snap to grid | `snap-to-grid` | `<path d="M4.5 9h15M4.5 15h15M9 4.5v15M15 4.5v15"></path>` | Form design toolbar |

**Icons for the reader and the core components (U6c).** U6c drew these on the same grid, because the reader v0.1 and the core components need them and no board had drawn them.

| Name | File | Markup | Used for |
| --- | --- | --- | --- |
| Fit width | `fit-width` | `<path d="M4.5 4.5v15M19.5 4.5v15"></path><path d="M8 12h8M10.5 9.5L8 12l2.5 2.5M13.5 9.5L16 12l-2.5 2.5"></path>` | Zoom to the page width |
| Fit page | `fit-page` | `<path d="M4.5 4.5h15M4.5 19.5h15"></path><path d="M12 8v8M9.5 10.5L12 8l2.5 2.5M9.5 13.5L12 16l2.5-2.5"></path>` | Zoom to the whole page |
| Error | `error` | `<circle cx="12" cy="12" r="8"></circle><path d="M12 7.5v5.5M12 16v.5"></path>` | Error notices and field errors (the mirror of Info) |
| Positive | `positive` | `<circle cx="12" cy="12" r="8"></circle><path d="M8.5 12.5l2.5 2.5 4.5-5"></path>` | Success notices, a valid result |
| Settings | `settings` | `<path d="M4.5 8h8M16.5 8h3M4.5 16h3M11.5 16h8"></path><circle cx="14.5" cy="8" r="2"></circle><circle cx="9.5" cy="16" r="2"></circle>` | Settings (two sliders, not a gear) |
| Clear | `clear` | `<circle cx="12" cy="12" r="8"></circle><path d="M9.5 9.5l5 5M14.5 9.5l-5 5"></path>` | Clear a search or text field |

Window caption glyphs (minimise, maximise, close) are drawn on a 10 × 10 grid with 1 px strokes and are mockup-only.

## 6. Layout

### 6.1 Desktop and web shell (inverted L)

| Region | Size | Background | Notes |
| --- | --- | --- | --- |
| Top bar | `min-height: 40px`, padding `0 16px`, gaps `8px 20px` | `ground`, `hairline` bottom | Three groups: file (flex `1 1 220px`), mode switcher (centred), tools (flex `1 1 220px`, right-aligned, gap 14). Wraps onto two lines when narrow |
| Secondary toolbar | `min-height: 40px`, padding `0 16px`, gaps `4px 6px`, 13 px text | `surface`, `hairline` bottom | Only in modes other than Read |
| Left rail | `220px` (every board) | `ground`, `hairline` right | Rule: default 220, min 196, user-resizable, remembered locally. 280 with four or more tabs (§8.10); a rail panel (§7.34) may be 260–280 |
| Canvas | flex `999 1 560px` (560 min before wrapping), padding `32px 16px` (desktop `28px 16px`) | `gutter` | Pages centred |
| Inspector | `288px`, padding `20px 20px 24px`, gap 22 | `surface`, `hairline` left | Only while something is selected |
| Wide docked panel | `360px`, same anatomy as the inspector | `surface`, `hairline` left | In the inspector's place, for panels whose rows carry a sentence: Comments, network activity, reports (sanitise, redaction verification), jobs. Opened by a toggle with `aria-expanded`; it replaces the inspector while open. **One right dock (proposed, Comment):** a wide panel and the inspector never show together; with the Comments panel open, selecting an annotation expands its thread, “Properties” in the thread's More menu swaps the dock to the inspector and “Back to comments” swaps it back |
| Status bar | `min-height: 28px`, padding `0 16px`, 12 px | `ground`, `hairline` top | Left: network indicator, then the sync item when the file is in the cloud; middle **(proposed, Desktop shell)**: notices, progress and persistent action items (a progress item such as “Printing pages 5–12” with Cancel printing, the “Update ready” action item), then at most one note, by priority **(proposed)**: signed-file or constraint note → repaired notice → history note (“Earlier versions stay inside the file until you choose Save as optimised…”, only once original content is deleted, replaced or recoloured) → tool hint while a creation tool is active → nothing; right: page label, then signature, security or claim badges (facts about the document, never notices or progress) |
| Window frame (desktop mockup) | title bar 30 px, traffic lights 12 px gap 8, radius 10 | see `window` | Draw for macOS. **Windows (proposed):** title bar 32 px, document glyph in `accent`, left-aligned title, caption buttons 46 × 32 with 10 px glyphs; menu bar 28 px. **Linux (proposed):** 36 px title bar, centred 13/600 title, one 24 px round close button on a 10% `text` fill. Windows and Linux frames use radius 8 |
| Document tab strip (desktop) **(proposed)** | 34 px, on the title-bar colour `#EFEBE3`, `#E0D9CD` rule below | see §7.33 | Hidden with one tab |

**Page sheet:** `max-width: 612px` (US Letter width at 100%), `min-height: 790px`, white, page shadow, no border, no radius. Gutter colour fills the rest.

**Breakpoints:**

| Width | Shell |
| --- | --- |
| < 600 px | Phone shell (§6.2) |
| 600–1023 px | Desktop shell; rail collapsed by default (opens docked, pushing the canvas); the inspector docks at 288 px and closes the rail if the canvas would fall below 560 px |
| ≥ 1024 px | Desktop shell, rail open by default |

A coarse pointer (`pointer: coarse`) uses the touch rules (§8.7) at any width. At phone width the desktop shell is never squeezed: the mode switcher moves into the bottom bar's Annotate entry and More sheet.

### 6.2 Phone shell (390 × 844 artboard)

| Region | Size | Notes |
| --- | --- | --- |
| Top bar | 56 px, padding `0 6px`, `ground`, `hairline` bottom | Back (44×44), centred title (Newsreader 17/500) over a 12 px `muted` subtitle, overflow (44×44). Auto-hides on scroll down, returns on scroll up or tap |
| Page | 358 px wide (16 px gutters), 16 px from the top bar, 14 px between pages | Phone page shadow |
| Page pill | 30 px high, padding `0 14px`, radius 15, `pill` bg, white 13/500 tabular, 100 px from the bottom (16 above the bar) | `aria-live="polite"`; fades out 1.5 s after scrolling stops |
| Bottom bar | 84 px incl. safe area: padding `6px 8px 22px`, 5 equal columns, gap 4, `ground`, `hairline` top | Search, Annotate, Pages, Reader, More. Buttons 56 high, radius 6, 22 px icon over 11.5 px label, gap 3 |
| Bottom action bar | 76 px incl. safe area: padding `4px 8px 18px`, 5 columns, buttons 52 high | Replaces the bottom bar while text is selected: Copy, Highlight, Comment, Share, More |
| Bottom sheet | `surface`, radius `12px 12px 0 0`, `hairline` top, sheet shadow | Grabber button 64×44 with a 36×5 `control-border` bar (radius 3, 8 px from top); header: title 20/500 + count 13 `muted`, close 44×44 |
| Home indicator | 134×5, radius 3, `text` at 0.85, 6–8 px from bottom | Mockups only; real apps leave `env(safe-area-inset-bottom)` |

**Sheet detents:** medium = sheet top at 404 px of 844 (≈ 48% from the top; 364 px tall above a 76 px action bar), from the file. Large = sheet top 56 px from the top (just under the top bar's space), collapsed = hidden. Tapping the grabber cycles medium → large → medium; dragging snaps; the grabber is a real `<button aria-label="Resize sheet">` (single-pointer alternative to the drag, WCAG 2.5.7). The page stays visible above a medium sheet.

**Tablet (1024 × 1366, proposed):** desktop shell with touch rules: rail as a docked panel (collapsed by default in portrait), inspector docked, bottom action bar instead of the mini toolbar, 44 px targets. Sizes: top bar 56 px with 44 × 44 icon buttons and 56 px mode buttons (padding `2px 10px 0`); secondary toolbar 56 px with 44 px tool toggles (padding `0 14px`, radius 6, 20 px icons, 14 px labels); status bar 44 px with 13 px items; page fitted to 880 px with 72 px gutters. Zoom controls and the Commands button leave the top bar on touch. A stylus gets a tool shelf (§7.35) instead of the secondary toolbar. **Tablet Comment toolbar (proposed, Comment C09):** the C01 Comment toolbar at touch size until a pen is used, then the tool shelf for the rest of the session; `comment.pen` is an alias of `comment.ink` (one op, one palette entry).

## 7. Components

Snippets use literal hex values; in canvases that expose the accent as a tweak, replace `#1F5C63` with `{{accent}}` and the tints with values computed in `renderVals()` (see `mix()` in the files). All snippets assume the root sets `font-family: 'Source Sans 3', 'Segoe UI', system-ui, sans-serif; font-size: 14px; line-height: 1.4; color: #1F1D1A` and `button{font:inherit;color:inherit}`.

### 7.0 Shared states

| State | Ghost / icon button | Secondary button | Primary button | Toggle (aria-pressed) |
| --- | --- | --- | --- | --- |
| Default | transparent, `text-3` | white, `control-border` | `accent` bg and border, white 600 | transparent, 1 px transparent border |
| Hover **(proposed)** | `hover` fill | `hover` fill over white | `accent-strong` | `hover` fill |
| Pressed **(proposed)** | `pressed` fill, label `text-3` | `pressed` | `#123A3F` (white 12.33) | — |
| Selected | — | — | — | `accent` border, `accent-tint` fill, `accent` 600 |
| Focus (keyboard) | `outline: 2px solid #1F5C63; outline-offset: 2px` (1 px offset where space is tight: mini toolbar, palette input; `-2px` inset inside edge-to-edge phone bars) | same | same | same |
| Disabled **(proposed)** | `disabled` text, no hover | `disabled` text, `hairline` border | `hairline` fill and border, `disabled` text | as ghost |

Use `:focus-visible`, never remove the ring, and every board shows the ring on one control.

- **Focus ring on dark media (proposed, Convert and produce X12):** on a dark poster or photo (§7.40) the ring is 2 px white with offset 2, because `focus` teal has too little contrast there.
- **Blocked primary (proposed, Sign/protect/redact G10):** a primary that is blocked for a reason the user must read (Save while a report blocks saving, §8.5) uses a `hairline` fill and edge with a `text-3` label (7.09:1 on `hairline`; `muted` would be 4.36:1), `aria-disabled="true"`, stays focusable, and has `aria-describedby` pointing at the strip that says why. Unlike the disabled primary above, its label stays readable because its reason matters.

### 7.1 Top bar

Anatomy: sidebar toggle · file name (Newsreader 16/500, ellipsis) + save state (12 `muted`: "No unsaved changes", "1 unsaved change") · mode switcher · page input `n` + `/ N` · zoom −/%/+ · find · Commands button with shortcut chip · Save (primary, only when there are changes) · More.

Variants **(proposed)**:

| Variant | Contents |
| --- | --- |
| Reader v0.1 (read-only product) | Sidebar toggle · file name (with “Loading pages…” or “Locked” only while those apply) · History (Back · Forward) · page n/N · zoom −/%/+ · Find · View (view settings, `aria-haspopup="dialog"`) · Reader (toggle, `aria-pressed`) · Commands · More. No mode switcher and no save state |
| Editor | Sidebar toggle · file name + save state · a 1×18 `hairline` divider · Undo · Redo · mode switcher · page n/N · zoom · Find · View · Commands · Save · More. In Read mode the Reader toggle joins View |
| No document (start window, home, settings) | File area replaced by “No document open”, the wordmark (Newsreader 18/500) or Back (“Back to Q3-board-report.pdf”) + page title; Commands and More only |
| Organise | The page input and zoom are replaced by a thumbnail size control (− 100% +); while the Crop tool shows a single page (O09) the same control zooms that page, its buttons named Zoom out and Zoom in **(proposed, Pages and navigation)** |

- **History buttons:** Back and Forward 26 px icon buttons in a `role="group"` before the page input; names say the destination (“Back to page 2”); disabled with the `disabled` token when there is nothing to go to.
- **Undo and Redo:** 28 px icon buttons whose names and tooltips include the step (“Undo move image”), disabled with no history.
- **Page input with a label:** widens to fit a page label (44 px for “A-1”); the physical position follows in `muted`: `(22 / 24)`.
- **Reader settings in the View slot (proposed, Reader B R14):** while Reader mode is on, the View button toggles the docked Reader settings panel instead of the View popover (same icon and place, `aria-label="Reader settings"`).
- **Save split button:** primary Save (radius `4px 0 0 4px`) plus a 24 px chevron button (radius `0 4px 4px 0`, “Save options”) behind a 45% white divider; open state `accent-strong`. Without the File System Access API (web), Save becomes Download with the Download icon.

```html
<header style="min-height: 40px; display: flex; flex-wrap: wrap; align-items: center; gap: 8px 20px; padding: 0 16px; border-bottom: 1px solid #E4DED3; background: #F7F4EE">
  <div style="display: flex; align-items: center; gap: 10px; flex: 1 1 220px; min-width: 0">
    <button aria-label="Sidebar" aria-pressed="true" style="width: 28px; height: 28px; display: flex; align-items: center; justify-content: center; border: 0; background: transparent; border-radius: 4px; color: #4A453E"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><rect x="3.5" y="4.5" width="17" height="15" rx="1.5"></rect><path d="M9 4.5v15"></path></svg></button>
    <span style="font-family: 'Newsreader', Georgia, serif; font-size: 16px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis">Q3-board-report.pdf</span>
    <span style="font-size: 12px; color: #6B645A; white-space: nowrap">No unsaved changes</span>
  </div>
  <!-- mode switcher (7.2) -->
  <div style="display: flex; align-items: center; justify-content: flex-end; gap: 14px; flex: 1 1 220px">
    <div style="display: flex; align-items: center; gap: 6px; font-variant-numeric: tabular-nums">
      <label for="pg" style="position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0)">Page</label>
      <input id="pg" value="3" style="width: 30px; height: 24px; box-sizing: border-box; text-align: center; border: 1px solid #8A8174; border-radius: 4px; background: #FFFFFF; font: inherit; font-variant-numeric: tabular-nums; color: #1F1D1A">
      <span style="color: #6B645A; white-space: nowrap">/ 24</span>
    </div>
    <!-- zoom: two 26×26 icon buttons around <span style="min-width: 40px; text-align: center">110%</span> -->
    <!-- find: icon button (7.3) -->
    <button style="height: 26px; flex-shrink: 0; white-space: nowrap; display: flex; align-items: center; gap: 8px; padding: 0 6px 0 10px; border: 1px solid #E4DED3; background: #FBF9F5; border-radius: 4px; color: #6B645A; font-size: 13px"><span>Commands</span><span style="font-size: 11px; padding: 1px 5px; border: 1px solid #D8D1C5; border-radius: 3px; color: #4A453E; background: #FFFFFF">Ctrl K</span></button>
    <button style="height: 26px; padding: 0 12px; border: 1px solid #1F5C63; background: #1F5C63; color: #FFFFFF; border-radius: 4px; font-size: 13px; font-weight: 600">Save</button>
    <!-- more: icon button -->
  </div>
</header>
```

### 7.2 Mode switcher

In `@laminapdf/ui`: `ModeSwitcher`. **Keyboard (decided, U6d):** each mode button is its own Tab stop, with Enter or Space to switch, because a `role="group"` gives no hint that arrow keys work. The secondary toolbars below use the toolbar pattern instead (ADR 0024 §8).

Buttons with `aria-pressed` in a `role="group"` (not tabs: modes change tools, not a panel). Active: `text` 600 + 2 px `accent` underline; inactive: `muted`, transparent underline (keeps height stable). Hover **(proposed)**: label turns `text`. Gap 22, height 40 (fills the top bar).

```html
<div role="group" aria-label="Mode" style="display: flex; align-items: stretch; gap: 22px; height: 40px">
  <button aria-pressed="true" style="height: 40px; display: flex; align-items: center; border: 0; background: transparent; padding: 2px 0 0; color: #1F1D1A; font-weight: 600; border-bottom: 2px solid #1F5C63">Read</button>
  <button aria-pressed="false" style="height: 40px; display: flex; align-items: center; border: 0; background: transparent; padding: 2px 0 0; color: #6B645A; border-bottom: 2px solid transparent">Comment</button>
  <button aria-pressed="false" style="height: 40px; display: flex; align-items: center; border: 0; background: transparent; padding: 2px 0 0; color: #6B645A; border-bottom: 2px solid transparent">Fill &amp; Sign</button>
  <button aria-pressed="false" style="height: 40px; display: flex; align-items: center; border: 0; background: transparent; padding: 2px 0 0; color: #6B645A; border-bottom: 2px solid transparent">Organise</button>
  <button aria-pressed="false" style="height: 40px; display: flex; align-items: center; border: 0; background: transparent; padding: 2px 0 0; color: #6B645A; border-bottom: 2px solid transparent">Edit</button>
</div>
```

### 7.3 Icon button

In `@laminapdf/ui`: `IconButton`, which requires its `label`.

Desktop 28×28 (top bar, inspector close), 26×26 (zoom), 32×32 (style tile); phone 44×44 with 22 px icon, radius 6. Radius 4, transparent, `text-3`.

```html
<button aria-label="Find in document" style="width: 28px; height: 28px; display: flex; align-items: center; justify-content: center; border: 0; background: transparent; border-radius: 4px; color: #4A453E"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6"></circle><path d="M15 15l5 5"></path></svg></button>
```

### 7.4 Text buttons

In `@laminapdf/ui`: `Button`.

Heights: 32 (default, padding 0 16), 30 (inspector, padding 0 14), 26 (top bar, padding 0 12, 13 px); phone ≥ 44 **(proposed)**. Radius 4. One primary per view.

```html
<button style="height: 32px; padding: 0 16px; border: 1px solid #1F5C63; background: #1F5C63; color: #FFFFFF; border-radius: 4px; font-weight: 600">Save</button>
<button style="height: 32px; padding: 0 16px; border: 1px solid #8A8174; background: #FFFFFF; border-radius: 4px">Reply</button>
<button style="height: 32px; padding: 0 12px; border: 0; background: transparent; border-radius: 4px; color: #1F5C63; font-weight: 600">Export highlights</button>
<button style="height: 32px; padding: 0 12px; border: 0; background: transparent; border-radius: 4px; color: #9B2335">Delete highlight</button>
<!-- danger, filled (proposed; only as the confirm button of a destructive dialog; white on it 7.82) -->
<button style="height: 32px; padding: 0 16px; border: 1px solid #9B2335; background: #9B2335; color: #FFFFFF; border-radius: 4px; font-weight: 600">Delete 3 pages</button>
```

### 7.5 Segmented tabs (rail tabs)

In `@laminapdf/ui`: `SegmentedTabs` and `TabPanel`. **Activation (decided, U6d):** automatic, so an arrow key selects the tab it moves to, because a rail panel appears as soon as its tab is selected; a slow panel passes `keyboardActivation="manual"`.

`role="tablist"` with `role="tab"`/`aria-selected`; 13 px, gap 16, padding `14px 18px 0`, 2 px `accent` underline on the selected tab, `hairline` under the row. Arrow keys move between tabs. Which tabs appear is in §8.10.

**More tabs button (proposed).** With four or more tabs the rail is 280 px, the row gap 12 and padding `14px 10px 0 14px`; after the third tab, outside the `tablist`, a 28 px icon button with the More icon (16 px), `aria-label="More tabs"`, `aria-haspopup="menu"`, `aria-expanded`, opening a menu (§7.26) of the other tabs. A tab chosen from it takes the third slot; the selected tab is always in the row.

```html
<button aria-label="More tabs" aria-haspopup="menu" aria-expanded="false" style="width: 28px; height: 28px; margin-bottom: 4px; display: flex; align-items: center; justify-content: center; border: 0; background: transparent; border-radius: 4px; color: #4A453E"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><circle cx="5.5" cy="12" r="0.9"></circle><circle cx="12" cy="12" r="0.9"></circle><circle cx="18.5" cy="12" r="0.9"></circle></svg></button>
```

**Segmented tabs in a sheet or page (proposed):** full-width 44 px tabs (15 px; selected 600 with a 2 px `accent` underline) over a hairline, for the phone Contents sheet and the Files home (“On this device · Cloud · Shared with me”).

```html
<div role="tablist" aria-label="Sidebar" style="display: flex; gap: 16px; padding: 14px 18px 0; border-bottom: 1px solid #E4DED3">
  <button role="tab" aria-selected="true" style="border: 0; background: transparent; padding: 0 0 8px; font-size: 13px; font-weight: 600; color: #1F1D1A; border-bottom: 2px solid #1F5C63">Pages</button>
  <button role="tab" aria-selected="false" style="border: 0; background: transparent; padding: 0 0 8px; font-size: 13px; color: #6B645A; border-bottom: 2px solid transparent">Outline</button>
  <button role="tab" aria-selected="false" style="border: 0; background: transparent; padding: 0 0 8px; font-size: 13px; color: #6B645A; border-bottom: 2px solid transparent">Attachments</button>
</div>
```

### 7.6 Thumbnail

In `@laminapdf/ui`: `Thumbnail`, a button whose `children` are the page image the viewer drew.

116×150 in the rail (column, gap 22, padding 22 0), 122×158 in the phone sheet (2-column grid, gaps `18px 16px`). White, thumbnail shadow, 1 px `hairline` outline; **current page**: `outline: 2px solid #1F5C63; outline-offset: 3px`, `aria-current="page"`, number 600 `text`; others number `muted`. Number 12 px (13 on phone) tabular, 8 px below. Thumbnails are named and numbered by the page label (“Page A-1, 22 of 24”).

**Keyboard focus (proposed)** is distinct from the current page: a rounded (radius 6) 2 px `focus` ring around the whole thumbnail and its number, with the `hover` fill and `8px 10px` padding.

**Organise tile (proposed):** 8 px padded tile, page 132 × 171 (landscape 171 × 132) with the thumbnail shadow and `hairline` outline. Selected: `accent-tint` tile, 2 px `accent` outline offset 3 on the page, a 20 px `accent` check badge top right with a 2 px gutter-coloured ring, number 600; keyboard focus adds a 2 px `accent` ring around the tile. Insertion bar: 2 px `accent` line with 8 px end dots, centred in the gap. Drag ghost: two offset 62 × 80 pages with the floating shadow and a count pill; source tiles fade to 55%. The grid is a `listbox` with `aria-multiselectable="true"`.

**Phone selection circle (proposed):** 24 px circle on thumbnails in selection mode: off = white with a 1.5 px `control-border` edge; on = `accent` with a white check (stroke 2.5).

```html
<button aria-label="Page 3" aria-current="page" style="display: flex; flex-direction: column; align-items: center; gap: 8px; border: 0; background: transparent; padding: 0">
  <span style="display: block; width: 116px; height: 150px; background: #FFFFFF; box-shadow: 0 1px 3px rgba(60,48,30,0.12); overflow: hidden; outline: 2px solid #1F5C63; outline-offset: 3px">
    <span style="display: flex; flex-direction: column; gap: 5px; padding: 14px 12px">
      <span style="height: 4px; width: 60%; background: #B9B2A7; border-radius: 1px"></span>
      <span style="height: 3px; width: 100%; background: #DDD7CD"></span>
      <span style="height: 3px; width: 94%; background: #DDD7CD"></span>
      <span style="height: 5px; width: 80%; background: #C9CFD3"></span>
    </span>
  </span>
  <span style="font-size: 12px; font-variant-numeric: tabular-nums; color: #1F1D1A; font-weight: 600">3</span>
</button>
```

### 7.7 Outline row

In `@laminapdf/ui`: `OutlineRow`. **Chevron (decided, U6h):** a row with children has a chevron button before its link, on desktop as on touch, so the link and the button each do one thing. The button keeps one name, such as “Sections in 2. Repairs backlog”, and `aria-expanded` says whether they show; a name that flips between Expand and Collapse would say the same thing twice. The tree below stays proposed.

Links in a `<nav aria-label="Document outline">`, padding `14px 10px`, gap 2. Row: Newsreader 15, padding `6px 8px`, radius 4, page number right (Source Sans 3 12 `muted` tabular). Level 2: 14 px, `text-3`, left padding 22. Current: `accent` text 600 on `accent-tint`, `aria-current="location"`. Hover **(proposed)**: `hover` fill.

**Outline tree (proposed; replaces the link list once an outline has levels):** the same rows as a WAI-ARIA tree (`ul role="tree"`, `li role="treeitem"` with `aria-level` and `aria-expanded`). A 16 px chevron (down = expanded, right = collapsed) is decoration inside the row, toggled by click or ← →, so there is no nested button. The current row tracks scrolling (the deepest entry at or above the top of the view).

- **Touch (proposed):** 48 px rows: a 44 px chevron button (`aria-expanded`, “Expand <section>”/“Collapse <section>”) or a 44 px spacer, then the link (Newsreader 17; level 2 at 16 with a 20–28 px indent), page number 13 `muted`. The current row's tint covers the whole row.
- **Editable tree row (proposed, Organise):** adds a drag grip (`accent` when selected), an inline rename input (Newsreader 14, 26 high, `control-border`, focus ring) and the row colour from the outline palette (§2.7). Keyboard moves: ⇧⌘↑ ⇧⌘↓ move, ⇧⌘← ⇧⌘→ change level (Ctrl Shift + arrows), only while the tree has focus.
- **Other trees** use the same rows: the XMP tree (28 px rows, groups 13/600 with a chevron, properties indented 30 with a right-aligned 12 px value), the object tree (`mono`), the tag tree and the chain-of-trust tree (28 px rows with a certificate icon and a 12 px note, inside a white bordered box). Selected rows use `accent-tint` and `accent`.

```html
<a href="#" aria-current="location" style="display: flex; justify-content: space-between; gap: 8px; padding: 6px 8px; border-radius: 4px; color: #1F5C63; background: #E1E6E3; text-decoration: none; font-family: 'Newsreader', Georgia, serif; font-size: 15px; font-weight: 600"><span>2. Repairs backlog</span><span style="font-family: 'Source Sans 3', system-ui, sans-serif; font-size: 12px; font-weight: 400; font-variant-numeric: tabular-nums">3</span></a>
<a href="#" style="display: flex; justify-content: space-between; gap: 8px; padding: 5px 8px 5px 22px; border-radius: 4px; color: #4A453E; text-decoration: none; font-family: 'Newsreader', Georgia, serif; font-size: 14px"><span>Open repairs by region</span><span style="font-family: 'Source Sans 3', system-ui, sans-serif; font-size: 12px; color: #6B645A; font-variant-numeric: tabular-nums">3</span></a>
```

### 7.8 Status bar items

In `@laminapdf/ui`: `StatusBarItem`, `NetworkIndicator`, `RepairedNotice` and `PageLabel`.

Each item is a button (min-height 24, 12 px, `muted`) that opens its panel. Network indicator: shield in `accent`, count in `text`, explanation in `muted`. Repaired notice: wrench + "Repaired on open · Details" (Details underlined). Page label: plain text, "Page 3 · Repairs backlog" or "Page 3 of 24". If requests > 0 **(proposed)**: count stays `text`, explanation names the host ("2 requests · to files.example.org only"); a request to an unexpected origin turns the shield and count `warning`. **Wording (decided, one form for every area):** with a document open the count is for that document: “0 network requests for this document · Offline-safe: nothing uploaded” when nothing was sent, otherwise “N network requests for this document · to lamina’s storage only” (the destination names of [Account and cloud](account-cloud-product.md); in shared review, “· live connection to lamina’s review server”). Screens without a document (start window, home, settings, product pages) drop “for this document”: “0 network requests · Offline-safe: nothing uploaded”, or the count plus the destination.

More items **(proposed)**; all keep the 2 px `focus` outline with a 1 px offset and radius 4:

| Item | Spec |
| --- | --- |
| Pressed item | While its panel or popover is open: `accent-tint` fill, 1 px `accent` border, `aria-expanded="true"` |
| Progress item | Glyph, label in `text`, a 140 × 4 bar (§7.22), a ghost text button (“Cancel printing”); reopens its job panel (“Making searchable · 1 of 2 pages”, “2 server jobs”) |
| Action item | `accent-tint` button (radius 4) with an `accent` 600 label and icon (“Update ready”), opening a status popover (§7.28) |
| Mode item | Shows an active view state and leaves it on press, with an inline shortcut chip (“Focus mode `Ctrl \`”) |
| Tool hint | Middle slot, one line with the keyboard alternative while a creation tool is active (`aria-live="polite"`) |
| Sync item | Left group beside the network indicator: sync, cloud-off or warning icon and text (“Live · comments synced 10:32”, “2 saves waiting to upload”, “Conflict copy · Details”) |
| Signature item | A §7.23 badge with the overall result, then “N signatures · Details” |
| Security item | Lock icon + “Protected” (`text`) + “· AES-256 · …” (`muted`); opens the protection summary |
| Claim badges | §7.23 badges naming a standard (“PDF/A-2b”, “PDF/UA-1 · 1 issue”) as one button that opens the check panel |

```html
<footer style="min-height: 28px; display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 4px 16px; padding: 0 16px; border-top: 1px solid #E4DED3; background: #F7F4EE; font-size: 12px; color: #6B645A">
  <button style="min-height: 24px; display: flex; align-items: center; gap: 7px; border: 0; background: transparent; padding: 0 4px; font-size: 12px; color: #6B645A">
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#1F5C63" stroke-width="1.5" aria-hidden="true"><path d="M12 3.5l7 2.5v5.5c0 4.3-3 7.6-7 9-4-1.4-7-4.7-7-9V6z"></path><path d="M9 12l2.2 2.2L15.5 10"></path></svg>
    <span style="color: #1F1D1A; font-variant-numeric: tabular-nums">0 network requests for this document</span><span>· Offline-safe: nothing uploaded</span>
  </button>
  <button style="min-height: 24px; display: flex; align-items: center; gap: 6px; border: 0; background: transparent; padding: 0 4px; font-size: 12px; color: #6B645A">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M14.5 5.5a4 4 0 0 0-5 5L4.5 15.5l4 4 5-5a4 4 0 0 0 5-5l-2.5 2.5-2.5-.5-.5-2.5z"></path></svg>
    <span>Repaired on open · <span style="text-decoration: underline">Details</span></span>
  </button>
  <span style="font-variant-numeric: tabular-nums">Page 3 · Repairs backlog</span>
</footer>
```

### 7.9 Selection mini toolbar (desktop, fine pointer)

`role="toolbar"`, above the first selected line (flips below on collision), `surface`, `chip-border` edge, radius 6, padding 4, gap 2, floating shadow, 13 px. Buttons 30 high, padding `0 10px`, 16 px icon + label; a 1×18 `hairline` divider before More (30×30 icon). Shown on `pointerup`, Shift+F10 reaches it, Escape dismisses.

```html
<div role="toolbar" aria-label="Selection" style="position: absolute; display: flex; align-items: center; gap: 2px; padding: 4px; background: #FBF9F5; border: 1px solid #D8D1C5; border-radius: 6px; box-shadow: 0 4px 14px rgba(60,48,30,0.14); font-size: 13px; line-height: 1; white-space: nowrap">
  <button style="height: 30px; display: flex; align-items: center; gap: 6px; padding: 0 10px; border: 0; background: transparent; border-radius: 4px"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><rect x="8.5" y="8.5" width="11" height="11" rx="1.5"></rect><path d="M15.5 8.5V6a1.5 1.5 0 0 0-1.5-1.5H6A1.5 1.5 0 0 0 4.5 6v8A1.5 1.5 0 0 0 6 15.5h2.5"></path></svg>Copy</button>
  <button style="height: 30px; display: flex; align-items: center; gap: 6px; padding: 0 10px; border: 0; background: transparent; border-radius: 4px"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M14.5 4.5l5 5L11 18H6v-5z"></path><path d="M4 20.5h9"></path></svg>Highlight</button>
  <button style="height: 30px; display: flex; align-items: center; gap: 6px; padding: 0 10px; border: 0; background: transparent; border-radius: 4px"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M5 5.5h14v10H10l-4 3.5v-3.5H5z"></path></svg>Comment</button>
  <span aria-hidden="true" style="width: 1px; height: 18px; background: #E4DED3"></span>
  <button aria-label="More selection actions" style="width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; border: 0; background: transparent; border-radius: 4px"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><circle cx="5.5" cy="12" r="0.9"></circle><circle cx="12" cy="12" r="0.9"></circle><circle cx="18.5" cy="12" r="0.9"></circle></svg></button>
</div>
```

### 7.10 Command palette

Modal over `scrim` (from below the window title bar), 96 px from the top, `max-width: 600px`, `surface`, `chip-border` edge, radius 6, palette shadow. Input: 44 high, inset 10, `control-border`, white, radius 4, 18 px search icon `muted`, 16 px text, `esc` chip; keeps focus (APG combobox: `role="combobox"`, `aria-activedescendant`). List: `role="listbox"`, padding `0 6px 8px`; section overlines ("Recent" first, then "Commands"); rows 40 high, padding `0 12px`, gap 12: title (query match in 600) · group name (12 `muted`) · shortcut chip. Active row: `accent-tint` fill, `inset 0 0 0 1px` `accent` ring, group `text-3`, chip border `accent`. Footer: 12 `muted` hints over a `hairline`.

```html
<div style="position: absolute; inset: 0; background: rgba(31,29,26,0.30); display: flex; justify-content: center; align-items: flex-start; padding: 96px 16px 0; box-sizing: border-box">
  <div role="dialog" aria-label="Command palette" aria-modal="true" style="width: 100%; max-width: 600px; background: #FBF9F5; border: 1px solid #D8D1C5; border-radius: 6px; box-shadow: 0 18px 48px rgba(31,29,26,0.22); overflow: hidden">
    <div style="display: flex; align-items: center; gap: 10px; margin: 10px; padding: 0 12px; height: 44px; border: 1px solid #8A8174; border-radius: 4px; background: #FFFFFF; outline: 2px solid #1F5C63; outline-offset: 1px">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#6B645A" stroke-width="1.5" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6"></circle><path d="M15 15l5 5"></path></svg>
      <label for="cmdq" style="position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0)">Search commands</label>
      <input id="cmdq" role="combobox" aria-expanded="true" aria-controls="cmdlist" aria-activedescendant="cmd1" value="highl" style="flex: 1 1 auto; min-width: 0; border: 0; outline: 0; background: transparent; font-size: 16px; color: #1F1D1A">
      <span style="font-size: 11px; padding: 1px 6px; border: 1px solid #D8D1C5; border-radius: 3px; color: #4A453E">esc</span>
    </div>
    <ul id="cmdlist" role="listbox" aria-label="Commands" style="list-style: none; margin: 0; padding: 0 6px 8px; display: flex; flex-direction: column">
      <li aria-hidden="true" style="padding: 8px 12px 4px; font-size: 11px; letter-spacing: 0.06em; text-transform: uppercase; color: #6B645A">Recent</li>
      <li id="cmd1" role="option" aria-selected="true" style="display: flex; align-items: center; gap: 12px; height: 40px; padding: 0 12px; border-radius: 4px; background: #E1E6E3; box-shadow: inset 0 0 0 1px #1F5C63">
        <span style="flex: 1 1 auto"><strong style="font-weight: 600">Highl</strong>ight selection</span>
        <span style="font-size: 12px; color: #4A453E">Comment</span>
        <span style="min-width: 22px; text-align: center; font-size: 12px; padding: 1px 6px; border: 1px solid #1F5C63; border-radius: 3px; background: #FFFFFF">H</span>
      </li>
      <li role="option" aria-selected="false" style="display: flex; align-items: center; gap: 12px; height: 40px; padding: 0 12px; border-radius: 4px">
        <span style="flex: 1 1 auto">Export <strong style="font-weight: 600">highl</strong>ights as Markdown</span>
        <span style="font-size: 12px; color: #6B645A">Export</span>
        <span style="font-size: 12px; padding: 1px 6px; border: 1px solid #D8D1C5; border-radius: 3px; background: #FFFFFF; color: #4A453E">⌥⌘E</span>
      </li>
    </ul>
    <div style="display: flex; flex-wrap: wrap; gap: 16px; padding: 9px 16px; border-top: 1px solid #E4DED3; font-size: 12px; color: #6B645A"><span>↑ ↓ to move</span><span>↵ to run</span><span>Type “go to 12” or “zoom 150” to pass a value</span></div>
  </div>
</div>
```

**Shortcut chip** (anywhere): 11–12 px, padding `1px 5–6px`, `chip-border`, radius 3, white, `text-3`, tabular. macOS glyphs run together (`⌥⌘E`, `⌘K`); Windows/Linux words with spaces (`Ctrl K`, `Ctrl Shift E`).

In `@laminapdf/ui`: `ShortcutChip`, which writes a command's keys for the platform the app names.

**Plan label (proposed):** a command outside the user's plan shows a neutral badge “In [PLAN NAME]” (§7.23; on the active row: white with `accent` border and text) in place of the shortcut chip, and the footer explains it. Commands in the plan show no badge; never a lock icon.

### 7.11 Inspector panel

Docked right, 288 px, `surface`. Sections stack with gap 22:

1. **Header:** title (Newsreader 20/500, line height 1.2) over a 12 `muted` context line ("Page 3 · Repairs backlog"); close icon button (16 px icon) top right.
2. **Subject preview:** for text annotations, the quoted text (Newsreader 15/1.5 italic `text-2`, curly quotes).
3. **Fields:** label (12/600 `text-3`, a real `<label>` when it labels an input) over the control, gap 8 (4 for read-only values).
4. **Actions:** pinned to the bottom (`margin-top: auto`), gap 8: secondary + danger ghost, 30 px high, 13 px.

The same shell **(proposed)** serves the **details panel** (a document-level explanation that isn't a selection: repairs, network activity; opened from a status-bar item that shows the pressed state), the **job panel** (long-running work on this document: title + context line, progress, a per-item list, review fields; it can close while work continues) and the wide docked panels of §6.1. Inspector extras: a **properties list** (two-column grid, 148 px labels 12/600 `text-3`, values 14 `text`, extra facts `muted`, gap `8px 16px`, 24 px copy buttons per value; sections headed with Heading 18), an **option list editor** (rows 30 high in a `control-border` box with `hairline` separators, 24 px Move up and Remove buttons per row, “Add option” ghost below) and **inset fields** (Left, Top, Right, Bottom number inputs in a 2 × 2 grid under a `legend`).

```html
<aside aria-label="Inspector" style="flex: 0 0 288px; box-sizing: border-box; border-left: 1px solid #E4DED3; background: #FBF9F5; display: flex; flex-direction: column; gap: 22px; padding: 20px 20px 24px">
  <div style="display: flex; align-items: flex-start; justify-content: space-between; gap: 8px">
    <div style="display: flex; flex-direction: column; gap: 2px">
      <h2 style="margin: 0; font-family: 'Newsreader', Georgia, serif; font-size: 20px; font-weight: 500; line-height: 1.2">Highlight</h2>
      <span style="font-size: 12px; color: #6B645A">Page 3 · Repairs backlog</span>
    </div>
    <button aria-label="Close inspector" style="width: 28px; height: 28px; display: flex; align-items: center; justify-content: center; border: 0; background: transparent; border-radius: 4px; color: #4A453E"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18"></path></svg></button>
  </div>
  <blockquote style="margin: 0; font-family: 'Newsreader', Georgia, serif; font-size: 15px; line-height: 1.5; color: #3A3631; font-style: italic">“The backlog fell by 18% over the quarter, but the oldest 10% of jobs are now older than 90 days.”</blockquote>
  <div style="display: flex; flex-direction: column; gap: 8px">
    <span style="font-size: 12px; font-weight: 600; color: #4A453E">Author</span>
    <div style="display: flex; align-items: center; gap: 10px">
      <span aria-hidden="true" style="width: 28px; height: 28px; border-radius: 50%; background: #E1E6E3; color: #1F5C63; display: flex; align-items: center; justify-content: center; font-size: 12px; font-weight: 600">RO</span><span>R. Osei</span>
    </div>
  </div>
  <div style="display: flex; flex-direction: column; gap: 8px">
    <label for="note" style="font-size: 12px; font-weight: 600; color: #4A453E">Note</label>
    <textarea id="note" rows="3" style="resize: vertical; box-sizing: border-box; width: 100%; padding: 8px 10px; border: 1px solid #8A8174; border-radius: 4px; background: #FFFFFF; font-size: 14px; line-height: 1.45">Ask for the 90-day list by region.</textarea>
  </div>
  <div style="display: flex; flex-direction: column; gap: 4px">
    <span style="font-size: 12px; font-weight: 600; color: #4A453E">Created</span>
    <span style="font-variant-numeric: tabular-nums">28 Oct 2026, 16:04</span>
  </div>
  <div style="display: flex; gap: 8px; margin-top: auto">
    <button style="height: 30px; padding: 0 14px; border: 1px solid #8A8174; background: #FFFFFF; border-radius: 4px; font-size: 13px">Reply</button>
    <button style="height: 30px; padding: 0 12px; border: 0; background: transparent; border-radius: 4px; font-size: 13px; color: #9B2335">Delete highlight</button>
  </div>
</aside>
```

### 7.12 Colour swatches

In `@laminapdf/ui`: `SwatchGroup`, whose colours are token names.

Circles with `aria-label` (colour name) and `aria-pressed`; 24 px in the secondary toolbar (gap 4, preceded by a 12 `muted` "Colour" label), 28 px in the inspector (gap 8). Edge 1 px `control-border`; selected `outline: 2px solid #1F1D1A` (offset 1 / 2). On touch **(proposed)**, the 26 px swatch sits centred in a 44 × 44 button and the selected ring is a `box-shadow` (2 px `surface` gap, 2 px `text`) so it doesn't change the layout. The same circles carry the other palettes of §2.7.

```html
<div role="group" aria-label="Colour" style="display: flex; gap: 8px">
  <button aria-label="Yellow" aria-pressed="true" style="width: 28px; height: 28px; border-radius: 50%; border: 1px solid #8A8174; background: #F2D675; outline: 2px solid #1F1D1A; outline-offset: 2px"></button>
  <button aria-label="Green" aria-pressed="false" style="width: 28px; height: 28px; border-radius: 50%; border: 1px solid #8A8174; background: #B7D9A8"></button>
  <button aria-label="Blue" aria-pressed="false" style="width: 28px; height: 28px; border-radius: 50%; border: 1px solid #8A8174; background: #A9C8E8"></button>
  <button aria-label="Pink" aria-pressed="false" style="width: 28px; height: 28px; border-radius: 50%; border: 1px solid #8A8174; background: #EBB4C8"></button>
</div>
```

### 7.13 Save note and save banner (signed file)

From the files: a **status-bar note** while the signed file has unsaved changes: signature glyph, "Signed file." in `text`, the rest `muted`.

```html
<span style="display: flex; align-items: center; gap: 7px; font-size: 12px; color: #6B645A">
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M4 19c2-1 3-4 5-4s1 3 3 3 2-2 4-2"></path><path d="M14 4.5l5 5-7 7h-2v-2z"></path></svg>
  <span><span style="color: #1F1D1A">Signed file.</span> Saving adds your changes after the signature; the signature stays valid.</span>
</span>
```

**Banner (proposed)** when saving would *change* what a signature covers: a strip under the top bar (and secondary toolbar), 36 px min, `warning-tint`, `hairline` bottom, signature glyph and lead word in `warning`, 13 px `text`, actions right. It appears with the first edit that triggers it (not at save time) and stays until the edit is undone or saved. Wording: "**Saving will invalidate the signature.** This file is certified to allow no changes. Save a copy instead to keep the original signed." with buttons "Save a copy…" (secondary; “Download a copy” on the web, the labels of `file.save-copy` in §8.4) and "Details" (ghost).

**Constraint banner (proposed, from the editor areas):** the banner is a labelled `region` that names each signature and offers **Save a copy…** (“Download a copy” on the web) · **Apply anyway…** (both secondary, 26 px) · **Details** (ghost). Apply anyway opens an `alertdialog` that lists each signature as a **signature row** (white card: signature glyph, signer 600, kind badge — Certification `accent`, Valid `positive` — and a 12 `muted` line with field, scope and date) and confirms with a danger-filled button naming the result (“Crop and invalidate signature”). It is one of the strips in §7.29.

- **Timing in certified files (proposed, Comment):** in a mode where every tool makes a change the certification forbids, the banner shows on mode entry instead of at the first offending edit: Comment mode at DocMDP P1 and P2, Fill & Sign at P1. Everywhere else the first-edit rule stands. The wording names the level (“certified … for form filling only” or “to allow no changes”).
- **Page changes in certified files (proposed, Edit session):** before Apply anyway the banner ends “… so nothing has been deleted”; after it, “Saving will invalidate the certification.”; Save a copy… then runs the remove-signature confirm worded for the copy.

### 7.14 Bottom bar (phone)

```html
<nav aria-label="Reader tools" style="position: absolute; left: 0; right: 0; bottom: 0; height: 84px; box-sizing: border-box; padding: 6px 8px 22px; display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 4px; background: #F7F4EE; border-top: 1px solid #E4DED3">
  <button style="height: 56px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 3px; border: 0; background: transparent; border-radius: 6px; color: #3A3631; font-size: 11.5px"><svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6"></circle><path d="M15 15l5 5"></path></svg>Search</button>
  <!-- Annotate, Pages, Reader, More: same button, icons from §5 -->
</nav>
```

An open destination (Pages sheet shown) **(proposed)**: icon and label `accent`, label 600, `aria-expanded="true"` on the button.

### 7.15 Bottom action bar (phone, text selected)

Same grid, 76 px, padding `4px 8px 18px`, buttons 52 high, `role="toolbar" aria-label="Selection actions"`. The active tool: `accent-tint` fill, `accent` 600 label.

```html
<div role="toolbar" aria-label="Selection actions" style="position: absolute; left: 0; right: 0; bottom: 0; height: 76px; box-sizing: border-box; padding: 4px 8px 18px; display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 4px; background: #F7F4EE; border-top: 1px solid #E4DED3">
  <button style="height: 52px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 3px; border: 0; background: transparent; border-radius: 6px; color: #3A3631; font-size: 11.5px"><svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><rect x="8.5" y="8.5" width="11" height="11" rx="1.5"></rect><path d="M15.5 8.5V6a1.5 1.5 0 0 0-1.5-1.5H6A1.5 1.5 0 0 0 4.5 6v8A1.5 1.5 0 0 0 6 15.5h2.5"></path></svg>Copy</button>
  <button style="height: 52px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 3px; border: 0; background: #E1E6E3; border-radius: 6px; color: #1F5C63; font-size: 11.5px; font-weight: 600"><svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M14.5 4.5l5 5L11 18H6v-5z"></path><path d="M4 20.5h9"></path></svg>Highlight</button>
  <!-- Comment, Share, More -->
</div>
```

Touch selection handles: 2×16 px `accent` caret with a 10 px `accent` dot (above the start, below the end), `aria-hidden`.

### 7.16 Bottom sheet

```html
<section aria-label="Pages" style="position: absolute; left: 0; right: 0; top: 404px; bottom: 76px; background: #FBF9F5; border-top: 1px solid #E4DED3; border-radius: 12px 12px 0 0; box-shadow: 0 -8px 28px rgba(60,48,30,0.12); display: flex; flex-direction: column; overflow: hidden">
  <div style="display: flex; justify-content: center">
    <button aria-label="Resize sheet" style="width: 64px; height: 44px; display: flex; align-items: flex-start; justify-content: center; padding-top: 8px; box-sizing: border-box; border: 0; background: transparent"><span style="width: 36px; height: 5px; border-radius: 3px; background: #8A8174"></span></button>
  </div>
  <div style="display: flex; align-items: center; justify-content: space-between; margin-top: -18px; padding: 0 8px 8px 20px">
    <div style="display: flex; align-items: baseline; gap: 10px">
      <h2 style="margin: 0; font-family: 'Newsreader', Georgia, serif; font-size: 20px; font-weight: 500">Pages</h2>
      <span style="font-size: 13px; color: #6B645A; font-variant-numeric: tabular-nums">24 pages</span>
    </div>
    <button aria-label="Close pages" style="width: 44px; height: 44px; display: flex; align-items: center; justify-content: center; border: 0; background: transparent; border-radius: 6px; color: #4A453E"><svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18"></path></svg></button>
  </div>
  <div style="display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 18px 16px; padding: 4px 28px 16px"><!-- thumbnails 122×158 --></div>
</section>
```

**Modal and non-modal sheets (proposed, Reader A):** a sheet that asks a question or holds a form is a modal dialog (`role="dialog"`, `aria-modal="true"`, focus trap). A sheet that lists results the user steps through with a toolbar that stays visible (phone find Results, R05) is non-modal: a labelled `section`, no focus trap, and the toolbar and page stay usable.

### 7.17 Page indicator pill (phone)

```html
<div aria-live="polite" style="position: absolute; left: 50%; bottom: 100px; transform: translateX(-50%); height: 30px; display: flex; align-items: center; padding: 0 14px; border-radius: 15px; background: rgba(31,29,26,0.78); color: #FFFFFF; font-size: 13px; font-weight: 500; font-variant-numeric: tabular-nums; letter-spacing: 0.02em">3 / 24</div>
```

**Hint state (proposed, Comment):** for the first three sessions of a mode that takes over one-finger gestures, the pill may show a one-line hint (“Use two fingers to scroll”) in place of the page number; it moves up to sit 16 px above whatever bar is showing.

### 7.18 Dialog

Built like the palette: `scrim`, centred card `max-width: 440px` (forms up to 560; **wide, proposed:** 820 for a live preview or a two-pane editor, with an options or tree column (min 260) and a `gutter`-coloured preview column (min 400) that stack below 700 px, header and footer over `hairline` rules, a summary at the left of the footer), `raised` = `surface` in light, `chip-border` edge, radius 6, palette shadow, padding 24, gap 16. Title Newsreader 20/500; body 14 `text`; actions right-aligned, gap 8, primary last (danger-filled for destructive). Close icon button top right unless the dialog asks a required question. `role="dialog"` (or `alertdialog` for destructive confirms), `aria-modal="true"`, `aria-labelledby` the title, focus trapped, Escape closes, focus returns to the opener. Phone: becomes a bottom sheet at the large detent with actions full width (44 px high).

Rules **(proposed)**:

- **Destructive confirm:** an `alertdialog` whose confirm button is danger-filled starts with focus on Cancel, lists the consequences (each with an 18 px icon), names any signatures it will remove in a `warning-tint` notice, and repeats the count on the button (“Apply 10 redactions”, “Flatten 14 comments”).
- **Live preview:** a page in thumbnail style (232 × 300) on a `gutter` well (radius 4, padding 16); new marks in their output colour inside 1 px dashed `control-border` boxes; a page stepper with a live region; `role="img"` with a describing note.
- **Server consent block:** the required step before any upload, inside the dialog (never a separate pop-up): white with a `control-border` edge; Share icon in `accent` + heading (“This conversion runs on lamina’s server”) + why the device can't do it and that nothing is sent until the person agrees; a consent definition list (`dl` in a 112 px / 1fr grid between `hairline` rules, terms 12/600 `text-3`) with four rows — What is sent · Where to · How long it stays · What it is used for — worded for the job **(proposed, Convert and produce: see its [consent wording per job](convert-compare-produce.md#consent-wording-per-job))**: the heading names the job (“This conversion runs on lamina’s server”, “Reading the text runs on lamina’s server”); What is sent says exactly what leaves (the whole file with its size, only the chosen pages, an address only, or *n* files with their total size); Where to uses the destination names of [Account and cloud](account-cloud-product.md) and names any other host the server will contact; How long it stays ends at download and at 24 hours at most; What it is used for names the one job and ends “Not kept, not used for anything else.”; a checkbox whose label names what is sent (file, pages, address or file count), unchecked by default and never remembered. The primary (“Upload and convert”, “Upload and read text”, “Send and convert”, “Upload and run”) stays disabled with a visible reason until it is ticked. The retention promise is set before upload; panels after upload repeat it and offer only “Delete from server now”, never a longer stay.
- **Popover dialog:** a non-modal `role="dialog"` anchored to its button with menu styling (§7.28), for pickers (stamps) that hold a tablist.

```html
<div role="dialog" aria-modal="true" aria-labelledby="dlg-t" style="width: 100%; max-width: 440px; box-sizing: border-box; padding: 24px; display: flex; flex-direction: column; gap: 16px; background: #FBF9F5; border: 1px solid #D8D1C5; border-radius: 6px; box-shadow: 0 18px 48px rgba(31,29,26,0.22)">
  <h2 id="dlg-t" style="margin: 0; font-family: 'Newsreader', Georgia, serif; font-size: 20px; font-weight: 500; line-height: 1.2">Move pages</h2>
  <p style="margin: 0">Move pages 4–6 to a new position. You can undo this in one step.</p>
  <div style="display: flex; flex-direction: column; gap: 8px">
    <label for="mv" style="font-size: 12px; font-weight: 600; color: #4A453E">Move after page</label>
    <input id="mv" value="12" style="height: 32px; box-sizing: border-box; padding: 0 10px; border: 1px solid #8A8174; border-radius: 4px; background: #FFFFFF; font: inherit; font-variant-numeric: tabular-nums">
  </div>
  <div style="display: flex; justify-content: flex-end; gap: 8px">
    <button style="height: 32px; padding: 0 16px; border: 1px solid #8A8174; background: #FFFFFF; border-radius: 4px">Cancel</button>
    <button style="height: 32px; padding: 0 16px; border: 1px solid #1F5C63; background: #1F5C63; color: #FFFFFF; border-radius: 4px; font-weight: 600">Move</button>
  </div>
</div>
```

### 7.19 Form controls (consistent with the page input and note field)

In `@laminapdf/ui`: `TextInput`, `TextArea`, `Select`, `Checkbox`, `RadioGroup` with `Radio`, and `Switch`. The switch is a visually hidden `<input type="checkbox" role="switch">` inside its label, like the checkbox below, rather than a `<button role="switch">`: it reports the same role and state, and keeps the whole row as its target.

All: `control-border` 1 px, white fill, radius 4, 14 px text (16 on touch), label above (12/600 `text-3`) with gap 8, help text below (12 `muted`), error text below (12 `danger`, with `aria-describedby` and `aria-invalid="true"`; the border turns `danger`). Focus: the 2 px `accent` ring, offset 1.

| Control | Size | Details |
| --- | --- | --- |
| Text input | 32 high (desktop), 44 (touch), padding `0 10px` | Placeholder `muted`; never use placeholder as label |
| Textarea | padding `8px 10px`, line height 1.45, `resize: vertical` | From the inspector |
| Select | 32 high, padding `0 30px 0 10px`, chevron `<path d="M7 10l5 5 5-5"></path>` 16 px `text-3` at right 8 | Native `<select>` styled with `appearance: none` |
| Checkbox | 16×16 box, radius 3, in a ≥ 24 px row; off: white, `control-border`; checked: `accent` fill, white check `<path d="M6 12.5l4 4 8-9"></path>` stroke 2; mixed **(proposed)**: `accent` fill with an 8 × 2 white dash; disabled: `ground` fill, `chip-border` edge, label `muted` | Label 14 px, gap 8 |
| Radio | 16×16 circle; off: white, `control-border`; selected: `accent` 2 px ring + 8 px `accent` dot | In a `role="radiogroup"` / `<fieldset>` with `<legend>` |
| Toggle (switch) | track 32×18 radius 9; off: white, `control-border`; on: `accent` fill; knob 12 px (off `control-border` colour, on white) | `<button role="switch" aria-checked>`; label left of it; use only for settings that apply at once |

**How to build check boxes and radios (decided, see [Decisions on area proposals](#decisions-on-area-proposals)):** a real `<input type="checkbox">` or `<input type="radio">`, visually hidden inside its `<label>`, followed by a drawn box or circle (`<span aria-hidden="true">`). The whole label row (≥ 24 px; 44 on touch) is the target, the native input keeps keyboard, form and screen-reader behaviour, and the ring is drawn on the box with `label:has(input:focus-visible)`. The mixed state is the input's `indeterminate` property. Don't use `role="checkbox"` buttons, and don't rely on `accent-color` (it can't set the edge colour).

```html
<label style="display: flex; align-items: center; gap: 8px; min-height: 24px">
  <input type="checkbox" checked style="position: absolute; width: 1px; height: 1px; opacity: 0; margin: 0">
  <span aria-hidden="true" style="width: 16px; height: 16px; box-sizing: border-box; border-radius: 3px; border: 1px solid #1F5C63; background: #1F5C63; display: flex; align-items: center; justify-content: center; color: #FFFFFF"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 12.5l4 4 8-9"></path></svg></span>
  Single-key shortcuts
</label>
<div style="display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 24px">
  <span id="sw-l">Show selection toolbar</span>
  <button role="switch" aria-checked="true" aria-labelledby="sw-l" style="width: 32px; height: 18px; padding: 0; border: 1px solid #1F5C63; border-radius: 9px; background: #1F5C63; position: relative"><span style="position: absolute; top: 2px; right: 2px; width: 12px; height: 12px; border-radius: 50%; background: #FFFFFF"></span></button>
</div>
```

More controls **(proposed)**:

| Control | Spec |
| --- | --- |
| Password field | The text input with an inline “Show” text toggle (13/600 `accent`, `aria-pressed`) inside the right edge |
| Keyword chips input | A 32 px min field holding 26 px chips (`ground` fill, `chip-border`, radius 3) with 24 px remove buttons (12 px icon), so the remove target meets the 24 px desktop minimum (§8.9), and a borderless input filling the rest |
| Filter chip | 24 px, white, `control-border`, radius 3, 12 px, a 12 px close glyph; the whole chip is a button (“Remove filter: Open”) |
| Segmented radio group | `role="radiogroup"` grid of equal buttons, 28 px (44 touch), gap 4, radius 4 (6 touch). Off: white + `control-border`; on: `accent` border, `accent-tint`, `accent` 600 |
| Toggle group | Joined 30 × 28 buttons in one `control-border` box (radius 4) with `hairline` dividers; pressed = `accent-tint` and `accent`. `role="group"` with a visible label, each button `aria-pressed` (bold, italic, alignment) |
| Radio card | A label holding a radio, a title (14) and one line (12 `muted`); padding `10px 12px`, radius 4, `control-border`, white; selected = `accent` border, `accent-tint`, title `accent` 600 (export formats, any format picker) |
| Option row with help | Radio label 14, help 12 `muted` indented 24 under it, rows split by `hairline`; the group is a `radiogroup` labelled by the section heading |
| Anchor grid | 3 × 3 `radiogroup` of 28 × 28 buttons, gap 4, radius 4, `control-border`; checked = `accent-tint` with an 8 px `accent` dot; disabled = `hairline` border and `disabled` dot |
| Stepper | Two secondary buttons (32 px; 44–56 touch) around an `<output aria-live>` value (“Smaller text” / “Larger text”, glyph `A` at two sizes) |
| Number stepper (touch) | 44 × 44 − and + buttons (radius 6, `control-border`) around a 72 × 44 input (16 px, tabular) |
| Ranges table | `<table>` with 12/600 `text-3` headers, `hairline` row rules, 32 px controls, a trailing 28 px remove button; each control labelled with its row and column |
| Licence key field | The input in `mono`; on the account page read-only and masked except the last group (Show / Copy key) |
| Disabled secondary button | `ground` fill, `chip-border` edge, `muted` label, `aria-disabled="true"` with the reason as its description |

### 7.20 Toast and notice

In `@laminapdf/ui`: `Notice`, and `Toast` for the toast's look; the toast region and its timing come with the overlays. **Toast colours (U6h):** the toast swaps the `text` and `surface` colours, so it is dark in the light theme, light in the dark theme and reads in high contrast; in the light theme its text is `surface` (`#FBF9F5`) rather than white.

- **Toast:** short confirmation of an action that already happened ("Copied", "Highlights exported"). Bottom centre, 24 px above the status bar (phone: above the bottom bar, where the pill sits; it replaces the pill). `text` bg `#1F1D1A`, white 13 px (16.81), radius 6, padding `8px 14px`, floating shadow; an optional action is white underlined text ("Undo"). `role="status"`; 4 s, paused while hovered or focused; never the only place an error is reported. **Persistent toast (proposed):** the same with two text actions, staying until acted on; only for something that waits on the user's consent (“Update ready · Restart · Later”).
- **Partial undo (proposed, Edit session):** when an undo applies only partly (`UndoOutcome::Partial`), a toast with normal timing plus a `warning-tint` notice at the top of the History tab until the next edit, since a toast is never the only place.
- **Notice:** an inline strip inside a panel or under the bars for a lasting state (repaired file details, approximate reflow, offline). Tint background (`warning-tint`, `danger-tint`, `positive-tint` or `surface`), `hairline` edge, radius 4, padding `10px 12px`, 16 px icon in the status colour, title 600 + body 13 px `text`. No left-border accent stripe.

```html
<div role="status" style="display: flex; gap: 10px; padding: 10px 12px; background: #F0E8DC; border: 1px solid #E4DED3; border-radius: 4px; font-size: 13px">
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#8A5300" stroke-width="1.5" aria-hidden="true"><path d="M12 4l9 16H3z"></path><path d="M12 10v4M12 17v.5"></path></svg>
  <span><strong style="font-weight: 600">Approximate layout.</strong> This file has no tags, so lamina guessed the reading order.</span>
</div>
```

### 7.21 Empty state

In `@laminapdf/ui`: `EmptyState`.

Centred in the region it fills, max-width 360, gap 12: optional 32 px stroke icon `muted`; title Newsreader 20/500 (Display 40/44 only on the home screen); one sentence 14 `muted`; at most one primary and one ghost action. Example: Outline tab of a file without bookmarks, "This file has no outline" / "Use Pages to move through it."

**Error page (proposed):** the empty state with a 32 px `danger` Warning icon, the error code in 12 `muted` under the sentence, `role="alert"`, one primary and one ghost action; focus moves to the primary action.

**Drop zone (proposed):** `surface`, 1 px dashed `control-border`, radius 6. Large (home screen, start window): padding `40px 24px`, centred 32 px File or Download icon, title Newsreader 20/500, hint 14 `muted`, buttons gap 8. Compact (inside a panel or dialog, editing modes only): padding `18px 12px`, 22 px `muted` icon, a 13 px line and a 12 px `muted` line. Always paired with a button that does the same. Drag-over: a full-window `accent-tint` overlay with a 2 px dashed `accent` inset and one line of text.

**File list row (proposed):** full-width button, min-height 56 (64 touch, 72 on the phone Files list), padding 8, gap 14: 20 px icon, Newsreader 15/500 name over a 12 `muted` meta line, date right; rows split by `hairline`. A missing file shows a dashed placeholder, the name in `muted`, “File not found” and Remove. On desktop, recent files may be cards with the last page viewed as a thumbnail.

### 7.22 Progress

In `@laminapdf/ui`: `ProgressBar` (determinate with a `value`, indeterminate without) and `Skeleton`. **Skeleton colour (U6h):** `hairline`, the nearest semantic token to `#ECE7DE`, so skeletons follow the theme.

- **Determinate bar:** 4 px track `hairline`, radius 2, `accent` fill (5.66 on the track), label above (12 `muted`, tabular: "Saving… 62%"), `role="progressbar"` with `aria-valuenow`.
- **Indeterminate:** the same bar with a 30% segment sliding (1.2 s, linear); with `prefers-reduced-motion`, a static 30% segment plus the label.
- **Spinner:** avoid; pages use placeholders, sidebars skeletons (`#ECE7DE` blocks, no shimmer under reduced motion).

### 7.23 Badge and tag

In `@laminapdf/ui`: `Badge`, which also draws tags.

Height 20, padding `0 8px`, radius 3, 12 px 600, tint background with the status colour as text: neutral `surface` + `chip-border` + `text-3`; accent `accent-tint` + `accent`; positive `positive-tint` + `positive` ("Signature valid"); warning `warning-tint` + `warning` ("Approximate"); danger `danger-tint` + `danger` ("Leak found"). Pair with an icon or word; never colour alone.

Badge sets **(proposed)**, all at this size:

- **Severity:** “Error” (danger) and “Warning” (warning), always a word.
- **Outcome:** “Replaced · n” (positive, Check), “Not replaced” (warning, Warning), “Skipped by you” (neutral).
- **Review status:** Accepted (positive, check), Rejected (danger, cross), Done (accent, check), Cancelled (neutral); open threads have none.
- **Capability:** Round-trip (positive, check), Write (accent), Read (neutral), Unsupported (dashed `control-border` outline, neutral).
- **Sync:** each with an icon (Synced, Uploading, Conflict copy, Waiting).
- **Pending removal:** a struck-through `muted` value, danger badge “Will be removed”, and a “Keep” ghost link.
- **Positive badge list:** positive badges with the Check icon as a labelled list of what something leaves out (crash report preview).
- **Count pill:** 16 px high, `accent`, white 11/600 tabular (bell count); the count is also in the button's `aria-label`.

```html
<span style="display: inline-flex; align-items: center; gap: 4px; height: 20px; padding: 0 8px; border-radius: 3px; background: #E7EBE3; color: #2F6A45; font-size: 12px; font-weight: 600"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M12 3.5l7 2.5v5.5c0 4.3-3 7.6-7 9-4-1.4-7-4.7-7-9V6z"></path><path d="M9 12l2.2 2.2L15.5 10"></path></svg>Signature valid</span>
```

### 7.24 Secondary-toolbar tool toggle

In `@laminapdf/ui`: `ToolToggle`, and `ToggleButton` for other pressed buttons.

From the Comment toolbar: 28 high, padding `0 10px`, gap 6, radius 4, 16 px icon + 13 px label, 1 px transparent border; selected = `accent` border, `accent-tint`, `accent` 600. Right end: a 12 `muted` context note ("1 comment on this page").

```html
<button aria-pressed="true" style="height: 28px; display: flex; align-items: center; gap: 6px; padding: 0 10px; border: 1px solid #1F5C63; background: #E1E6E3; color: #1F5C63; border-radius: 4px; font-weight: 600"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M14.5 4.5l5 5L11 18H6v-5z"></path><path d="M4 20.5h9"></path></svg>Highlight</button>
```

**Split tool button (proposed):** the toggle (right radius 0) plus a 20 × 28 chevron part (left radius 0, 14 px chevron, `aria-label="Choose <thing>"`, `aria-haspopup="menu"`). The toggle part is the last-used item and carries the pressed state (Text box ▾ in Comment; Shape ▾ in Edit, A01). **Menu tool button:** the toggle with a 14 px chevron after the label, `aria-haspopup="menu"` (Shapes ▾ in Comment, Stamp ▾, Page setup ▾, Form data ▾); when the active tool lives inside its menu, the menu button shows selected.

**Tool toolbar (proposed):** the secondary toolbar of a tool that works inside a mode (Redact, Protect, Form design, Accessibility, Print production, Measure). Same metrics; it starts with the tool's 16 px icon and its name in Newsreader 15/500 and ends with a context note and **Done** (secondary), which returns to the mode's own toolbar.

### 7.25 Page-appearance chips

In `@laminapdf/ui`: `PageAppearanceChips`.

From the style tile: 4-column grid gap 10; chip padding 8, gap 8, radius 6; preview 52 high, radius 3, "Aa" in Source Serif 4 20 px on the appearance's paper and ink; selected chip white with a 2 px `accent` border and a 13/600 label, others `surface` with `chip-border` and 13/400. `role="group"` + `aria-pressed`.

**Theme chips (proposed):** the same anatomy with a 52 px preview of the chrome ground and a small white page; Match system splits light and dark.

**Compact chips (proposed, Reader B R10):** in popovers narrower than 400 px and in phone sheets the chips sit in a 2 × 2 grid: a 40 × 32 `Aa` preview to the left of a 13 px label, padding 6, gap 10, radius 6; selected as above. Keeps “High contrast” on one line.

**Theme radio grid (proposed, Reader B R10):** the Theme radio group (§7.19) in a 2 × 2 grid, rows 32 px (44 on the phone), order Light · Dark · High contrast · Match system. In the Settings window (S07) the same radio group may be drawn as preview cards (preview, drawn radio, label) in that order.

**Accent swatches (proposed, second review):** under Theme, a `role="radiogroup"` named “Accent” of four 32 px round swatches (44 px targets on touch) named Teal, Blue, Plum and Green, with the §7.12 selected ring and the selected name beside the row; disabled in High contrast with the help line “High contrast uses your system colours” (§2.8).

### 7.26 Menu (proposed)

One menu for every lamina-drawn menu: toolbar menus (Shapes, Insert, Form data, Filter), More menus, the More tabs menu, context menus and the tab overflow menu. Native menus (the macOS menu bar, Windows and Linux menu bars) are drawn by the OS; the desktop boards only sketch them.

- `role="menu"` popover (z 30) under its button: `surface`, `chip-border` edge, radius 6, floating shadow, padding 4, width 260 (wider when needed). The opening button shows the `pressed` fill and `aria-expanded="true"`.
- Items 32 high (44 on touch), padding `0 10px`, gap 10: 16 px icon `text-3` (items without an icon indent to 36), 14 px label, shortcut chip at the right. Check and radio items (`menuitemcheckbox`, `menuitemradio`) show the Check icon in the icon column.
- Active item: the palette's active-row style (`accent-tint`, inset 1 px `accent` ring). Disabled: `disabled` text, `aria-disabled`, the reason in a tooltip. A destructive item goes last, in `danger`.
- Separators 1 px `hairline` with 6 px side margins; optional overline group headers; an optional 12 `muted` note or footer hint below the items, outside `role="menu"`, linked with `aria-describedby`.
- Submenus end with the Chevron right icon. Context menus flip on collision and prefer not to cover the selection.
- **Menu with descriptions:** padding 6, width 320–360; items padding `8px 10px`, optional 18 px icon, title 14/600 over a 12 px sentence (`muted`, `text-3` when active). For choices that must be told apart by what they do (Save options, the signature-field menu). An optional explanation box (`ground` fill, overline title) may follow.
- **Listbox popup:** a dropdown field's open list; as the menu, with items 28 high and `role="listbox"`.
- **Menu sheet (phone):** a sheet holding a `role="menu"` list of 52–56 px rows: 22 px icon, 16 px label, optional 13 px `muted` subline or value on the right.

### 7.27 Tooltip and link preview (proposed)

- **Tooltip:** `surface`, `chip-border` edge, radius 6, floating shadow, 12 px text plus a shortcut chip, 6 px below the control; `role="tooltip"`; shown on hover after 500 ms and on keyboard focus; hoverable; Escape dismisses.
- **Link preview:** popover (z 30) 248 px, `surface`, `chip-border`, radius 6, floating shadow, padding 12, gap 10: a 92 px crop of the target page's top (thumbnail shadow) over the target title (Newsreader 15/500) and a 12 `muted` line (“Link to page 11 · Enter to go”). Shown after 300 ms on hover or focus; `role="tooltip"`, hoverable, Escape dismisses.
- **Address box:** a URL shown before acting on it: white, `hairline` edge, radius 4, padding `10px 12px`, wraps anywhere, host in 600 `text` (punycode for international hosts), the rest `text-3`.

### 7.28 Popover (proposed)

A non-modal `role="dialog"` anchored to its button: `surface`, `chip-border` edge, radius 6, menu shadow, padding 16 (20 for the update popover), gap 12–18, title Heading 18. Sections use a 12/600 `text-3` label over the control and 12 `muted` help; a footer over a `hairline`. Uses: View settings (360 px), the popovers of status-bar items (340 px, anchored above the status bar), notifications, pickers with a tablist (stamps).

### 7.29 Strip (proposed)

A full-width strip under the top bar and secondary toolbar for a lasting state of this window: min-height 36, padding `6px 16px`, `hairline` bottom, 16 px icon, a lead sentence in 600, an optional 12 px `text-3` second line, actions on the right (28 px high: ghost · secondary), and a dismiss button when the state allows it. `role="alert"` when it asks for a decision, `role="status"` when it informs. The tint says what kind of state it is:

| Tint | Use |
| --- | --- |
| `warning-tint`, lead in `warning` | Saving would change a signature (§7.13), the file changed on disk while there are unsaved changes, a certified file that forbids comments |
| `danger-tint`, lead in `danger` | Disallowed changes after signing; saving blocked by a report |
| `accent-tint`, lead in `accent` | A past revision is open (Clock icon), an editing lease |
| `surface` | Offline, session restored, a retrying reload |

A notice (§7.20) is the same message inside a panel. Strips never stack: the more serious one wins.

### 7.30 Find bar and results (proposed)

- **Find bar (desktop and web):** takes the secondary-toolbar slot, `role="search"`: a 28 px field (300 px, search icon, the count inside in `muted` tabular: “3 of 9”, “40+” while counting, “No matches”), Previous and Next 28 px icon buttons, a divider, Match case and Whole words toggles (§7.24 without icons), a context note and Close. In editable files a **Replace** disclosure adds a second row: a 300 px field, Replace, Skip, a primary “Replace n checked” and a note.
- **Results list (rail tab “Results”):** page group labels 12/600 `text-3` (“Page 3 · Repairs backlog”), result links 13/1.45 with the match in 600, padding `6px 8px`, radius 4; current result `accent-tint` + inset `accent` ring. **With change preview** (replace): a checkbox row whose label shows the context with the old words struck through in `muted` and the new words 600 in `accent`.
- **Phone find (one design for every area):** the top bar becomes a 44 px search field (16 px text, count inside) with **Done**; the bottom bar becomes a three-column toolbar (Previous · Results · Next) with the action-bar sizes (§7.15).
- Marks on the page use §2.6.

### 7.31 Lists and tables (proposed)

| Component | Spec |
| --- | --- |
| Data table in a panel | 13 px tabular, header row 12 `muted` over a `hairline`, rows with 6 px padding over hairlines, a visually hidden `<caption>` |
| File table | A real `table`; name in Newsreader 16; sync badges with an icon each; inline 160 px progress for uploads |
| Capability table | Rows split by hairlines; row-group headers in Newsreader 18 on `ground`; capability badges (§7.23) |
| History list | Rail tab rows (padding `6px 8px`, radius 4): 14 px label over a 12 `muted` meta line; current row `accent-tint` with an inset `accent` ring; undone rows struck through with “Undone” in the meta |
| Timeline | Ordered list with a 9 px dot and a 1 px `chip-border` connector per row; the open item has a filled `accent` dot and the selected-row treatment |
| Ordered file list | White list in a `hairline` box, 52 px rows: drag grip, position, name over meta, and 28 px Move up / Move down / Remove buttons (disabled where a move isn't possible) |
| Thread row | Comments panel: type mark (fill swatch or line glyph), author, type, date, status badge, quote (Newsreader italic), note, reply count; the header is a button that selects the annotation. The open thread uses the active-row style, never a left stripe |
| Bulk action footer | A `ground` strip with a `hairline` top at the bottom of a panel: count in 600, actions right |
| Fields tab rows | Overline per page (“Page 1 · 12 fields”), rows 28 high: number 12 `muted` tabular · label 13 · state (✓ `positive`, “Required” `danger`, “Read-only”/“Calculated” `muted`, “Script skipped” `warning`); current `accent-tint` + `accent` 600, `aria-current` |
| Review row | Text and kind, then 28 px Accept (`accent` border and tint when pressed) and Reject (`danger` border and tint when pressed) toggle buttons; rejected text struck through in `muted` |
| Check row (reports) | Name 13/600 + badge on one line, how it was checked (12 `text-3`) below; a failing row expands into a `danger-tint` box with the cause and fix buttons |
| Notification row | Link row, padding 10, radius 4: avatar, sentence with the actor in 600, excerpt 13 `text-2`, context 12; unread rows on `accent-tint` with an 8 px `accent` dot and “New” 12/600 |
| People row | 48 px: avatar, name over email (or source line), access or role select (28 high), a remove icon button named for the person |

### 7.32 Page overlays (proposed)

Drawn over the page (layer 1, §4) and `aria-hidden` unless stated. Every overlay that can be dragged has inspector fields and arrow keys as its single-pointer alternative (WCAG 2.5.7).

| Overlay | Spec |
| --- | --- |
| Selection handles | 8 × 8 white squares with a 1.5 px `accent` border (7.58 on white) at corners and edge midpoints, 4 px outside a 1 px `accent` outline, each in a 24 × 24 hit area. Rotation: a 10 px white circle on a 1 px stem 24–30 px above the top edge. Text boxes show only the side handles. Line points (callout knee and end): round 10 px handles. Crop frames, measurements and design-mode fields use the same squares |
| Point handles | Anchor: 8 px square, the selected one filled `accent`; control: a 6 px circle on a 1 px line |
| Snap guide | 1 px dashed `accent` line with a label chip (11 px white on `accent`, radius 3) in the page margin, only during a drag |
| Crop overlay | Kept area: 2 px `accent` frame; cut area `rgba(31,29,26,0.38)` over the page; handles as above, 6 px outside the frame |
| Destination and bead marker | 2 px `accent` line (dashed for a bookmark destination) or outline (thread bead) with a 16–18 px `accent` tag (white 11/600, radius 3) placed in white space, never over text |
| Editable-run outline | 1 px dashed `control-border`, offset 3, on every editable run while the Edit text tool is active |
| Substituted-glyph mark | 2 px dotted `warning` underline, offset 3, in the editor only |
| Field overlay (fill) | The native control over the widget, square corners, 1 px `control-border`, `field-tint` fill, page ink in Source Sans 3 at the widget's size. Focused: white fill + 2 px `accent` ring, offset 1. Required: `danger` asterisk after the printed label, `aria-required`. Check box 14 px square, radio 14 px circle with an 8 px ink dot, built as in §7.19 |
| Read-only and calculated field | 1 px dashed `control-border`, white, 14 px Lock `text-3`, text `text-2`, `aria-readonly`; calculated adds the Calculated icon, the value 600 right-aligned and a 12 px caption naming the formula, as an `<output>` |
| Field error and message | 2 px `danger` border; under the field a message: `danger-tint`, 1 px `danger` edge, radius 4, padding `6px 8px`, 14 px Warning, 12 px text, `role="alert"`, referenced by `aria-describedby` |
| Design-mode field | 1 px dashed `accent` edge, `field-tint` fill, field name in `mono` 11 `accent`. Selected: solid edge + 2 px `accent` outline offset 2 and the selection handles; a size chip under it (`text` background, white 11 tabular, radius 3) |
| Tab-stop badge | 20 px pill, `accent`, white 11/600 tabular, at the field's top-left (−11, −11) |
| Suggestion outline | Pending: 2 px dashed `accent` outline, offset 2, with an 18 px type tag (`accent`, white 11/600) inside it at the right end; current: solid; accepted: becomes a design-mode field with a `positive-tint` “Accepted · Text” tag |
| Reading-order badge | 22 px `accent` circle with a white 12/600 number in the page margin beside each tagged block; artifacts get a neutral “Artifact” tag |
| Redaction mark | `danger-tint` fill with a 1.5 px `danger` outline (accepted) or dashed outline (to review); preview and applied: a solid `text` box (`#1F1D1A`) with optional overlay text, `role="img"` named “Redacted” |
| Remote selection | 2 px outline in the reviewer's presence colour, offset 2, with a 20 px name tag (white 12/600 on the colour, radius 3) at the top right |
| Pen hover preview | 2 px `accent` caret plus a 14 px ring (2 px `accent` over 60% white), only while a pen hovers |
| Target (proposed, Sign/protect/redact) | The page object a dialog, menu or sheet acts on (the field being signed behind G01's menu, G02, G03, G11's sheet): 1 px `accent` outline, offset 2, on an `accent-tint` fill. It is not the focus ring, so a board still shows one 2 px ring |

### 7.33 Desktop windows (proposed)

- **Document tab strip:** 34 px row on `#EFEBE3`, `#E0D9CD` rule below. Tabs `flex: 0 1 220px`, min 120, padding `0 4px 0 12px`, 1 px `#E0D9CD` dividers; label 13 px (active `text` 600, others `muted`, ellipsis); the active tab sits on `ground` with `margin-bottom: -1px` so it joins the top bar; hover `hover` fill. A 24 × 24 close button with a 14 px cross on the active, hovered and focused tab. When tabs don't fit at 120 px, a “+N” overflow button (26 px, chevron, pressed = toggle-selected style) opens a menu (§7.26) of all tabs; an add button (26 px, Plus). `role="tablist"` with roving tabindex; hidden with one tab.
- **Settings window and settings page:** a section list (200 px in the desktop window, 220 px on the web; `ground`, `hairline` right, padding `16px 10px`, 32 px link rows, radius 4; current `accent` 600 on `accent-tint`, `aria-current="page"`); content on `surface` with padding `28px 32px` (max 960); setting rows: label + 12 px help on the left, control on the right, 12 px vertical padding, `hairline` between. **One section list (proposed, Desktop shell S07 and Account and cloud H02):** General · Appearance · Updates · Privacy · Shortcuts · Licence · Account and plan · Notifications · Advanced · About; the web page drops Updates and Advanced.

### 7.34 Rail panel and view bar (proposed)

- **Rail panel:** a rail view that replaces the tabs for one task (Revisions, Layers): a 28 px back button (“Back to sidebar tabs”), Newsreader 18 title, a 12 `muted` count; 260–280 wide. The standard way to give the rail a fourth task without a fourth tab.
- **View bar:** for full-canvas views in Read mode (the object inspector): the secondary toolbar's box holding Back to pages (ghost, Back icon), a divider, a 40 px tab row (§7.5 style), a neutral “Read only” badge and a `muted` facts line. Not a mode toolbar: no tools.

### 7.35 Touch bars and sheets (proposed)

| Component | Spec |
| --- | --- |
| Annotate bar (phone) | The bottom-bar grid reused for Comment tools, with a 60 px `surface` options strip above it for colour and width |
| Tool shelf (tablet stylus) | `role="toolbar"` of 48 px icon-over-label buttons (22 px icons) │ four line colours in 44 px buttons │ widths │ a “Draw with finger” switch |
| Form bar (phone) | `surface`, `hairline` top, sheet shadow, padding `8px 8px 8px 16px`: label 15/600 + position 13 `muted` tabular; hint 13 `text-3` (error `danger`); 44 px Previous / Next (secondary, radius 6), ghost “Next required”, primary Done. Rides on the keyboard (`visualViewport`) and replaces the bottom bar |
| Choice sheet (phone) | Modal sheet at the medium detent: title + “Required · field 7 of 27”, a radiogroup of 48 px rows (selected: `accent-tint`, `accent` 600, Check icon), a footer with Previous (secondary) and Next (primary), 44 high |
| Contents sheet | §7.16 sheet whose header is followed by the full-width sheet tabs (§7.5); on tablet it spans the width and stops at the status bar |

### 7.36 Signing (proposed)

- **Signature pad:** white, 1 px `control-border`, radius 4, 180 px high (phone: full width), a `chip-border` baseline 34 px from the bottom and a 12 px hint under it; ink from the signature palette (§2.7).
- **Signature appearance:** the signature image (or none) beside three lines of 9.5–11.5 px details (“Digitally signed by …”, time, reason).
- **Typed signature styles:** Caveat 500, Newsreader italic, Newsreader with letter spacing (§3).
- **Signature row:** see §7.13.

### 7.37 Code block and payload (proposed)

`pre` on white, 1 px `hairline` edge (`control-border` when focusable), radius 4, padding `12px 14px`, `mono` 12.5/1.6, horizontal scroll. Used for crash reports, the telemetry payload, code samples, raw XML and JavaScript shown as text.

### 7.38 Avatars and presence (proposed)

24–48 px circles with initials 11–16/600. Your own avatar: `accent-tint` + `accent`. Reviewers: their presence colour (§2.7) with white initials. Pending invitees: `surface` + `chip-border` + `text-3`. Stacks overlap by 6 px with a 2 px `ground` ring. The bell: a 28 px icon button with a count pill (§7.23).

### 7.39 Site and product pages (proposed)

- **Docs-site header:** 64 px, max 1120 px: wordmark Source Sans 3 24/700 (the docs site is sans-only, §3), the one site nav for every docs-site page (15 px links Download · Docs · Pricing · Performance · Privacy; current 600 + 2 px `accent` underline), and one secondary button, “Open the web app”. The download page uses a 56 px variant with a 22 px Source Sans 3 wordmark and the same five links and button. Footer 12 `muted` with the licence line. No app shell.
- **Large primary link:** 40 px primary button styling on an `<a>`, with the Download icon (“Download for macOS”).
- **Plan block** (in dialogs): white, `hairline` edge, radius 4, padding 16: plan name Newsreader 18, price 600 + terms `muted`, a 13 px check list. **Plan card** (docs site): `surface`, padding 28, name 26, price 40 Source Sans 3 700.

### 7.40 Media poster (proposed)

A poster block with a centred 64 px white play button (named with the title and length), a dark duration chip, and an inline info notice for content lamina can't play.

## 8. Patterns

### 8.1 Modes and their secondary toolbars

One secondary toolbar at a time, under the top bar, never two: a tool toolbar (§7.24) or the find bar (§7.30) replaces the mode's own toolbar while it is open, and Done or Close brings the mode's toolbar back. Tools are toggles (§7.24) with a 16 px icon and a label; colour swatches follow a divider; a context note sits at the right end; `…` holds what doesn't fit. Below 1024 px the toolbar wraps onto two rows; on the phone the touch bars of §7.35 replace it.

| Mode or tool | Secondary toolbar |
| --- | --- |
| Read | None (page + top bar only); the find bar while find is open |
| Comment **(proposed, from C01)** | Select │ Highlight · Underline · Strike · Caret · Note · Text box ▾ · Ink · Shapes ▾ · Stamp ▾ · Attach file │ Colour · Width │ … · “n comments on this page” · Comments. Colour and width show only for tools that use them; the colour set swaps with the tool (§2.7). Fitting rules **(proposed, Comment)**: Underline, Strike and Caret are icon-only; Text box ▾ is a split tool button and Shapes ▾ a menu tool button (§7.24); the page-count note hides while Colour and Width show and the count moves into the Comments button's name. The row is then about 1,350 px wide, so Comment wraps below about 1,366 px rather than 1,024 px |
| Fill & Sign **(proposed, from P01)** | Previous field · Next field · Highlight fields (toggle) │ Text · Check · Date │ Add a signature image (Signature image icon) · Sign with a certificate (Certificate icon) · Form data ▾ · … · “4 of 9 required fields filled” |
| Organise **(proposed, from O06 + O09–O11)** | Insert ▾ · Rotate left · Rotate right · Extract · Delete │ Move to page… │ Page setup ▾ (Crop, Resize…, Page boxes…, Page labels…, Make landscape, Make portrait) · Bookmark ▾ (Add bookmark, Create a contents page…, Named destinations…) · More page actions (⋯) · context note (“2 pages selected”). The canvas becomes the page grid; the rail shows Outline · History (no Pages tab), plus Attachments when the file has attachments (§8.10) |
| Edit **(proposed, from A01)** | Select · Edit text · Add text · Image · Shape ▾ · Link │ Page marks ▾ · Replace… │ Redact · Protect · Tools ▾ (Form design, Actions, Accessibility, Print production) · context note with the active tool's hint |
| Redact (tool of Edit) | “Redact” title · Mark text · Mark area · Find sensitive… │ Preview result (toggle) · state text (“11 marks · nothing removed yet”; after applying “10 redactions applied · undo with ⌘Z until you save”) · Apply redactions… (secondary, hidden after applying) · Done. Apply covers accepted marks only **(proposed, Sign/protect/redact)**: the confirm repeats the count of accepted marks on its button and lists marks still to review and rejected ones as “left as they are” |
| Protect (tool of Edit) | “Protect” title · its settings · Done |
| Form design (tool of Edit) | “Form design” title · Select · Text field · Check box · Radio group · Dropdown · List box · Button · Signature · Date │ Tab order · Detect fields · … · Snap to grid · field count · Done. While Tab order or Detect fields is on, the field tools give way to a `muted` “Field tools are paused…” note |
| Accessibility, Print production, Actions (tools of Edit) | Title · the tool's commands · context note · Done |
| Measure (tool of Comment) | “Measure” title · Distance · Perimeter · Area │ Scale · Calibrate… · Units · Done; opened from the palette and the “This page has a scale · Measure” note |

### 8.2 Selection → inspector

Selecting an annotation, field, page or object opens the docked inspector (it pushes the canvas, never covers it) and outlines the object on the page with the 2 px `accent` focus outline. Deselecting (Escape, click empty page) closes it. Text selection does **not** open the inspector; it shows the mini toolbar (desktop) or the action bar (touch). Creating an annotation selects it. After placing an object, the tool returns to Select with the new object selected.

**Floating layers (proposed list):** only short-lived layers float over the page: the selection mini toolbar, tooltips and link previews, menus and popovers, the palette, dialogs, toasts. Everything else docks.

### 8.3 Focus mode

Hides the rail and the secondary toolbar (top and status bars stay); a selection brings the inspector back until deselected. Toggle: palette command "Focus mode" and `⌘\` / `Ctrl \` **(proposed)**; leaving it is the same shortcut or Escape with nothing selected.

### 8.4 Explicit save

No auto-save. The top bar shows the state ("No unsaved changes", "1 unsaved change", "Saving…", "Saved"); Save (primary) appears only with changes. Saving is incremental by default. Before a save changes anything about signatures, say so in the status-bar note or the banner (§7.13). Wording:

- "Signed file. Saving adds your changes after the signature; the signature stays valid." (from the files)
- "Saving will invalidate the signature. …" (banner, proposed)
- Closing with changes: dialog "Save changes to Q3-board-report.pdf?" with "Save", "Don't save", "Cancel".
- **Save a copy** (`file.save-copy`, proposed): “Download a copy” on the web, “Save a copy…” on the desktop. It writes the document as it is now: in the reader v0.1 (no edits) that is the original bytes; in an edit session it includes the unsaved changes, and under a certification conflict it still needs the remove-signature confirm (ADR 0007). It has no shortcut: `Mod S` is Save, in edit sessions only. The web fallback for Save without File System Access is Download (`file.download`), which then rebases onto the downloaded bytes.

**File changed on disk (proposed, desktop):** with no unsaved changes, reload silently keeping page and zoom, then a toast “Reloaded: Q3-board-report.pdf changed on disk”. With unsaved changes, a `warning-tint` strip (§7.29) offers Compare · Keep mine · Reload from disk when the change is detected; the next Save opens the alertdialog “This file changed on disk” with Save as… (primary) · Reload · Overwrite (danger ghost). Keep mine dismisses the strip and keeps editing; the next Save still opens that dialog. **Overwrite** writes the whole file: the bytes kept at open plus the session as an incremental revision, rebuilt into a temporary file that is then renamed over the changed one (appending to the changed file is impossible). It is not a `FullWriter` rewrite, so signatures in the opened file stay valid (ADR 0007) **(proposed, Edit session)**.

**Save blocked by a report (proposed, Edit session O02, O03):** while the redaction report blocks saving, the Save options menu and the close dialog show the block reason and “Show report” (§8.5).

**Undo:** every action is one op and one undo step, including a replace-all or a page mark on every page. Undo and Redo name the step.

### 8.5 Trust surfaces

- **Network indicator:** always in the status bar (desktop/web) and in More on the phone; opens a panel listing each origin and what the count proves.
- **Signature badge:** a badge (§7.23) in the status bar and the signature's inspector: "Signature valid", "Signed, changed after signing" (warning), "Signature not trusted" (danger) **(proposed wording)**.
- **Redaction report:** after Apply, a panel listing each check with positive/danger badges; Save is disabled with a reason ("Saving is blocked: 1 leak found") while any check fails.
- **Repaired file:** status-bar notice "Repaired on open · Details"; details in a panel, never a modal.
- **JavaScript (proposed standard wording):** wherever a file's scripts are mentioned (repair details “Not run”, document properties, the status-bar note, the actions list), say “This file contains JavaScript. lamina never runs it.” Forms add what lamina handled natively (“lamina never runs JavaScript; it handled the 3 scripts it recognises natively and skipped 1.”). This assumes the outcome of ADR 0021 (PDF JavaScript policy, [E8a](../../todos/ready/E8a-adr-0021-javascript-policy.md)), which is **still pending**; change the wording if that ADR decides otherwise.
- **Marked is not removed:** any state that keeps original content in the file says so in words where the user acts (toolbar state text, inspector note, status bar); the word “redacted” appears only after Apply and a passing report. The history note (§6.1) says the same for deleted or replaced content.
- **Blocked save:** when a report finds a problem the user must fix first, Save stays visible as the blocked primary (§7.0), `aria-disabled="true"` with `aria-describedby` on the `danger` strip's text; the strip carries the fix button; Save re-enables as soon as a re-run passes.
- **Uploads ask first:** nothing leaves the device without the server consent block (§7.18); afterwards the network indicator names the host and counts the approved uploads, and a panel lists each upload with a delete control.

### 8.6 Page appearance vs UI theme

Three separate settings in View settings: **Theme** (Light, Dark, High contrast, Match system) recolours the chrome; **Accent** (Teal, Blue, Plum, Green, [§2.8](#28-accent-colours)) recolours the accent-derived tokens of the chrome and the text selection on the page; **Page appearance** (Original, Sepia, Dark, High contrast) recolours page content in the renderer. Original is the default in every theme: draw white pages on the dark gutter in dark mockups. Photos and annotation colours never change.

### 8.7 Touch rules

- No floating toolbar on touch; the bottom action bar replaces the bottom bar while text is selected, and the OS selection menu is left alone.
- Targets ≥ 44×44; spacing ≥ 4 between targets.
- Panels become bottom sheets with detents; the inspector is a sheet at medium detent on the phone.
- Every drag has a tap alternative (grabber button, "Move to page…").
- The viewer owns pan, pinch and double-tap; long-press selects text.

### 8.8 Keyboard and shortcuts

- Every command lives in the registry with one shortcut per platform (`Mod` = ⌘/Ctrl); chips and help are generated from it.
- **Collisions to avoid** (browser, OS and assistive tech): `Mod G` / `Mod Shift G` (find next/previous), `Ctrl Shift E` (Firefox network monitor), `Mod W/T/N/Q/L/D/H/J` in the web app (`Mod P` only for lamina’s own Print), `Mod Tab`, `F5`, `F6`, `F11`, `F12`, `Ctrl Shift I/J/C`, `⌘H`, `⌘M`, `⌘Space`, `⌘.`. **On Windows never use `Ctrl Alt` + letter**: it is AltGr on many keyboard layouts (`Ctrl Alt E` types €). So the file’s `⌥⌘E` (Export highlights) must not become `Ctrl Alt E`; low-frequency commands like it may have no Windows/Linux shortcut at all (palette and menu only) unless the collision test finds a free one.
- **Single-key shortcuts** (`H` highlight, `⌫` remove highlight) work only while the page has focus, never in inputs, and Settings has "Single-key shortcuts" (on by default) to turn them off (WCAG 2.1.4).
- Escape closes the innermost layer (mini toolbar → palette/dialog → selection → focus mode). Shift+F10 opens the mini toolbar or context menu. F6 **(proposed: desktop app only)** cycles regions: top bar → rail → canvas → inspector → status bar.
- The shortcut help dialog lists the active platform's shortcuts only.

- **Scope of single keys (proposed):** a single key belongs to one mode and one focus region, so the same letter can mean different things in different modes but never two things at once. Tool letters work in the mode whose toolbar shows the tool; `R` is Reader mode in Read mode, Rectangle in Comment mode and Rotate right in the Organise grid. Single keys never switch mode.
- **Desktop-only keys:** the desktop app may bind `⌘W`/`Ctrl W` (close tab), `⇧⌘W` (close window), `⌘Q`/`Ctrl Q` (quit), `⇧⌘N`/`Ctrl Shift N` (new window), `⇧⌘S`/`Ctrl Shift S` (Save as), `⌥⌘F`/`Ctrl H` (Find and replace), `⌘[`/`⌘]` (Back, Forward) and `F1` (Keyboard shortcuts), because no browser sits around it. The web app leaves them to the browser (on the web, Back and Forward are the browser's own; link jumps are pushed to history). The shortcut help shows them only in the desktop app. lamina never binds `⌘N`, `⌘T`, `⌘L` or `Mod G`.

**Shortcut table (proposed).** One row per command; macOS / Windows and Linux. “Page” means the page has focus (single-key rules apply). A command not listed has no shortcut and is in the palette and menus.

| Command | Id | macOS | Windows / Linux | Scope |
| --- | --- | --- | --- | --- |
| Command palette | `palette.open` | ⌘K | Ctrl K | Everywhere |
| Keyboard shortcuts | `help.shortcuts` | ? (page); F1 (desktop app) | ? (page); F1 (desktop app) | Page; F1 desktop app only **(proposed, Desktop shell)**: a native menu accelerator can't be a page-scoped single key |
| Open a file | `file.open` | ⌘O | Ctrl O | Everywhere |
| Save | `file.save` | ⌘S | Ctrl S | Edit sessions only |
| Save as… | `file.save-as` | ⇧⌘S | Ctrl Shift S | Desktop app only |
| Print | `file.print` | ⌘P | Ctrl P | Everywhere (lamina's own print) |
| Settings | `app.settings` | ⌘, | Ctrl , | Desktop app |
| New window | `window.new` | ⇧⌘N | Ctrl Shift N | Desktop app |
| Close tab / Close window | `tab.close` / `window.close` | ⌘W / ⇧⌘W | Ctrl W / Ctrl Shift W | Desktop app |
| Quit | `app.quit` | ⌘Q | Ctrl Q | Desktop app |
| Next / previous tab | `tab.next` / `tab.previous` | ⌃Tab, ⇧⌘] / ⌃⇧Tab, ⇧⌘[ | Ctrl Tab / Ctrl Shift Tab | Desktop app |
| Undo / Redo | `edit.undo` / `edit.redo` | ⌘Z / ⇧⌘Z | Ctrl Z / Ctrl Y (also Ctrl Shift Z) | Edit sessions |
| Cut, copy, paste | `edit.cut`, `edit.copy`, `edit.paste` | ⌘X, ⌘C, ⌘V | Ctrl X, Ctrl C, Ctrl V | Everywhere (copy) / editor |
| Close the open layer | `layer.close` | Esc | Esc | Everywhere |
| Selection actions | `selection.actions` | ⇧F10 | Shift F10 | Selection |
| Focus mode | `view.focus-mode` | ⌘\ | Ctrl \ | Everywhere |
| Full screen | `view.full-screen` | ⌃⌘F | — | macOS desktop app |
| Zoom in / out / actual size | `view.zoom-in` / `view.zoom-out` / `view.actual-size` | ⌘+ / ⌘− / ⌘0 (desktop); + / − / 0 (page) | Ctrl + / Ctrl − / Ctrl 0 (desktop); + / − / 0 (page) | Web: single keys only, Ctrl ± stays the browser's |
| Next / previous page | `nav.next-page` / `nav.previous-page` | Page Down / Page Up | same | Page |
| First / last page | `nav.first-page` / `nav.last-page` | Home / End | same | Page |
| Back / Forward | `nav.back` / `nav.forward` | ⌘[ / ⌘] | Alt ← / Alt → | Desktop app; the browser's own on the web |
| Reader mode | `view.reader-mode` | R | R | Page, Read mode |
| Find | `find.open` | ⌘F | Ctrl F | Everywhere |
| Next / previous match | `find.next` / `find.previous` | ↵, F3 / ⇧↵, ⇧F3 | Enter, F3 / Shift Enter, Shift F3 | Find field |
| Close find | `find.close` | Esc | Esc | Find bar |
| Find and replace | `find.open-replace` | ⌥⌘F | Ctrl H | Desktop app only |
| Next / previous field | `form.focus-next-field` / `form.focus-previous-field` | Tab / ⇧Tab | Tab / Shift Tab | Fill & Sign |
| Next region / previous region | `view.focus-next-region` | F6 / ⇧F6 (desktop); ⌃F6 / ⌃⇧F6 (web) | F6 / Shift F6 (desktop); Ctrl F6 / Ctrl Shift F6 (web) | Everywhere; the web keys need the collision test |
| Select (Comment, Edit) | `comment.select`, `tool.select` | V | V | Page, that mode |
| Highlight | `comment.highlight` | H | H | Page, Comment (and from the selection) |
| Underline · Note · Text box · Ink | `comment.underline` · `comment.note` · `comment.text-box` · `comment.ink` | U · N · T · P | same | Page, Comment |
| Rectangle · Arrow | `comment.shape` (`kind`) | R · A | same | Page, Comment |
| Delete selected annotation | `annot.delete` | ⌫ | Backspace, Delete | Page, Comment |
| Bold / italic while editing text | `annot.toggle-bold`, `edit.toggle-bold` / `annot.toggle-italic`, `edit.toggle-italic` | ⌘B / ⌘I | Ctrl B / Ctrl I | Text editing |
| Export highlights as Markdown | `comments.export-markdown` | ⌥⌘E | — (never Ctrl Alt E) | Everywhere |
| Edit text · Add text | `tool.edit-text` · `tool.add-text` | E · T | same | Page, Edit |
| Nudge 1 pt / 10 pt | `edit.transform-object` | arrows / ⇧ arrows | arrows / Shift arrows | Selected object |
| Delete selection | `edit.delete` | ⌫ | Delete | Selected object |
| Edit points | `edit.edit-points` | ↵ | Enter | Selected path |
| Rotate right / left | `pages.rotate-right` / `pages.rotate-left` | R / ⇧R | R / Shift R | Organise grid |
| Delete pages | `pages.delete` | ⌫ | Delete | Organise grid |
| Select all pages | `pages.select-all` | ⌘A | Ctrl A | Organise grid |
| Move pages | `pages.move` | ⇧⌘ arrows | Ctrl Shift arrows | Organise grid |
| Move a bookmark / change its level | — (no id yet) | ⇧⌘↑ ⇧⌘↓ / ⇧⌘← ⇧⌘→ | Ctrl Shift arrows | Outline tree in Organise |
| Move a tab stop / a tag | `form-design.move-tab-stop-up` `…-down` / `tags.move-earlier` `…-later` | ⌥↑ / ⌥↓ | Alt ↑ / Alt ↓ | That list |
| Next / previous change | `compare.next-change` / `compare.previous-change` | ⌥⌘↓ / ⌥⌘↑ | — (never Ctrl Alt) | Compare view |
| Don't save (close dialog) | — | ⌘⌫ | Alt N | Close dialog |

**Command ids (proposed rule).** Every command in the registry has one id, used by the palette, menus, shortcuts, telemetry counts and the docs.

- Form: `namespace.verb-object`, lowercase, kebab-case, exactly one dot (`file.save-copy`, `pages.rotate-left`). Variants are arguments, not more segments: `view.theme` (`light`, `dark`, `high-contrast`, `system`), `view.accent` (`teal`, `blue`, `plum`, `green`), `view.page-appearance`, `view.page-layout`, `comment.shape` (`kind`), `tool.shape` (`kind`), `comment.set-colour` (colour name).
- The namespace names the thing acted on or the surface, never a mode: `app`, `window`, `tab`, `session`, `crash`, `help`, `palette`, `layer`, `selection`, `privacy`, `settings`, `account`, `licence`, `mode`, `file`, `view`, `nav`, `find`, `edit`, `tool`, `pages`, `document`, `xmp`, `organise` (only bookmarks, destinations, attachments and the initial view), `comment` (Comment tools and threads), `comments` (the Comments panel), `annot` (a selected annotation), `attachment`, `stamps`, `form`, `form-design`, `sign`, `signatures`, `security`, `redact`, `a11y`, `tags`, `actions`, `layers`, `measure`, `print`, `ocr`, `convert`, `compare`, `revisions`, `batch`, `jobs` (server jobs).
- Page operations are `pages.*` in every mode. Tool toggles are `tool.*` in Edit and `comment.*` in Comment. A command that opens a tool toolbar is `<namespace>.open` (`redact.open`, `form-design.open`, `actions.open`, `print.open`) unless the tool's job is its own command (`a11y.check`, `security.protect`).
- One id per action across areas; labels may differ by platform or state (`file.save-copy` is “Download a copy” on the web and “Save a copy…” on the desktop).
- Preference keys in Settings (`open.target`, `watch.reload`…) are not command ids; a palette command that changes a setting is `settings.<name>`.
- The full list is in the [Command ids appendix](#appendix-command-ids).

### 8.9 Accessibility checklist (WCAG 2.2 AA)

- [ ] Text ≥ 4.5:1 (including 12 px `muted` labels) on the ground it sits on; check `muted` on `gutter` (4.74) and on `pressed` (use `text-3`)
- [ ] Control boundaries, focus rings, selected states ≥ 3:1; never `hairline`/`chip-border` as the only boundary
- [ ] Visible `:focus-visible` ring on every focusable element; one ring drawn on every board; focus never hidden under sticky bars or sheets (2.4.11)
- [ ] Targets ≥ 24×24 desktop, ≥ 44×44 touch (2.5.8)
- [ ] Real `<button>`, `<input>`, `<label>`, `<nav>`, `<aside aria-label>`; `aria-label` on icon-only buttons; `aria-hidden` on SVGs
- [ ] Modes: `role="group"` + `aria-pressed`; rail: `role="tablist"`; palette: combobox + listbox with `aria-activedescendant`; dialogs: `aria-modal`, focus trap, focus return
- [ ] Current page `aria-current="page"`, current outline row `aria-current="location"`; page pill and toasts `aria-live="polite"`
- [ ] Single-pointer alternative to every drag (2.5.7); single-key shortcuts can be turned off (2.1.4)
- [ ] State never shown by colour alone (swatches have names, badges have words or icons)
- [ ] `prefers-reduced-motion`: no sliding sheets or shimmer, instant cross-fades
- [ ] Zoom to 200% and 320 CSS px width reflows the chrome (top bar wraps; phone shell below 600)
- [ ] `forced-colors` mode keeps borders and focus rings visible (use real borders/outlines, not only shadows)

### 8.10 Rail tab set (proposed)

- The rail starts with **Pages · Outline**, then contextual tabs that exist only when the file or the moment needs them: Attachments (the file has attachments; always in Edit), Results (find is open), History (an edit session exists), Fields (the file has form fields), Signatures (the file has signatures), Review and Find sensitive (Redact), Threads (the file has article threads). “Outline” is the PDF's bookmarks; there is no separate user Bookmarks tab.
- **Width:** 220 px for up to three tabs; 280 px with four or more, showing three tabs and the More tabs button (§7.5).
- **Third slot:** the tab of what you are doing now takes the third visible place (Results while find is open, Fields in Fill & Sign and Form design, Review in Redact, Find sensitive while that search is open); the rest go into More tabs. The selected tab is always visible.
- **Organise** shows Outline · History and no Pages tab (the grid is the pages); Attachments joins them as a contextual tab when the file has attachments **(proposed, Pages and navigation)**.
- A task that needs the whole rail uses a rail panel (§7.34) instead of a tab.

### 8.11 Read mode is read-only (proposed)

Nothing in Read mode changes the file, in any area. Panels opened in Read show facts and offer the mode that edits them (“Edit in Organise” for bookmarks and attachments, “Edit in Edit” for links, “Fill in this form in Fill & Sign” when a field is clicked). Reading threads and saving attachments out are allowed; replying, filling, drawing and deleting switch mode first (the selection mini toolbar's Highlight and Comment are the exception: they create the annotation and switch to Comment).

### 8.12 Compare (proposed)

One compare view, from X04: side by side or overlay, version selects, Previous / Next change, “Change n of N”, diff marks (§2.7), read-only on both files, Close compare. Other entry points open it with a preset: “See what changed after signing” (G05) opens it on the signed revision and the current file with the signature's scope noted; “Compare with disk” opens the on-disk version; the cloud conflict opens both copies.

## 9. Writing style

- **Sentence case** for everything except the product's mode names, which are proper nouns ("Fill & Sign", "Organise").
- **British spelling** (colour, organise), curly quotes and apostrophes (’ “ ”), the middle dot `·` to join short facts, `…` for commands that ask for more input ("Go to page…", "Move to page…").
- **Plain words, the user's object first:** "Delete highlight", not "Remove annotation object"; say what happens, not what the system does.
- **Numbers:** tabular, thousands with commas ("1,248"), percentages without a space ("110%"), dates as "28 Oct 2026, 16:04".
- **States say the fact:** "No unsaved changes", "1 unsaved change", "1 comment on this page", "24 pages".
- **Trust copy states proof, not reassurance:** "0 network requests for this document · Offline-safe: nothing uploaded"; "Signed file. Saving adds your changes after the signature; the signature stays valid."; "Repaired on open · Details".
- **Hints are short imperatives:** "↑ ↓ to move", "↵ to run", "Type “go to 12” or “zoom 150” to pass a value".
- **Labels for icon-only buttons** are verb phrases: "Find in document", "Close inspector", "Back to files", "Resize sheet", "More selection actions".
- No exclamation marks, no "Oops", no blame ("This file has a damaged cross-reference table; lamina rebuilt it", not "Invalid file").
- Pricing, when shown: placeholders `[PLAN NAME]`, `[PRICE]`; never a lock icon on a feature the user is already using.

**Sample document convention.** Every mockup shows **Northwind Housing Trust — Q3 Board Report** (`Q3-board-report.pdf`, 24 pages, fictional). Page 3: running header "Northwind Housing Trust · Q3 Board Report", heading "2. Repairs backlog", the three paragraphs from the files, the chart "Open repairs by region, end of September" (North 412, East 288, South 351, West 197), footer "Page 3 of 24". The selected or highlighted sentence is "The backlog fell by 18% over the quarter, but the oldest 10% of jobs are now older than 90 days."; the highlight's author is "R. Osei", note "Ask for the 90-day list by region.", created "28 Oct 2026, 16:04". Outline: 1. Chair’s summary (1) · 2. Repairs backlog (3) › Open repairs by region (3) · 3. Rent collection (7) · 4. Surveyor capacity (11) · 5. Risk register (16) · Appendix A. Data notes (22). Invent new facts only inside this document's world, never statistics about real products; no lorem ipsum.

## 10. Authoring notes for the Design canvas

- **Reuse the snippets** in §7 and the icons in §5 verbatim; change copy, not structure. If a board needs a new component, build it from these tokens and add it here.
- Each artboard is one `.dc.html` file under `project/`, listed in `canvas.json` (`boards`, `order`, optional `notes`). Keep the file skeleton: `<script src="./support.js"></script>` in `<head>`, markup inside `<x-dc>`, fonts and base CSS in `<helmet>` (the §3 link line, plus `body{margin:0}`, `a{color:#1F5C63}a:hover{color:#16454B}`, `button,input,textarea{font:inherit;color:inherit}`), and logic in `<script type="text/x-dc" data-dc-script data-props='…'>` with a `Component extends DCLogic` whose `renderVals()` returns the template values. Loops and conditions use `<sc-for list="{{…}}" as="t" hint-placeholder-count="n">` and `<sc-if value="{{…}}" hint-placeholder-val="{{ true }}">`.
- **Artboard sizes:**
  - Web and desktop: a PAGE board, `"w": 1440`, `"h": 900`, `"expand": "fill"` in `canvas.json`, `$preview` `{"width":1440,"height":900}`; an app shell's root is `height: 100vh; display: flex; flex-direction: column` with non-wrapping rows (a `flex-wrap: wrap` row grows to its tallest child under a fixed height); site and document-length boards keep `min-height: 100vh`.
  - Phone: fixed `390 × 844`, root `width: 390px; height: 844px; position: relative; overflow: hidden`.
  - Tablet (if used): fixed `1024 × 1366`.
- **Never use % heights**; use flex, `min-height: 100vh` on page roots, or fixed px on fixed artboards. Widths may use % inside a sized parent (chart bars).
- **Keep the shell inside the board (second review, 2026-10-29).** An app shell (top bar, canvas, status bar) has `height: 100vh` on its root, not `min-height`, and every flex row or column between the root and a scrolling region has `min-height: 0` (or `min-width: 0`), so long lists, threads, dialogs and pages scroll inside their region and the status bar stays at the bottom of the 900 px board. Dialog overlays scroll their body, never the footer. Only site and document-length boards (H08, H09, S09) may grow, and their `canvas.json` height says so.
- **Text that must not wrap:** page counts (`/ 24`), shortcut chips, badges, page labels in rows and menu items, and button labels get `white-space: nowrap; flex-shrink: 0`; long names (file names, titles) get `min-width: 0` and an ellipsis instead. Check boards with the real fonts loaded, not fallbacks.
- Tweaks: at most 2–3 `data-props` per artboard: `accent` with the options `#1F5C63`, `#2D4A6B`, `#5B3B6B`, `#495533` (Teal, Blue, Plum, Green, §2.8), plus one boolean such as `showRail`, `showPalette` or an enum such as `pageAppearance` or `theme`. Copy stays literal markup.
- **Every accent-derived colour follows the tweak:** `accent`, `accent-tint` (`mix(accent, surface, .12)`), `selection-tint` (`mix(accent, '#FFFFFF', .22)`), focus rings, pressed toggles, target fields, badges and links (give each `<a>` an inline `color: {{accent}}`; the `<helmet>` rule is only a fallback). Never hard-code `#1F5C63`, `#16454B` or `#E1E6E3` for UI. Teal drawn as page content (the Edit “Teal” swatch, a logo, a chart recoloured by the user) stays `#1F5C63` and is not accent.
- Draw the macOS window frame for desktop boards (§6.1); no browser chrome for web boards; no fake device status bar on phones (the home indicator is allowed).
- Show one keyboard focus ring per board; use real elements and ARIA as in the snippets.
- No gradients, no left-border accent cards, no emoji, no shadows on docked panels.

## Decisions on area proposals

On 2026-10-29 the “New components” and “Proposed design-language additions” sections of the eleven area documents were merged into this file. Where areas proposed different things, one was chosen here. “Text updated” means the area document's prose now uses the choice; “Board follow-up” means a `.dc.html` board (or a design change beyond a rename) still shows the other form.

| # | Proposal | Areas | Choice | Why | Still using the other form |
| --- | --- | --- | --- | --- | --- |
| 1 | Rail tab set and overflow | Reader A, Forms, Pages and navigation, Add and edit, Sign/protect/redact, Reader B | Reader A's rule (§8.10): `Pages · Outline` + contextual tabs; 220 px up to three tabs, 280 px with four or more; a 28 px More (⋯) icon button outside the tablist; the current task's tab takes the third slot; no user Bookmarks tab | One rule for every board; the selected tab never hides | Resolved on the boards in the second review (2026-10-29) |
| 2 | Organise rail and toolbar | Edit session (O06), Pages and navigation | O06's toolbar order plus Pages and navigation's Page setup ▾ and Bookmark ▾ (§8.1); the rail stays with Outline · History and no Pages tab | Without a rail, History and bookmark editing are unreachable in Organise | Text updated (O06). O06 resolved in the second review; Pages and navigation boards (Delete before Extract) not rechecked |
| 3 | Where Redact and Protect live | Sign/protect/redact, Add and edit | Visible on the Edit toolbar: `… │ Redact · Protect · Tools ▾`; Tools ▾ keeps Form design, Actions, Accessibility, Print production | Principle 3: every action has a visible home; both are frequent and consequential | Resolved on the boards in the second review (2026-10-29) |
| 4 | Fill & Sign toolbar | Forms, Sign/protect/redact, design language | Forms' P01 version with Highlight fields and Form data ▾, signing actions with the Signature image and Certificate icons; ids `sign.add-signature-image`, `sign.with-certificate` (no area used another signing id) | Forms owns the mode and drew it in full | Resolved on the boards in the second review (2026-10-29) |
| 5 | Find match colours | Reader A, Reader B, Sign/protect/redact | Reader A's `find-match` / `find-current` tokens per page appearance (§2.6), also for sensitive-content hits | Computed for every appearance; distinct from the selection tint and from yellow highlights | Resolved on the boards in the second review (2026-10-29) |
| 6 | Phone find | Reader A (R05), Reader B (R18) | Reader A's: search field with Done in the top bar, Previous · Results · Next bottom toolbar (§7.30), for every area | One phone find everywhere | Resolved on the boards in the second review (2026-10-29) |
| 7 | Monospace stack | Desktop shell, Convert and produce vs Forms, Pages and navigation, Account and cloud | The `mono` token with `"Liberation Mono"`, 12.5/1.6 for blocks (§3) | Linux has no `ui-monospace`; one size for every code block | Resolved on the boards in the second review (2026-10-29) |
| 8 | Drawing check boxes and radios | Desktop shell (`role="checkbox"` buttons), Forms (hidden native input), Add and edit (`role="checkbox"` tri-state), design language (`accent-color`) | Native input visually hidden inside its label with a drawn box; mixed via `indeterminate` (§7.19) | Native semantics and keyboard, consistent drawing, edge colour under our control | Resolved on the boards in the second review (2026-10-29): every area draws the §7.19 pattern |
| 9 | Selection handle size | Comment (9 px), Add and edit (8 px), Forms (7 px, 1 px border), Pages and navigation (10 px crop), Convert and produce (8 px) | 8 px squares, 1.5 px `accent` border, 24 px hit area, round 10 px line points (§7.32) | Add and edit owns object editing and specified the full set | Forms P05 resolved; Comment C02–C03 and Pages and navigation O09 not rechecked |
| 10 | Menu | Reader A, Desktop shell, Comment, Forms, Add and edit, Edit session, Sign/protect/redact | One menu (§7.26): 32 px items, shortcut chips, palette active row; a with-descriptions variant (14/600 title over 12 px) | Seven near-identical specs | Resolved on the boards in the second review (2026-10-29) |
| 11 | Dialog widths | Design language (440/560), Add and edit (760), Edit session (880), Convert and produce (820) | 440 · 560 · 820 (§7.18) | Three steps are enough | Resolved on the boards in the second review (2026-10-29); G06-remove-security also moved to 560 |
| 12 | Strips and banners | Reader B, Desktop shell, Edit session, Sign/protect/redact, Convert and produce, Account and cloud | One strip (§7.29), 36 px min, tinted by meaning | Six near-identical components | Resolved on the boards in the second review (2026-10-29) |
| 13 | Badge size | Convert and produce (18 px, 11/600) vs §7.23 (20 px, 12/600) | 20 px, 12/600 | 12 px is the UI minimum (§3) | Resolved on the boards in the second review (2026-10-29) |
| 14 | Docked panel widths | Reader B (288–320), Comment (360), Sign/protect/redact (340, 360), Convert and produce (inspector 300) | 288 inspector; 360 wide docked panel (§6.1) | Two widths | Resolved on the boards in the second review (2026-10-29) |
| 15 | Icon drawings | Several | One drawing per name (§5): Printer (Convert), File (common; not Pages and navigation's rounded one, not Desktop shell's Document), Lock rx 1.5, Paperclip (Reader A), Link (Reader A), Download (common; not Desktop shell's tray, not Forms' Import), Upload = Share, Select (Comment/Forms), Undo (Edit session), Align (Add and edit), Arrow up/down (Edit session), Eye (Convert), Delete (Edit session), Redact (Sign), Replace = Swap (Add and edit), Tools (Add and edit grid, not the wrench), Plus = Zoom in, Forward vs Chevron right kept distinct, Drag grip stroked | Same name, same drawing | Resolved in the second review for Reader A, Reader B, Desktop shell, Forms, Add and edit, Sign/protect/redact and Convert and produce; Pages and navigation, Comment and Account and cloud not rechecked. Mark area redrawn as a dashed rectangle with a corner handle (decision 37) |
| 16 | Command id form | Reader B (`view.theme.dark`, `view.zoom.actual-size`…), Comment (`annot.text.bold`, `comment.shape.rectangle`, `comment.stamp.place`…) | `namespace.verb-object` with one dot; variants are arguments (§8.8) | Ids stay stable when options change | Text updated (Reader B, Comment) |
| 17 | Page operation ids | Edit session (`pages.*`), Pages and navigation (`organise.*`) | `pages.*` in every mode, including crop, resize, boxes, labels, orientation; `pages.export-images` for page images and `file.export-pictures` for embedded pictures; `organise.*` kept only for bookmarks, destinations, attachments, initial view | Commands act on pages whatever the mode | Resolved on the boards in the second review (2026-10-29) |
| 18 | Save a copy | Reader A, Reader B (`file.download-copy`, Ctrl S), Desktop shell (`file.export-copy`), Edit session | `file.save-copy`, no shortcut; Mod S is Save, in edit sessions only; `file.download` is only Save's web fallback (§8.4) | One action, one id; Ctrl S must not mean two things | Resolved on the boards in the second review (2026-10-29) |
| 19 | Document properties | Reader A/B (`file.properties`), Desktop shell (`doc.properties`), Edit session (`document.info`) | `file.properties`, label “Document properties…”; read-only in Read, editable in the editor | One dialog, one id | Resolved on the boards in the second review (2026-10-29) |
| 20 | Other duplicate ids | Desktop shell, Add and edit, Reader A/B, Convert and produce, Forms, Edit session | `privacy.network-activity` (not `network.show`), `view.reader-mode` (not `reader.open`), `find.match-case` / `find.whole-words` / `find.open-replace` (not `find.toggle-*`, `find.replace`), `annot.delete` (not `comment.delete`), `tool.link` (not `edit.link`), `security.protect` (not `protect.open`), `form-design.open` (not `form-design.open-tool`), `actions.open` (not `edit.open-actions`, `actions.list`), `a11y.check` (not `a11y.open`), `print.open` (not `produce.open`), `file.save-optimised`, `file.save-as-pdfa`, `file.save-as-pdfua`, `file.show-constraint-details` (not `save.*`), `mode.fill-sign` (not `form.switch-to-fill-sign`) | Namespace by object; one id per action | Text updated in every area doc |
| 21 | Single key `R` | Reader B (Reader mode), Comment (Rectangle), Edit session (Rotate right) | Single keys are scoped by mode and focus (§8.8): R = Reader mode in Read only | Each use is where people expect it | Text updated (Reader B R12) |
| 22 | Read mode edits | Comment (mini toolbar highlights “without switching”; “Pen draws in Read mode” setting), others | Read mode is read-only in every area (§8.11); Highlight from the mini toolbar switches to Comment | Read promises nothing changes | Follow-up (design change): Comment C01 entry points and C09 pen setting |
| 23 | Compare view | Sign/protect/redact G05, Convert and produce X04 | X04's view is canonical; G05 opens it with a signature-revision preset (§8.12) | One compare UI | Resolved on the boards in the second review (2026-10-29) |
| 24 | File changed on disk while dirty | Desktop shell S04 (strip), Edit session O02 (dialog) | Clean: silent reload + toast. Dirty: S04's strip when detected; O02's dialog at the next Save; Overwrite rewrites the whole file through a temporary file (§8.4, decision 40) | The strip informs at once; the dialog decides at save time | Text updated (Desktop shell S04, Edit session O02) |
| 25 | JavaScript wording | Reader A, Convert and produce X11, Forms P02 | “This file contains JavaScript. lamina never runs it.” (§8.5); ADR 0021 still pending | One sentence everywhere | Resolved on the boards in the second review (2026-10-29) |
| 26 | Reader v0.1 top bar | Reader A, Reader B, Account and cloud | Reader A's (§7.1): no mode switcher or save state; History, View, Reader | v0.1 is read-only | None (editor-area Read boards keep the mode switcher) |
| 27 | Editor top bar View button | Reader B, Account and cloud vs Add and edit, Forms, Edit session | View (view settings) in every editor top bar (§7.1) | Page appearance must be reachable in every mode | Resolved on the boards in the second review (2026-10-29) |
| 28 | Content palettes | Comment (line colours), Add and edit (content colours), Pages and navigation (outline colours), Sign (ink) | Kept separate, one job each (§2.7) | They are written into the file with different meanings | None |
| 29 | Diff overlay orange ratio | Convert and produce (“6.9:1”) | `#B4540F` is 4.98:1 on white | Recomputed | None (the additions text is replaced) |
| 30 | Drop zone | Reader A (radius 6, large), Pages and navigation (radius 4, compact) | Two sizes, radius 6 (§7.21) | Same component | Board follow-up: Pages and navigation O14 |
| 31 | Measure tool entry | Convert and produce X12 (“Comment mode › Measure tool (Tools menu…)”) | Comment has no Tools menu: Measure opens from the palette, the “This page has a scale · Measure” note and the Comment toolbar's `…` (§8.1) | Keeps the Comment toolbar as drawn in C01 | Resolved on the boards in the second review (2026-10-29) |
| 32 | Second merge (final review) | Every area | The remaining “Proposed design-language additions” were merged on 2026-10-29 into §2.3, §2.5, §3, §4, §5, §6.1, §6.2, §7.0, §7.1, §7.5, §7.13, §7.16–§7.21, §7.24, §7.25, §7.32, §7.33, §7.39, §8.1, §8.4, §8.5, §8.8, §8.10, marked “(proposed)” with the area named; new command ids went into the appendix | One place for the rules | See rows 33–45 |
| 33 | Shape tool buttons | Comment (Shapes ▾ menu), Add and edit (Shape ▾ split), §7.24 (Shape ▾ as a split example) | Comment's Shapes ▾ is a menu tool button; Edit's Shape ▾ stays a split tool button (A01); §7.24 names each | Comment has no “last-used shape” toggle; Edit has | None |
| 34 | Status bar slots | Desktop shell (right slot only the page label), Sign/protect/redact and Convert and produce (badges right) | Middle: notices, progress and persistent action items (Printing · Cancel, Update ready), then one note by priority; right: page label then signature, security and claim badges (§6.1) | Badges are facts about the document, not notices | None |
| 35 | Banner “Save a copy” label | §7.13 (“Save a copy”), §8.4 (“Save a copy…”) | The `file.save-copy` labels: “Save a copy…” on the desktop, “Download a copy” on the web | One label per platform | Text and boards updated (Forms P01 web board now says “Download a copy”) |
| 36 | Network indicator wording | Reader A, Reader B, Account and cloud (“for this document”) vs others (“· Offline-safe: nothing uploaded”) | With a document: “0 network requests for this document · Offline-safe: nothing uploaded”; without one: “0 network requests · Offline-safe: nothing uploaded” (§7.8) | Both halves are proof; the count is per document | Text and boards aligned on 2026-10-29 |
| 37 | Mark area icon | Sign/protect/redact (closes decision 15's open point) | Dashed rectangle with a square corner handle (§5) | No longer reads as Focus mode | None (G08–G10 draw it) |
| 38 | 11 px labels on the page | Forms (§7.32 overlay labels) vs §3 (11 px only for chips and overlines) | Allowed as an explicit §3 exception for page-overlay labels and tags | They sit on the page at its zoom and repeat at ≥ 12 px in the inspector or a rail tab | None |
| 39 | Keyword chip remove button | Edit session (24 px) vs §7.19 (20 px) | 26 px chips with 24 px remove buttons (§7.19) | Meets the 24 px desktop target | None |
| 40 | Overwrite on a changed file | §8.4 (“a full rewrite”), Edit session O02 | Overwrite rebuilds the whole file from the bytes kept at open plus the session as an incremental revision, via a temporary file and a rename; not a `FullWriter` save (§8.4) | “Full rewrite” would read as breaking signatures (ADR 0007) | None |
| 41 | Organise rail and zoom | §8.10 (Outline · History only), §7.1 (thumbnail control replaces the page input), Pages and navigation | Organise may add the contextual Attachments tab; the thumbnail size control replaces page input and zoom and zooms the single page while Crop is on | Matches O09–O14 | None |
| 42 | Docs-site header | §7.39 (download page: three links), Account and cloud H08/H09, Desktop shell S09 | One nav for every docs-site page: Download · Docs · Pricing · Performance · Privacy + “Open the web app”, in both header sizes | One site, one nav | None |
| 43 | `help.shortcuts` on desktop | Desktop shell (F1), §8.8 (`?` only) | `?` on the page everywhere; F1 in the desktop app (§8.8) | A native menu accelerator can't be a page-scoped single key | None |
| 44 | Server consent wording | §7.18 (four fixed rows, checkbox repeats the file name), Convert and produce | Four rows worded per job; checkbox names what is sent; retention fixed before upload and never extended (§7.18) | Pages, addresses and batches are not “the file” | None |
| 45 | Linux window frame radius | Desktop shell (8), §4 (10 only for macOS) | Windows and Linux frames radius 8 (§4, §6.1) | Matches the drawn boards | None |
| 46 | Accent colours (owner, 2026-10-29) | Every area (canvas tweak Teal · Blue · Green, light only) | Teal (default), Blue, Plum and Green in light and dark, a user setting beside Theme, saved to the account (§2.8, §8.6); Green moved to moss `#495533`; the text selection on the page follows the accent | People asked to choose; every set passes AA and is derived like Teal | Boards updated: every board has the four-option `accent` tweak; R10 and S07 draw the Accent row; F01, F03 and R11 preview dark |

## Appendix: Command ids

Every command id used in the area documents on 2026-10-29 (regenerated after the second merge, decision 32), after the renames in the decisions table, sorted by id; `mode.read` and `mode.edit` come from §8.1. Shortcuts are macOS · Windows/Linux (one value means the same on both); “—” means palette and menus only. “Shared key, scoped” marks a key used by more than one command in different modes or focus regions (§8.8); no two commands share a key in the same scope. ⚠ marks a shortcut the web app must not use. Preference keys in Settings (`open.target`, `session.reopen`, `watch.reload`, `tabs.allow-duplicates`, `recent.enabled`, `print.mode`) are not commands and are left out.

| Id | Title | Area and screen | Shortcut | Note |
| --- | --- | --- | --- | --- |
| `a11y.check` | Check accessibility | Add and edit; Convert and produce X06 | — | Also opens the Accessibility tool (was `a11y.open`) |
| `account.change-plan` | Change plan | Account and cloud H02 | — |  |
| `account.create` | Create an account | Account and cloud H01 | — |  |
| `account.delete` | Delete account… | Account and cloud H02 | — |  |
| `account.export-data` | Export my data | Account and cloud H02 | — |  |
| `account.mark-notifications-read` | Mark notifications read | Account and cloud H07 | — |  |
| `account.open` | Account and plan | Account and cloud H02 | — |  |
| `account.open-notifications` | Open notifications | Account and cloud H07 | — |  |
| `account.request-code` | Request code | Account and cloud H01 | — |  |
| `account.sign-in` | Sign in | Account and cloud H01 | — |  |
| `account.sign-out` | Sign out | Account and cloud H02 | — |  |
| `account.sign-out-device` | Sign out device | Account and cloud H02 | — |  |
| `account.upgrade-plan` | Upgrade plan | Account and cloud H11 | — |  |
| `account.view-receipts` | View receipts | Account and cloud H02 | — |  |
| `actions.open` | Actions | Add and edit; Convert and produce X11 | — |  |
| `annot.delete` | Delete annotation | Reader B R12, R16; Comment C02, C05 | ⌫ (page) — shared key, scoped |  |
| `annot.done-editing` | Done editing | Comment C03 | — | Close to `comment.done` and `edit.finish-text` |
| `annot.set-alignment` | Text alignment (annotation) | Comment C03 | — |  |
| `annot.set-border-style` | Set border style | Comment C02 | — |  |
| `annot.set-colour` | Set colour | Comment C02 | — |  |
| `annot.set-fill` | Set fill | Comment C02 | — |  |
| `annot.set-font` | Font (annotation) | Comment C03 | — |  |
| `annot.set-font-size` | Font size (annotation) | Comment C03 | — |  |
| `annot.set-line-ends` | Set line ends | Comment C02 | — |  |
| `annot.set-opacity` | Set opacity | Comment C02 | — |  |
| `annot.set-width` | Set width | Comment C02 | — |  |
| `annot.toggle-bold` | Bold (annotation text) | Comment C03 | ⌘B · Ctrl B (editing) |  |
| `annot.toggle-italic` | Italic (annotation text) | Comment C03 | ⌘I · Ctrl I (editing) |  |
| `app.about` | About lamina | Desktop shell S03 | — |  |
| `app.check-for-updates` | Check for updates… | Desktop shell S03, S06 | — |  |
| `app.dismiss-offline` | Dismiss offline notice | Reader B R15 | — |  |
| `app.dismiss-update` | Dismiss update notice | Reader B R15 | — |  |
| `app.install` | Add to home screen… | Reader A R09; Reader B R15, R18 | — |  |
| `app.quit` | Quit | Desktop shell S03 | ⌘Q · Ctrl Q (desktop only) |  |
| `app.reload-update` | Reload to update | Reader B R15 | — |  |
| `app.restart-to-update` | Restart to update | Desktop shell S06 | — |  |
| `app.settings` | Settings | Desktop shell S01, S03, S07 | ⌘, · Ctrl , (desktop) | Settings window; `view.settings` is the popover |
| `app.update-later` | Update later | Desktop shell S06 | — |  |
| `attachment.replace` | Replace attached file | Comment C05 | — |  |
| `attachment.save` | Save attached file (file annotation) | Comment C05 | — | Same job as `file.save-attachment`; merge later |
| `batch.open` | Batch… | Convert and produce X05 | — |  |
| `batch.open-recipe` | Open recipe… | Convert and produce X05 | — |  |
| `batch.run` | Run batch | Convert and produce X05 | — |  |
| `batch.save-recipe` | Save recipe | Convert and produce X05 | — |  |
| `batch.stop` | Stop after current files | Convert and produce X05 | — |  |
| `comment.attach-file` | Attach file | Comment C01 | — | File annotation; `organise.attach-file` embeds a file |
| `comment.callout` | Callout | Comment C01 | — |  |
| `comment.caret` | Caret | Comment C01 | — |  |
| `comment.done` | Done (phone annotate) | Comment C08 | — | Phone; close to `annot.done-editing` |
| `comment.draw-with-finger` | Draw with finger | Comment C09 | — |  |
| `comment.eraser` | Eraser | Comment C08, C09 | — |  |
| `comment.go-to` | Go to comment | Comment C04 | — |  |
| `comment.highlight` | Highlight | Reader B R11, R12, R17; Comment C01 | H (page) | Also the selection toolbar’s Highlight |
| `comment.highlighter` | Highlighter | Comment C08, C09 | — |  |
| `comment.ink` | Ink | Reader B R17; Comment C01, C08, C09 | P (page, Comment) | `comment.pen` is its alias |
| `comment.lasso` | Lasso | Comment C09 | — |  |
| `comment.note` | Note | Reader B R17; Comment C01, C08, C09 | N (page, Comment) |  |
| `comment.pen` | Pen | Comment C08, C09 | — | Alias of `comment.ink` (one op, one palette entry; §6.2 tablet) |
| `comment.place-stamp` | Place stamp | Comment C05 | — |  |
| `comment.reply` | Reply | Reader B R16; Comment C04, C08; Account and cloud H06 | — |  |
| `comment.resolve` | Resolve | Account and cloud H06 | — |  |
| `comment.select` | Select (Comment) | Comment C01 | V (page, Comment) — shared key, scoped |  |
| `comment.set-colour` | Comment colour | Comment C01 | — |  |
| `comment.set-status` | Set review status | Comment C04, C08 | — |  |
| `comment.shape` | Shape (rectangle, ellipse, line, arrow, polygon, polyline) | Reader B R17; Comment C01, C09 | R rectangle, A arrow (page, Comment) — shared key, scoped |  |
| `comment.show-reviewers` | Show reviewers | Account and cloud H06 | — |  |
| `comment.stamp` | Stamp | Comment C01 | — |  |
| `comment.strike` | Strike | Comment C01 | — |  |
| `comment.text-box` | Text box | Comment C01, C09 | T (page, Comment) — shared key, scoped |  |
| `comment.typewriter` | Typewriter text | Comment C01 | — |  |
| `comment.underline` | Underline | Reader B R17; Comment C01 | U (page, Comment) |  |
| `comment.width` | Line width | Comment C01 | — |  |
| `comments.copy-markdown` | Copy highlights as Markdown | Comment C06 | — |  |
| `comments.delete-selected` | Delete selected comments | Comment C04 | — |  |
| `comments.export` | Export comments… | Comment C06 | — |  |
| `comments.export-markdown` | Export highlights as Markdown | Comment C06 | ⌥⌘E (macOS only) |  |
| `comments.filter` | Filter comments | Comment C04 | — |  |
| `comments.flatten` | Flatten comments… | Comment C07 | — |  |
| `comments.import` | Import comments… | Comment C06 | — |  |
| `comments.print-summary` | Print comment summary | Comment C07 | — |  |
| `comments.search` | Search comments | Comment C04 | — |  |
| `comments.sort` | Sort comments | Comment C04 | — |  |
| `comments.summary` | Comment summary… | Comment C07 | — |  |
| `comments.toggle-panel` | Comments panel | Comment C01 | — |  |
| `compare.export-report` | Export compare report | Convert and produce X04 | — |  |
| `compare.next-change` | Next change | Convert and produce X04 | ⌥⌘↓ (macOS only) |  |
| `compare.open` | Compare… | Convert and produce X03, X04; Account and cloud H04 | — |  |
| `compare.previous-change` | Previous change | Convert and produce X04 | ⌥⌘↑ (macOS only) |  |
| `convert.from-pdf` | Convert from PDF… | Convert and produce X02 | — |  |
| `convert.to-pdf` | Convert to PDF… | Convert and produce X02 | — |  |
| `crash.copy-report` | Copy report | Desktop shell S08 | — |  |
| `crash.dont-send` | Don’t send report | Desktop shell S08 | — |  |
| `crash.send-report` | Send report | Desktop shell S08 | — |  |
| `crash.view-report` | View report | Desktop shell S08 | — |  |
| `document.merge` | Merge documents… | Edit session O06, O07 | — |  |
| `document.merge-add-files` | Add files to merge | Edit session O07 | — |  |
| `document.remove-metadata` | Remove metadata | Edit session O04 | — |  |
| `document.set-metadata` | Set metadata | Edit session O04 | — |  |
| `document.split` | Split document… | Edit session O06, O08 | — |  |
| `edit.add-bates` | Add Bates numbers… | Add and edit A03 | — |  |
| `edit.add-header-footer` | Add header and footer… | Add and edit A03 | — |  |
| `edit.add-link` | Add link | Pages and navigation O12 | — |  |
| `edit.add-watermark` | Add watermark… | Add and edit A03 | — |  |
| `edit.align-text` | Align text | Add and edit A01 | — |  |
| `edit.bring-forward` | Bring forward | Add and edit A02, A05 | — |  |
| `edit.bring-to-front` | Bring to front | Add and edit A02, A05 | — |  |
| `edit.copy` | Copy | Reader B R11, R12; Desktop shell S03; Add and edit A05 | ⌘C · Ctrl C | Also copies selected text in Read |
| `edit.crop-image` | Crop image | Add and edit A02 | — |  |
| `edit.cut` | Cut | Add and edit A05 | ⌘X · Ctrl X |  |
| `edit.delete` | Delete selection | Add and edit A01, A02, A05 | ⌫ · Delete (object) — shared key, scoped |  |
| `edit.delete-link` | Delete link | Pages and navigation O12 | — |  |
| `edit.duplicate` | Duplicate | Add and edit A02, A05 | — | Objects; `pages.duplicate` is for pages |
| `edit.edit-points` | Edit points | Add and edit A05 | ↵ |  |
| `edit.finish-editing` | Finish editing (shared review) | Account and cloud H06 | — |  |
| `edit.finish-text` | Finish text | Add and edit A04 | — | Close to `annot.done-editing` |
| `edit.go-to-step` | Go to step | Edit session O01 | — |  |
| `edit.history` | Show history | Edit session O01 | — |  |
| `edit.insert-symbol` | Insert symbol | Add and edit A01 | — |  |
| `edit.invert-mask` | Invert mask | Add and edit A05 | — |  |
| `edit.make-link` | Make link | Pages and navigation O12 | — |  |
| `edit.manage-page-marks` | Manage page marks | Add and edit A03 | — |  |
| `edit.notify-when-free` | Notify when free | Account and cloud H06 | — |  |
| `edit.paste` | Paste | Add and edit | ⌘V · Ctrl V |  |
| `edit.redo` | Redo | Reader B R16; Edit session O01 | ⇧⌘Z · Ctrl Y, Ctrl Shift Z |  |
| `edit.release-clip` | Release clip | Add and edit A05 | — |  |
| `edit.release-mask` | Release mask | Add and edit A05 | — |  |
| `edit.replace-checked` | Replace checked | Add and edit A06 | — |  |
| `edit.replace-image` | Replace image | Add and edit A02, A05 | — |  |
| `edit.replace-one` | Replace one | Add and edit A06 | — |  |
| `edit.revert-object` | Revert object | Add and edit A05 | — |  |
| `edit.revert-text` | Revert text | Add and edit A04 | — |  |
| `edit.select-all` | Select all | Desktop shell S03 | — |  |
| `edit.send-backward` | Send backward | Add and edit A02, A05 | — |  |
| `edit.send-to-back` | Send to back | Add and edit A02, A05 | — |  |
| `edit.set-fill` | Set fill | Add and edit A02 | — |  |
| `edit.set-font` | Set font | Add and edit A01 | — |  |
| `edit.set-font-size` | Set font size | Add and edit A01 | — |  |
| `edit.set-line-spacing` | Set line spacing | Add and edit A01 | — |  |
| `edit.set-opacity` | Set opacity | Add and edit A02 | — |  |
| `edit.set-paragraph-font` | Set paragraph font | Add and edit A04 | — |  |
| `edit.set-stroke` | Set stroke | Add and edit A02 | — |  |
| `edit.set-structure-role` | Set structure role | Add and edit A01 | — |  |
| `edit.skip-match` | Skip match | Add and edit A06 | — |  |
| `edit.substitute-missing-glyphs` | Substitute missing glyphs | Add and edit A04 | — |  |
| `edit.toggle-bold` | Bold | Add and edit A01 | — |  |
| `edit.toggle-italic` | Italic | Add and edit A01 | — |  |
| `edit.toggle-subscript` | Toggle subscript | Add and edit A01 | — |  |
| `edit.toggle-superscript` | Toggle superscript | Add and edit A01 | — |  |
| `edit.toggle-underline` | Toggle underline | Add and edit A01 | — |  |
| `edit.transform-object` | Move, resize or rotate object | Add and edit A02 | arrows, ⇧ arrows (nudge) |  |
| `edit.undo` | Undo | Reader B R16; Edit session O01 | ⌘Z · Ctrl Z |  |
| `file.change-reviewer-role` | Change reviewer role | Account and cloud H05 | — |  |
| `file.clear-recent` | Clear recent | Reader A R01; Desktop shell S01, S03 | — |  |
| `file.compare-with-disk` | Compare with the version on disk | Desktop shell S04; Edit session O02; Convert and produce X04 | — | Opens `compare.open` with the on-disk preset (§8.12) |
| `file.copy-repair-details` | Copy repair details | Reader A R04 | — |  |
| `file.copy-review-link` | Copy review link | Account and cloud H05 | — |  |
| `file.discard-changes` | Discard changes | Edit session O03 | — |  |
| `file.download` | Download (Save on the web without file access) | Edit session O02 | — | Same action as `file.save` on the web without File System Access; not a separate menu item |
| `file.download-cloud-copy` | Download cloud copy | Account and cloud H03 | — |  |
| `file.download-optimised` | Download optimised | Edit session O02 | — |  |
| `file.export` | Export | Pages and navigation O16 | — |  |
| `file.export-links-and-bookmarks` | Export links and bookmarks | Pages and navigation O16 | — |  |
| `file.export-pictures` | Export pictures | Pages and navigation O16 | — |  |
| `file.export-text` | Export text | Pages and navigation O16 | — |  |
| `file.go-to-object` | Go to object | Pages and navigation O15 | — |  |
| `file.inspect-objects` | Inspect objects | Pages and navigation O15 | — |  |
| `file.invite-reviewer` | Invite reviewer | Account and cloud H05 | — |  |
| `file.keep-mine` | Keep mine | Desktop shell S04; Edit session O02 | — |  |
| `file.locate` | Locate | Desktop shell S04 | — |  |
| `file.open` | Open a file | Reader A R01, R09; Reader B R12; Desktop shell S01, S02, S03, S10; Account and cloud H03 | ⌘O · Ctrl O |  |
| `file.open-cloud-list` | Open cloud list | Account and cloud H03 | — |  |
| `file.open-other-version` | Open other version | Account and cloud H04 | — |  |
| `file.open-recent` | Open recent | Reader A R01; Desktop shell S01, S03 | — |  |
| `file.open-url` | Open from URL | Reader A R01 | — |  |
| `file.overwrite` | Overwrite | Edit session O02 | — |  |
| `file.print` | Print | Reader A R09; Reader B R12, R18; Desktop shell S03, S05 | ⌘P · Ctrl P |  |
| `file.properties` | Document properties | Reader A R08, R09; Reader B R18; Desktop shell S03; Edit session O04 | — |  |
| `file.reload-from-disk` | Reload from disk | Desktop shell S04; Edit session O02 | — |  |
| `file.remove-from-cloud` | Remove from cloud | Account and cloud H03 | — |  |
| `file.remove-recent` | Remove recent | Desktop shell S01 | — |  |
| `file.remove-reviewer` | Remove reviewer | Account and cloud H05 | — |  |
| `file.rename` | Rename | Account and cloud H03 | — |  |
| `file.save` | Save | Desktop shell S03; Edit session O02, O03 | ⌘S · Ctrl S (edit sessions) |  |
| `file.save-as` | Save as… | Edit session O02 | ⇧⌘S · Ctrl Shift S (desktop only) |  |
| `file.save-as-pdfa` | Save as PDF/A… | Convert and produce X08 | — |  |
| `file.save-as-pdfua` | Save as PDF/UA… | Convert and produce X08 | — |  |
| `file.save-attachment` | Save attachment (embedded file) | Reader A R06; Pages and navigation O14 | — | Same job as `attachment.save` (save an embedded file out); merge later |
| `file.save-copy` | Download a copy / Save a copy… | Reader A R09; Reader B R12, R18; Desktop shell S03; Edit session O02; Comment C01 | — | “Download a copy” on the web, “Save a copy…” on the desktop |
| `file.save-invalidating-signatures` | Save invalidating signatures | Edit session O02; Comment C01 | — |  |
| `file.save-optimised` | Save an optimised copy… | Edit session O02; Pages and navigation O14; Convert and produce X09 | — |  |
| `file.share-for-review` | Share for review | Account and cloud H03, H05 | — |  |
| `file.show-constraint-details` | Signature constraint details | Edit session O02; Comment C01 | — |  |
| `file.show-repairs` | Show repairs | Reader A R04 | — |  |
| `file.show-sync-conflict` | Show sync conflict | Account and cloud H04 | — |  |
| `file.show-upload-queue` | Show upload queue | Account and cloud H04 | — |  |
| `file.stop-syncing` | Stop syncing this document | Account and cloud H03 | — |  |
| `file.toggle-review-link` | Toggle review link | Account and cloud H05 | — |  |
| `file.unlock` | Unlock (password) | Reader A R03 | — |  |
| `file.upload-to-cloud` | Upload to cloud | Account and cloud H03 | — |  |
| `find.close` | Close find | Reader A R05; Reader B R18; Add and edit A06 | Esc |  |
| `find.match-case` | Match case | Reader A R05; Reader B R18; Add and edit A06 | — |  |
| `find.next` | Next match | Reader A R05; Reader B R12, R18; Desktop shell S03; Add and edit A06 | ↵, F3 (find field) |  |
| `find.open` | Find in document | Reader A R05; Reader B R11, R12, R17, R18; Desktop shell S03, S10 | ⌘F · Ctrl F |  |
| `find.open-replace` | Find and replace | Reader A R05; Add and edit A06 | ⌥⌘F · Ctrl H (desktop only; Ctrl H is history on web) |  |
| `find.previous` | Previous match | Reader A R05; Reader B R12, R18; Desktop shell S03; Add and edit A06 | ⇧↵, ⇧F3 (find field) |  |
| `find.whole-words` | Whole words | Reader A R05; Reader B R18; Add and edit A06 | — |  |
| `form-design.accept-page-suggestions` | Accept page suggestions | Forms P07 | — |  |
| `form-design.accept-suggestion` | Accept suggestion | Forms P07 | — |  |
| `form-design.add-button` | Add button | Forms P05 | — |  |
| `form-design.add-check-box` | Add check box | Forms P05 | — |  |
| `form-design.add-date-field` | Add date field | Forms P05 | — |  |
| `form-design.add-dropdown` | Add dropdown | Forms P05 | — |  |
| `form-design.add-list-box` | Add list box | Forms P05 | — |  |
| `form-design.add-option` | Add option | Forms P05 | — |  |
| `form-design.add-radio-group` | Add radio group | Forms P05 | — |  |
| `form-design.add-signature-field` | Add signature field | Forms P05 | — |  |
| `form-design.add-text-field` | Add text field | Forms P05 | — |  |
| `form-design.close-tool` | Close tool | Forms P05 | — |  |
| `form-design.delete-field` | Delete field | Forms P05 | — |  |
| `form-design.detect-fields` | Detect fields | Forms P07 | — |  |
| `form-design.duplicate-field` | Duplicate field | Forms P05 | — |  |
| `form-design.edit-calculation-order` | Edit calculation order | Forms P05 | — |  |
| `form-design.focus-next-unlabelled-field` | Focus next unlabelled field | Forms P06 | — |  |
| `form-design.group-as-radio-group` | Group as radio group | Forms P07 | — |  |
| `form-design.move-field` | Move field | Forms P05 | — |  |
| `form-design.move-option-up` | Move option up | Forms P05 | — |  |
| `form-design.move-tab-stop-down` | Move tab stop down | Forms P06 | ⌥↓ · Alt ↓ (list) |  |
| `form-design.move-tab-stop-up` | Move tab stop up | Forms P06 | ⌥↑ · Alt ↑ (list) |  |
| `form-design.open` | Form design | Add and edit; Forms P05 | — |  |
| `form-design.reject-suggestion` | Reject suggestion | Forms P07 | — |  |
| `form-design.remove-option` | Remove option | Forms P05 | — |  |
| `form-design.rename-field` | Rename field | Forms P05 | — |  |
| `form-design.set-field-label` | Set field label | Forms P05 | — |  |
| `form-design.set-tab-order` | Set tab order | Forms P06 | — |  |
| `form-design.toggle-snap` | Toggle snap | Forms P05 | — |  |
| `form-design.toggle-tab-order` | Toggle tab order | Forms P06 | — |  |
| `form-design.use-nearby-text` | Use nearby text | Forms P06 | — |  |
| `form.add-check` | Add check mark | Forms P01 | — |  |
| `form.add-date` | Add date | Forms P01 | — |  |
| `form.add-text` | Add text (flat form) | Forms P01 | — | Fill & Sign counterpart of `tool.add-text` (flat forms) |
| `form.export-data` | Export form data… | Forms P03 | — |  |
| `form.finish-filling` | Finish filling | Forms P04 | — |  |
| `form.flatten-fields` | Flatten fields | Forms P03 | — |  |
| `form.focus-next-field` | Next field | Forms P01, P04 | Tab |  |
| `form.focus-next-invalid-field` | Focus next invalid field | Forms P02 | — |  |
| `form.focus-next-required-field` | Focus next required field | Forms P01, P04 | — |  |
| `form.focus-previous-field` | Previous field | Forms P01, P04 | Shift Tab |  |
| `form.import-data` | Import form data… | Forms P03 | — |  |
| `form.open-data-menu` | Form data menu | Forms P01 | — |  |
| `form.open-overflow` | Open overflow | Forms P01 | — |  |
| `form.reset-fields` | Reset fields | Forms P03 | — |  |
| `form.set-value` | Set value | Forms P01 | — |  |
| `form.start-filling` | Start filling | Forms P04 | — |  |
| `form.toggle-field-highlight` | Toggle field highlight | Forms P01 | — |  |
| `help.capabilities` | What lamina can read and write | Edit session O05 | — |  |
| `help.licences` | Licences | Desktop shell S03 | — |  |
| `help.open` | Help | Desktop shell S03 | — |  |
| `help.reader-mode` | About Reader mode | Reader B R14 | — |  |
| `help.report-issue` | Report issue | Desktop shell S03 | — |  |
| `help.shortcuts` | Keyboard shortcuts | Reader A R09; Reader B R12; Desktop shell S03 | ? (page); F1 (desktop app) |  |
| `help.whats-new` | What’s new | Desktop shell S06 | — |  |
| `jobs.cancel` | Cancel and delete server job | Convert and produce X02 | — |  |
| `jobs.delete-from-server` | Delete from server now | Convert and produce X02 | — |  |
| `jobs.download-result` | Download result | Convert and produce X02 | — |  |
| `jobs.open` | Server jobs | Convert and produce X02 | — |  |
| `layer.close` | Close the open panel or dialog | Reader B R12, R13, R15 | Esc |  |
| `layers.show` | Show layers | Convert and produce X11 | — |  |
| `licence.copy-key` | Copy licence key | Account and cloud H02 | — |  |
| `licence.download-file` | Download licence file | Account and cloud H02 | — |  |
| `licence.enter` | Enter a licence key | Account and cloud H02 | — |  |
| `licence.open-file` | Open licence file… | Account and cloud H02 | — |  |
| `measure.area` | Measure area | Convert and produce X12 | — |  |
| `measure.distance` | Measure distance | Convert and produce X12 | — |  |
| `measure.perimeter` | Measure perimeter | Convert and produce X12 | — |  |
| `mode.comment` | Comment mode | ADR 0024 §2 | — |  |
| `mode.edit` | Edit mode | Design language §8.1 | — |  |
| `mode.fill-sign` | Fill & Sign mode | Forms P01 | — |  |
| `mode.organise` | Organise mode | Edit session O06 | — |  |
| `mode.read` | Read mode | Design language §8.1 | — |  |
| `nav.back` | Back | Reader A R07; Reader B R11, R12; Desktop shell S03, S10 | ⌘[ · Alt ← (desktop; browser’s own on web) |  |
| `nav.copy-link` | Copy link | Reader A R07 | — |  |
| `nav.first-page` | First page | Reader B R12; Desktop shell S03 | Home |  |
| `nav.follow-link` | Follow link | Reader A R07; Pages and navigation O12 | — |  |
| `nav.forward` | Forward | Reader A R07; Reader B R11, R12; Desktop shell S03, S10 | ⌘] · Alt → (desktop; browser’s own on web) |  |
| `nav.go-to-outline-item` | Go to outline item | Reader A R06; Reader B R17, R18 | — |  |
| `nav.go-to-page` | Go to page… | Reader A R06; Reader B R14; Desktop shell S03, S10 | — |  |
| `nav.last-page` | Last page | Reader B R12; Desktop shell S03 | End |  |
| `nav.next-page` | Next page | Reader B R12; Desktop shell S03 | Page Down |  |
| `nav.next-thread-part` | Next thread part | Pages and navigation O13 | — |  |
| `nav.open-external-link` | Open external link | Reader A R07 | — |  |
| `nav.previous-page` | Previous page | Reader B R12; Desktop shell S03 | Page Up |  |
| `nav.previous-thread-part` | Previous thread part | Pages and navigation O13 | — |  |
| `nav.view-original-page` | View original page | Reader B R14 | — |  |
| `ocr.make-searchable` | Make searchable… | Convert and produce X01 | — |  |
| `ocr.run` | Run OCR | Convert and produce X01 | — |  |
| `organise.add-bookmark` | Add bookmark | Edit session O06; Pages and navigation O11 | — |  |
| `organise.attach-file` | Attach file | Pages and navigation O14 | — | Embeds a file; `comment.attach-file` adds a file annotation |
| `organise.create-contents-page` | Create contents page | Edit session O06; Pages and navigation O11 | — |  |
| `organise.delete-attachment` | Delete attachment | Pages and navigation O14 | — |  |
| `organise.delete-bookmark` | Delete bookmark | Pages and navigation O11 | — |  |
| `organise.initial-view` | Initial view | Pages and navigation O13 | — |  |
| `organise.named-destinations` | Named destinations | Edit session O06; Pages and navigation O11 | — |  |
| `organise.set-bookmark-destination` | Set bookmark destination | Pages and navigation O11 | — |  |
| `pages.crop` | Crop pages | Edit session O06; Pages and navigation O09 | — |  |
| `pages.delete` | Delete pages | Edit session O06; Pages and navigation O17 | ⌫ · Delete (Organise grid) — shared key, scoped |  |
| `pages.duplicate` | Duplicate pages | Edit session O06; Pages and navigation O17 | — |  |
| `pages.edit-boxes` | Page boxes… | Edit session O06; Pages and navigation O09 | — |  |
| `pages.edit-labels` | Page labels… | Edit session O06; Pages and navigation O10 | — |  |
| `pages.export-images` | Export pages as images | Edit session O06; Pages and navigation O16 | — |  |
| `pages.extract` | Extract pages | Edit session O06 | — |  |
| `pages.insert-blank` | Insert blank pages… | Edit session O06 | — |  |
| `pages.insert-from-file` | Insert pages from a file… | Edit session O06, O07 | — |  |
| `pages.insert-images` | Insert pages from images… | Edit session O06, O07 | — |  |
| `pages.make-landscape` | Make landscape | Edit session O06; Pages and navigation O09 | — |  |
| `pages.make-portrait` | Make portrait | Edit session O06; Pages and navigation O09 | — |  |
| `pages.move` | Move pages | Edit session O06 | ⇧⌘ arrows · Ctrl Shift arrows (grid) |  |
| `pages.move-to` | Move to page… | Edit session O06; Pages and navigation O17 | — |  |
| `pages.replace` | Replace pages | Edit session O06 | — |  |
| `pages.resize` | Resize pages… | Edit session O06; Pages and navigation O09 | — |  |
| `pages.reverse` | Reverse page order | Edit session O06 | — |  |
| `pages.rotate-left` | Rotate left | Edit session O06 | ⇧R (Organise grid) |  |
| `pages.rotate-right` | Rotate right | Edit session O06; Pages and navigation O17; Account and cloud H10 | R (Organise grid) — shared key, scoped |  |
| `pages.select` | Select pages | Edit session O06 | — |  |
| `pages.select-all` | Select all pages | Pages and navigation O17 | ⌘A · Ctrl A (Organise grid) |  |
| `palette.open` | Command palette | Reader B R11, R12; Desktop shell S03, S10 | ⌘K · Ctrl K |  |
| `print.cancel` | Cancel printing | Reader A R09; Desktop shell S05 | — |  |
| `print.compatibility` | Try compatibility printing | Desktop shell S05 | — |  |
| `print.convert-colours` | Convert colours… | Convert and produce X10 | — |  |
| `print.impose` | Impose… | Convert and produce X10 | — |  |
| `print.open` | Print production | Add and edit | — |  |
| `print.preflight` | Preflight | Convert and produce X10 | — |  |
| `print.set-output-intent` | Set output intent… | Convert and produce X10 | — |  |
| `privacy.copy-report` | Copy network report | Reader B R13 | — |  |
| `privacy.network-activity` | Network activity | Reader A R09; Reader B R11, R13, R16, R18; Desktop shell S03, S10 | — |  |
| `privacy.show-how-to-check` | How to check this yourself | Reader B R13 | — |  |
| `redact.apply` | Apply redactions… | Sign, protect, redact G10 | — |  |
| `redact.find-sensitive` | Find sensitive content | Sign, protect, redact G09 | — |  |
| `redact.mark-selection` | Mark for redaction | Sign, protect, redact G08 | — |  |
| `redact.open` | Redact | Add and edit; Sign, protect, redact G08 | — |  |
| `revisions.back-to-current` | Back to current | Convert and produce X03 | Esc (revision open) |  |
| `revisions.open` | Show revisions | Convert and produce X03 | — |  |
| `revisions.open-revision` | Open revision | Convert and produce X03 | — |  |
| `revisions.save-revision-as` | Save this revision as… | Convert and produce X03 | — |  |
| `security.protect` | Protect document… | Add and edit; Sign, protect, redact G06 | — |  |
| `security.remove-protection` | Remove protection… | Sign, protect, redact G06 | — |  |
| `security.sanitise` | Sanitise document… | Sign, protect, redact G07 | — |  |
| `selection.actions` | Show selection actions | Reader B R12 | Shift F10 |  |
| `session.reopen-selected` | Reopen selected tabs | Desktop shell S08 | — |  |
| `session.show-missing` | Show missing file | Desktop shell S02 | — |  |
| `session.start-empty` | Start with no documents | Desktop shell S08 | — |  |
| `settings.comment-emails` | Email me about comments | Account and cloud H05 | — |  |
| `settings.crash-reports-always` | Always send crash reports | Desktop shell S08 | — |  |
| `settings.privacy` | Privacy settings | Account and cloud H10 | — |  |
| `settings.update-channel` | Update channel | Desktop shell S06 | — |  |
| `settings.update-enabled` | Automatic updates | Desktop shell S06 | — |  |
| `settings.usage-counts` | Send usage counts | Desktop shell S07; Account and cloud H10 | — |  |
| `sign.add-signature-image` | Add a signature image | Forms P01; Sign, protect, redact G01, G11 | — |  |
| `sign.certify` | Certify document | Sign, protect, redact G03 | — |  |
| `sign.open-sheet` | Sign (phone sheet) | Sign, protect, redact G11 | — |  |
| `sign.with-certificate` | Sign with a certificate | Forms P01; Sign, protect, redact G02, G11 | — |  |
| `signatures.compare` | Compare with signed version | Sign, protect, redact G05; Convert and produce X03, X04 | — | Opens `compare.open` with the signed-revision preset (§8.12) |
| `signatures.show-panel` | Show signatures | Sign, protect, redact G04 | — |  |
| `stamps.add-custom` | Add custom stamp | Comment C05 | — |  |
| `tab.activate` | Activate tab | Desktop shell S02, S10 | — |  |
| `tab.close` | Close tab | Desktop shell S02, S03, S04, S10 | ⌘W · Ctrl W (desktop only) ⚠ web collision |  |
| `tab.list` | List tabs | Desktop shell S02 | — |  |
| `tab.move-to-new-window` | Move tab to new window | Desktop shell S02, S03 | — |  |
| `tab.move-to-window` | Move tab to window | Desktop shell S02, S03 | — |  |
| `tab.next` | Next tab | Desktop shell S02, S03 | ⌃Tab, ⇧⌘] · Ctrl Tab (desktop) |  |
| `tab.previous` | Previous tab | Desktop shell S02, S03 | ⌃⇧Tab, ⇧⌘[ · Ctrl Shift Tab (desktop) |  |
| `tags.move-earlier` | Move tag earlier | Convert and produce X07 | ⌥↑ · Alt ↑ (list) |  |
| `tags.move-later` | Move tag later | Convert and produce X07 | ⌥↓ · Alt ↓ (list) |  |
| `tags.show` | Show tags | Convert and produce X07 | — |  |
| `tags.tag-selection-as` | Tag selection as… | Convert and produce X07 | — |  |
| `tool.add-text` | Add text tool | Add and edit A01 | T (page, Edit) — shared key, scoped |  |
| `tool.edit-text` | Edit text tool | Add and edit A04 | E (page, Edit) |  |
| `tool.image` | Image tool | Add and edit A02 | — |  |
| `tool.link` | Link tool | Pages and navigation O12; Add and edit | — |  |
| `tool.select` | Select (Edit) | Add and edit A05; Forms P05 | V (page, Edit) — shared key, scoped |  |
| `tool.shape` | Shape tool | Add and edit A02 | — |  |
| `view.accent` | Accent colour | Reader B R10; Desktop shell S03, S07 | — | `teal`, `blue`, `plum`, `green`; synced with the account (§2.8) |
| `view.actual-size` | Actual size | Reader B R10, R12; Desktop shell S03 | ⌘0 · Ctrl 0 (desktop); 0 (page) |  |
| `view.collapse-outline` | Collapse outline | Reader A R06; Reader B R17, R18 | — |  |
| `view.continuous` | Continuous scrolling | Reader B R10; Desktop shell S03 | — |  |
| `view.expand-outline` | Expand outline | Reader A R06; Reader B R17, R18 | — |  |
| `view.fit-page` | Fit page | Reader B R10; Desktop shell S03 | — |  |
| `view.fit-width` | Fit width | Reader B R10; Desktop shell S03 | — |  |
| `view.focus-mode` | Focus mode | Reader B R12, R16; Desktop shell S03 | ⌘\ · Ctrl \ |  |
| `view.focus-next-region` | Move to next region | Forms P01 | F6 (desktop); Ctrl F6 (web, proposed) |  |
| `view.full-screen` | Full screen | Desktop shell S03 | ⌃⌘F (macOS desktop) |  |
| `view.open-more-tabs` | More tabs | Forms P01 | — |  |
| `view.page-appearance` | Page appearance | Reader B R10, R14; Desktop shell S03 | — |  |
| `view.page-layout` | Page layout | Reader B R10; Desktop shell S03 | — |  |
| `view.reader-line-length` | Line length (Reader mode) | Reader B R14 | — |  |
| `view.reader-line-spacing` | Line spacing (Reader mode) | Reader B R14 | — |  |
| `view.reader-mode` | Reader mode | Reader A; Reader B R11, R12, R14; Desktop shell S03, S10 | R (page, Read mode) — shared key, scoped |  |
| `view.reader-reset` | Reset Reader settings | Reader B R14 | — |  |
| `view.reader-text-larger` | Larger text (Reader mode) | Reader B R14 | — |  |
| `view.reader-text-smaller` | Smaller text (Reader mode) | Reader B R14 | — |  |
| `view.rotate-display` | Rotate view | ADR 0024 appendix | — | `clockwise`, `counter-clockwise`; turns the view, never the file |
| `view.set-zoom` | Zoom to… | ADR 0024 appendix | — | A percentage, 10–800 (`zoom 150` in the palette) |
| `view.settings` | View settings | Reader A; Reader B R10, R11, R16, R17, R18 | — | Popover; `app.settings` is the Settings window |
| `view.show-attachments` | Show attachments | Reader A R06; Reader B R11, R17, R18 | — |  |
| `view.show-fields-tab` | Show fields tab | Forms P01 | — |  |
| `view.show-outline` | Show outline | Reader A R06; Reader B R11, R17, R18 | — |  |
| `view.show-pages` | Show pages | Reader A R06; Reader B R11, R17, R18 | — |  |
| `view.theme` | Theme | Reader B R10, R11; Desktop shell S03 | — |  |
| `view.thumbnail-size` | Thumbnail size | Edit session O06 | — |  |
| `view.toggle-sidebar` | Show or hide sidebar | Reader A R06; Reader B R11, R17; Desktop shell S03, S10 | — |  |
| `view.zoom-in` | Zoom in | Reader B R11, R12; Desktop shell S03, S10 | ⌘+ · Ctrl + (desktop); + (page) |  |
| `view.zoom-out` | Zoom out | Reader B R11, R12; Desktop shell S03, S10 | ⌘− · Ctrl − (desktop); − (page) |  |
| `window.close` | Close window | Desktop shell S03, S10 | ⇧⌘W · Ctrl Shift W (desktop only) |  |
| `window.new` | New window | Desktop shell S01, S03 | ⇧⌘N · Ctrl Shift N (desktop) |  |
| `xmp.remove-property` | Remove property | Edit session O04 | — |  |
| `xmp.replace-packet` | Replace packet | Edit session O04 | — |  |
| `xmp.set-property` | Set property | Edit session O04 | — |  |
