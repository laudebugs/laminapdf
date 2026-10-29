# Paper: design language

- **Status:** Draft (the owner chose the Paper direction on 2026-10-29)
- **Source of truth:** the reviewed Paper canvas (`Main.dc.html` web reader, `Desktop.dc.html` Comment mode with inspector and palette, `Mobile-read.dc.html`, `Mobile-select.dc.html`, `Style.dc.html` style tile). Every value marked *from the files* is copied from them; values marked **(proposed)** are new and are the defaults until a reviewed board replaces them.
- **Rules it serves:** the [UI/UX plan](../../plans/ui-ux.md), principles [§4](../../plans/ui-ux.md#4-principles) and layout and patterns §5.
- **Screens to design with it:** the [screen inventory](screens.md).

Designers: copy the snippets below rather than redrawing a component. Engineers: the token names in §2–§4 are the names to use in `@lamina/ui` (as CSS custom properties, `--paper-<name>`).

## 1. Personality

**Calm, editorial, book-like.** The chrome recedes like the margins of a well-set book, so the page is the only thing that speaks: warm paper grounds, hairline rules instead of boxes, a serif for titles and a quiet sans for controls, one deep teal accent used sparingly. Use it everywhere in lamina; when a screen feels busy, remove chrome rather than adding colour.

It serves principles 1 (open into reading: the page dominates, no ribbon), 2 (docked, not floating: hairline-separated panels, shadows only on short-lived layers), 7 (prove, don't claim: the network indicator is a first-class status item) and 9 (accessible by construction: every text pair ≥ 4.5:1, every control boundary ≥ 3:1). See [§4 Principles](../../plans/ui-ux.md#4-principles).

## 2. Colour

Contrast ratios are WCAG 2.x relative-luminance ratios, recomputed for this document. Thresholds: text ≥ 4.5:1; UI component boundaries, focus rings and state indicators ≥ 3:1. ⚠ marks a pair below the threshold, with the reason it is acceptable or the rule that avoids it.

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
| `accent-strong` | `#16454B` | Link hover; primary hover **(proposed for buttons)** | ground 9.63 · white on it 10.57 |
| `on-accent` | `#FFFFFF` | Text on `accent` | 7.58 |
| `accent-tint` | `#E1E6E3` | Selected rows and pressed toggles (= `accent` 12% over `surface`) | text 13.32 · accent 6.00 |
| `selection-tint` | `#CEDBDD` | Text selection on an Original page (= `accent` 22% over white) | page ink 10.91 · ⚠ vs white page 1.42 (see note 1) |
| `focus` | `#1F5C63` | Focus ring: same as `accent` | ground 6.91 · surface 7.21 · gutter 6.16 · accent-tint 6.00 |
| `danger` | `#9B2335` | Destructive actions ("Delete highlight"), errors | ground 7.12 · surface 7.44 · white 7.82 |
| `positive` | `#2F6A45` | Success, valid signature | ground 5.85 · surface 6.11 · white 6.43 |
| `warning` **(proposed)** | `#8A5300` | Warnings: approximate reflow, signature that saving would break | ground 5.77 · surface 6.02 · white 6.33 · gutter 5.14 |
| `danger-tint` / `positive-tint` / `warning-tint` **(proposed)** | `#F1E4E2` / `#E7EBE3` / `#F0E8DC` | Backgrounds of badges and notices (10% over `surface`) | own colour on tint 6.31 / 5.32 / 5.21; text ≥ 13.5 |
| `hover` **(proposed)** | `rgba(31,29,26,0.06)` | Hover fill of ghost and icon buttons (= `#EAE7E1` on ground) | text 13.62 · muted 4.73 |
| `pressed` **(proposed)** | `rgba(31,29,26,0.10)` | Pressed fill (= `#E1DED9` on ground) | text 12.53 · ⚠ muted 4.35: a pressed control's label switches to `text-3` |
| `disabled` **(proposed)** | `#A39C91` text, `hairline` border | Disabled controls (exempt from contrast; also set `disabled`/`aria-disabled`) | surface 2.59 |
| `scrim` | `rgba(31,29,26,0.30)` | Dims the app behind the palette and dialogs | — |
| `pill` | `rgba(31,29,26,0.78)` | Page-indicator pill on the phone | white text on it (over gutter) 8.95 |
| `window` | `#DCD6CB` bg, `#C9C2B6` border, `#EFEBE3` title bar, `#E0D9CD` title-bar rule | Drawn macOS window frame in mockups only | title text `text-2` on title bar 10.08 |

Notes:

1. **Selection tint and highlights on white are fills, not boundaries.** `selection-tint` (1.42) and the highlight colours (1.43–1.77) are low-contrast against the page by design, so text stays readable through them (ink on them ≥ 8.77). The *selected* state never relies on the fill alone: a selected annotation gets a 2 px `focus` outline (5.29 against yellow), the mini toolbar or the action bar appears, and touch selections draw `accent` handles (7.58 on white).
2. The `Commands` button in the top bar has a `hairline` border (1.22); its visible text identifies it, so WCAG 1.4.11 does not require the border. Don't copy that border onto icon-only controls.
3. Tweak accents in the canvas (`#2D4A6B` blue, `#3E4A3C` green) also pass: 8.31 / 8.50 on ground, white on them 9.12 / 9.34. Teal `#1F5C63` is the default.

### 2.2 Highlight and annotation colours

From the files. Annotation colours carry meaning, so they never change with UI theme or page appearance (plan §5.5).

| Name | Hex | Page ink `#262421` on it | Swatch edge |
| --- | --- | --- | --- |
| Yellow (default) | `#F2D675` | 10.80 | `control-border` 1 px (3.65 on surface) |
| Green | `#B7D9A8` | 9.95 | same |
| Blue | `#A9C8E8` | 8.93 | same |
| Pink | `#EBB4C8` | 8.77 | same |

The selected swatch adds `outline: 2px solid #1F1D1A` (offset 1 px in the toolbar, 2 px in the inspector).

### 2.3 Dark UI theme (proposed)

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
| `muted` | `#A39B8E` | ground 6.13 · surface 5.63 · raised 5.07 · on hover fill 4.52 |
| `hairline` (decorative) | `#3A3732` | ⚠ ground 1.42 |
| `chip-border` (decorative) | `#4A463F` | ⚠ raised 1.49 |
| `control-border` | `#7E766A` | ground 3.76 · surface 3.45 · raised 3.11 |
| `accent` / `focus` | `#7DBDC2` | ground 7.97 · surface 7.32 · raised 6.59 |
| `accent-strong` (hover) | `#9BCFD3` | — |
| `on-accent` | `#1E1D1A` | on accent 7.97 |
| `accent-tint` | `#343C3B` (16% over surface) | text 9.19 · accent 5.35 |
| `danger` | `#EE8C98` | ground 7.10 · surface 6.52 · raised 5.87 |
| `positive` | `#86C49A` | ground 8.33 · surface 7.65 |
| `warning` | `#E0AE5A` | ground 8.32 · surface 7.64 |
| `hover` | `rgba(236,231,222,0.08)` (= `#2E2D2A` on ground) | text ≥ 11 |
| `disabled` | `#6A645A` | exempt (2.64) |
| `scrim` | `rgba(0,0,0,0.50)` | — |

Dark page shadow **(proposed)**: `0 1px 2px rgba(0,0,0,0.40), 0 8px 28px rgba(0,0,0,0.45)`. A high-contrast UI theme follows `forced-colors` system colours (plan §5.5), not a hand-made palette.

### 2.4 Page appearance

Separate from the UI theme; chosen per document or globally. From the style tile (`Aa` chips).

| Appearance | Paper | Ink | Ink on paper | Selection tint (proposed) |
| --- | --- | --- | --- | --- |
| Original | `#FFFFFF` | `#262421` | 15.48 | `#CEDBDD` (ink 10.91) |
| Sepia | `#F4ECD8` | `#3B2F20` | 11.05 | `#C5CCBE` (ink 7.91) |
| Dark | `#22211F` | `#E6E1D8` | 12.35 | `#425858` (ink 5.83) |
| High contrast | `#000000` | `#FFFFFF` (chip label weight 600) | 21.00 | system `Highlight` |

Photos are never inverted and annotation colours never change.

### 2.5 Mockup-only colours (document content)

For drawing the sample page and thumbnails; not UI tokens. Page heading `#1C1B19` (17.21 on white), page muted `#6E6A64` (5.37), page rule `#E6E3DE`, chart bars `#4E5A63` (7.08), chart labels `#3A3733` (11.84). Thumbnail lines: heading `#B9B2A7`, text `#DDD7CD`, chart `#C9CFD3`. Phone page-4 placeholders `#CFC9BF` / `#E3DED6`; phone page rules `rgba(0,0,0,0.1)` and header/footer `opacity: 0.75`.

## 3. Typography

| Family | Use |
| --- | --- |
| **Newsreader** (serif, optical sizes 6–72) | Titles: file name, inspector and sheet titles, section headings, outline rows, quoted text |
| **Source Sans 3** | Every control, label and message; fallbacks `'Segoe UI', system-ui, sans-serif` (Newsreader falls back to `Georgia, serif`) |
| **Source Serif 4** | Page mockups only (the sample document's body). Never in UI |

The link line used in every file (keep it byte-identical, inside `<helmet>`):

```html
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Newsreader:opsz,wght@6..72,400;6..72,500;6..72,600&amp;family=Source+Sans+3:wght@400;500;600&amp;family=Source+Serif+4:opsz,wght@8..60,400;8..60,600&amp;display=swap">
```

The product will self-host these fonts (no third-party requests: principle 7); the link is for mockups.

### Type scale

| Name | Family | Size / line height / weight | Usage |
| --- | --- | --- | --- |
| Display | Newsreader | 40 / 44 / 400 | Empty states, home screen heading |
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

Rules:

- **Tabular numerals** (`font-variant-numeric: tabular-nums`) on every number that changes or lines up: page input and `/ 24`, zoom %, counts ("0 network requests", "24 pages"), dates, thumbnail and outline page numbers, shortcut chips, the page pill.
- **Minimum sizes:** 12 px for UI text; 11 px only for shortcut chips and uppercase overlines; 11.5 px only for phone bar labels under a 22 px icon. 16 px for any text input on touch.
- Sentence case everywhere; uppercase only for overlines.

## 4. Space, shape, depth

**Spacing scale** (from the style tile): `4 · 8 · 12 · 16 · 24 · 32 · 48`. Layout gaps and paddings use the scale. Inside components the files also use `2, 6, 10, 14, 18, 20, 22, 28` (for example 22 px between mode buttons, 20 px inspector padding); reuse those exact values for the same components, don't invent new ones.

**Radius:** `3` shortcut chips, appearance-chip previews · `4` buttons, inputs, rows, swatches tiles · `6` floating panels (selection toolbar, palette, dialog), appearance chips, phone bar buttons · `12` bottom sheet top corners · `height / 2` pills and circles (colour swatches, avatar, page pill). `10` is only the drawn macOS window.

**Borders:** 1 px everywhere. `hairline` separates regions (top bar bottom, rail right, inspector left, status bar top, secondary toolbar bottom, toolbar dividers 1×18 px). `control-border` on anything you type into or press that has no other boundary. Selected state: 2 px `accent` underline (modes, tabs) or 1 px `accent` border + `accent-tint` fill (toggles).

**Shadows** only on the page sheet and on short-lived floating layers. Panels never have shadows.

| Element | Shadow |
| --- | --- |
| Page sheet (web, desktop) | `0 1px 2px rgba(60,48,30,0.08), 0 8px 28px rgba(60,48,30,0.10)` |
| Page sheet (phone) | `0 1px 2px rgba(60,48,30,0.08), 0 6px 20px rgba(60,48,30,0.10)` |
| Thumbnail | `0 1px 3px rgba(60,48,30,0.12)` |
| Selection mini toolbar; toast, menu, tooltip **(proposed)** | `0 4px 14px rgba(60,48,30,0.14)` |
| Bottom sheet | `0 -8px 28px rgba(60,48,30,0.12)` |
| Command palette; dialog **(proposed)** | `0 18px 48px rgba(31,29,26,0.22)` over `scrim` |

**Z-order (proposed):**

| Layer | z-index |
| --- | --- |
| Pages, then page overlays (selection, highlights, handles) | 0, 1 |
| Docked chrome (bars, rail, inspector) | 10 |
| Selection mini toolbar, page pill | 20 |
| Menus, popovers, tooltips | 30 |
| Bottom sheet | 40 |
| Scrim + palette / dialog | 50 |
| Toasts | 60 |

## 5. Iconography

- Inline SVG, `viewBox="0 0 24 24"`, `fill="none"`, `stroke="currentColor"`, `stroke-width="1.5"`, default (butt) caps and miter joins, no fills. No icon fonts, no emoji.
- Drawn on the 24 grid with ~3.5–4.5 px of margin; rendered at **14/15** (status bar), **16** (buttons with a text label, zoom, close in the inspector, selection toolbar), **18** (top-bar icon buttons, palette search), **20** (sheet close), **22** (phone bars and top bar).
- Colour comes from the button (`currentColor`): `text-3` in desktop chrome, `text`/`text-2` on the phone, `accent` when selected. The network shield is the one icon stroked in `accent` while idle.
- Always `aria-hidden="true"` on the SVG. Icon-only buttons carry `aria-label` (a verb phrase: "Find in document", "Close inspector"); buttons with a visible label need nothing extra.
- Hit area is the button, not the glyph: ≥ 24 px desktop (files use 26–32), ≥ 44 px touch.

Icons drawn so far (copy exactly):

| Name | Markup |
| --- | --- |
| Sidebar | `<rect x="3.5" y="4.5" width="17" height="15" rx="1.5"></rect><path d="M9 4.5v15"></path>` |
| Find / Search | `<circle cx="10.5" cy="10.5" r="6"></circle><path d="M15 15l5 5"></path>` |
| Zoom out | `<path d="M6 12h12"></path>` |
| Zoom in | `<path d="M6 12h12M12 6v12"></path>` |
| More (⋯) | `<circle cx="5.5" cy="12" r="0.9"></circle><circle cx="12" cy="12" r="0.9"></circle><circle cx="18.5" cy="12" r="0.9"></circle>` |
| Close | `<path d="M6 6l12 12M18 6L6 18"></path>` |
| Back | `<path d="M15 5l-7 7 7 7"></path>` |
| Copy | `<rect x="8.5" y="8.5" width="11" height="11" rx="1.5"></rect><path d="M15.5 8.5V6a1.5 1.5 0 0 0-1.5-1.5H6A1.5 1.5 0 0 0 4.5 6v8A1.5 1.5 0 0 0 6 15.5h2.5"></path>` |
| Highlight / Annotate | `<path d="M14.5 4.5l5 5L11 18H6v-5z"></path><path d="M4 20.5h9"></path>` |
| Comment / Note | `<path d="M5 5.5h14v10H10l-4 3.5v-3.5H5z"></path>` |
| Underline | `<path d="M7 4.5v6a5 5 0 0 0 10 0v-6"></path><path d="M5 20h14"></path>` |
| Ink | `<path d="M4 17c3-6 5-9 7-9s1 6 3 6 3-4 6-6"></path>` |
| Shapes | `<rect x="4" y="4" width="10" height="10" rx="1"></rect><circle cx="15.5" cy="15.5" r="4.5"></circle>` |
| Share | `<path d="M12 15V4.5M8 8l4-3.5L16 8"></path><path d="M7 11H5.5v8.5h13V11H17"></path>` |
| Pages | `<rect x="4.5" y="4.5" width="6" height="7.5" rx="1"></rect><rect x="13.5" y="4.5" width="6" height="7.5" rx="1"></rect><rect x="4.5" y="14" width="6" height="5.5" rx="1"></rect><rect x="13.5" y="14" width="6" height="5.5" rx="1"></rect>` |
| Reader (book) | `<path d="M12 6.5c-2-1.3-4.5-2-8-2v13c3.5 0 6 .7 8 2 2-1.3 4.5-2 8-2v-13c-3.5 0-6 .7-8 2z"></path><path d="M12 6.5v13"></path>` |
| Network shield | `<path d="M12 3.5l7 2.5v5.5c0 4.3-3 7.6-7 9-4-1.4-7-4.7-7-9V6z"></path><path d="M9 12l2.2 2.2L15.5 10"></path>` |
| Repaired (wrench) | `<path d="M14.5 5.5a4 4 0 0 0-5 5L4.5 15.5l4 4 5-5a4 4 0 0 0 5-5l-2.5 2.5-2.5-.5-.5-2.5z"></path>` |
| Signed file | `<path d="M4 19c2-1 3-4 5-4s1 3 3 3 2-2 4-2"></path><path d="M14 4.5l5 5-7 7h-2v-2z"></path>` |
| Warning **(proposed)** | `<path d="M12 4l9 16H3z"></path><path d="M12 10v4M12 17v.5"></path>` |
| Chevron down **(proposed)** | `<path d="M7 10l5 5 5-5"></path>` |
| Check **(proposed; stroke 2 inside checkboxes)** | `<path d="M6 12.5l4 4 8-9"></path>` |

Wrap each in:

```html
<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">…</svg>
```

New icons follow the same grid and stroke; add them to this table when a board introduces one.

## 6. Layout

### 6.1 Desktop and web shell (inverted L)

| Region | Size | Background | Notes |
| --- | --- | --- | --- |
| Top bar | `min-height: 40px`, padding `0 16px`, gaps `8px 20px` | `ground`, `hairline` bottom | Three groups: file (flex `1 1 220px`), mode switcher (centred), tools (flex `1 1 220px`, right-aligned, gap 14). Wraps onto two lines when narrow |
| Secondary toolbar | `min-height: 40px`, padding `0 16px`, gaps `4px 6px`, 13 px text | `surface`, `hairline` bottom | Only in modes other than Read |
| Left rail | `220px` (Outline, desktop board) · `196px` (Pages, web board) | `ground`, `hairline` right | Rule **(proposed)**: default 220, min 196, user-resizable, remembered locally |
| Canvas | flex `999 1 560px` (560 min before wrapping), padding `32px 16px` (desktop `28px 16px`) | `gutter` | Pages centred |
| Inspector | `288px`, padding `20px 20px 24px`, gap 22 | `surface`, `hairline` left | Only while something is selected |
| Status bar | `min-height: 28px`, padding `0 16px`, 12 px | `ground`, `hairline` top | Left: network indicator; middle: repaired or signed-file notice; right: page label |
| Window frame (desktop mockup) | title bar 30 px, traffic lights 12 px gap 8, radius 10 | see `window` | Draw for macOS; Windows/Linux differ in controls only |

**Page sheet:** `max-width: 612px` (US Letter width at 100%), `min-height: 790px`, white, page shadow, no border, no radius. Gutter colour fills the rest.

**Breakpoints (proposed):**

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
| Page pill | 30 px high, padding `0 14px`, radius 15, `pill` bg, white 13/500 tabular, 100 px from the bottom (16 above the bar) | `aria-live="polite"`; fades out after scrolling stops **(proposed: 1.5 s)** |
| Bottom bar | 84 px incl. safe area: padding `6px 8px 22px`, 5 equal columns, gap 4, `ground`, `hairline` top | Search, Annotate, Pages, Reader, More. Buttons 56 high, radius 6, 22 px icon over 11.5 px label, gap 3 |
| Bottom action bar | 76 px incl. safe area: padding `4px 8px 18px`, 5 columns, buttons 52 high | Replaces the bottom bar while text is selected: Copy, Highlight, Comment, Share, More |
| Bottom sheet | `surface`, radius `12px 12px 0 0`, `hairline` top, sheet shadow | Grabber button 64×44 with a 36×5 `control-border` bar (radius 3, 8 px from top); header: title 20/500 + count 13 `muted`, close 44×44 |
| Home indicator | 134×5, radius 3, `text` at 0.85, 6–8 px from bottom | Mockups only; real apps leave `env(safe-area-inset-bottom)` |

**Sheet detents:** medium = sheet top at 404 px of 844 (≈ 48% from the top; 364 px tall above a 76 px action bar), from the file. Large **(proposed)** = sheet top 56 px from the top (just under the top bar's space), collapsed = hidden. Tapping the grabber cycles medium → large → medium; dragging snaps; the grabber is a real `<button aria-label="Resize sheet">` (single-pointer alternative to the drag, WCAG 2.5.7). The page stays visible above a medium sheet.

**Tablet (1024 × 1366, proposed):** desktop shell with touch rules: rail as a docked panel (collapsed by default in portrait), inspector docked, bottom action bar instead of the mini toolbar, 44 px targets.

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

### 7.1 Top bar

Anatomy: sidebar toggle · file name (Newsreader 16/500, ellipsis) + save state (12 `muted`: "No unsaved changes", "1 unsaved change") · mode switcher · page input `n` + `/ N` · zoom −/%/+ · find · Commands button with shortcut chip · Save (primary, only when there are changes) · More.

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
      <span style="color: #6B645A">/ 24</span>
    </div>
    <!-- zoom: two 26×26 icon buttons around <span style="min-width: 40px; text-align: center">110%</span> -->
    <!-- find: icon button (7.3) -->
    <button style="height: 26px; display: flex; align-items: center; gap: 8px; padding: 0 6px 0 10px; border: 1px solid #E4DED3; background: #FBF9F5; border-radius: 4px; color: #6B645A; font-size: 13px"><span>Commands</span><span style="font-size: 11px; padding: 1px 5px; border: 1px solid #D8D1C5; border-radius: 3px; color: #4A453E; background: #FFFFFF">Ctrl K</span></button>
    <button style="height: 26px; padding: 0 12px; border: 1px solid #1F5C63; background: #1F5C63; color: #FFFFFF; border-radius: 4px; font-size: 13px; font-weight: 600">Save</button>
    <!-- more: icon button -->
  </div>
</header>
```

### 7.2 Mode switcher

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

Desktop 28×28 (top bar, inspector close), 26×26 (zoom), 32×32 (style tile); phone 44×44 with 22 px icon, radius 6. Radius 4, transparent, `text-3`.

```html
<button aria-label="Find in document" style="width: 28px; height: 28px; display: flex; align-items: center; justify-content: center; border: 0; background: transparent; border-radius: 4px; color: #4A453E"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6"></circle><path d="M15 15l5 5"></path></svg></button>
```

### 7.4 Text buttons

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

`role="tablist"` with `role="tab"`/`aria-selected`; 13 px, gap 16, padding `14px 18px 0`, 2 px `accent` underline on the selected tab, `hairline` under the row. Arrow keys move between tabs.

```html
<div role="tablist" aria-label="Sidebar" style="display: flex; gap: 16px; padding: 14px 18px 0; border-bottom: 1px solid #E4DED3">
  <button role="tab" aria-selected="true" style="border: 0; background: transparent; padding: 0 0 8px; font-size: 13px; font-weight: 600; color: #1F1D1A; border-bottom: 2px solid #1F5C63">Pages</button>
  <button role="tab" aria-selected="false" style="border: 0; background: transparent; padding: 0 0 8px; font-size: 13px; color: #6B645A; border-bottom: 2px solid transparent">Outline</button>
  <button role="tab" aria-selected="false" style="border: 0; background: transparent; padding: 0 0 8px; font-size: 13px; color: #6B645A; border-bottom: 2px solid transparent">Bookmarks</button>
</div>
```

### 7.6 Thumbnail

116×150 in the rail (column, gap 22, padding 22 0), 122×158 in the phone sheet (2-column grid, gaps `18px 16px`). White, thumbnail shadow, 1 px `hairline` outline; **current page**: `outline: 2px solid #1F5C63; outline-offset: 3px`, `aria-current="page"`, number 600 `text`; others number `muted`. Number 12 px (13 on phone) tabular, 8 px below.

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

Links in a `<nav aria-label="Document outline">`, padding `14px 10px`, gap 2. Row: Newsreader 15, padding `6px 8px`, radius 4, page number right (Source Sans 3 12 `muted` tabular). Level 2: 14 px, `text-3`, left padding 22. Current: `accent` text 600 on `accent-tint`, `aria-current="location"`. Hover **(proposed)**: `hover` fill. Expand/collapse **(proposed)**: a 16 px chevron button before the label, `aria-expanded`.

```html
<a href="#" aria-current="location" style="display: flex; justify-content: space-between; gap: 8px; padding: 6px 8px; border-radius: 4px; color: #1F5C63; background: #E1E6E3; text-decoration: none; font-family: 'Newsreader', Georgia, serif; font-size: 15px; font-weight: 600"><span>2. Repairs backlog</span><span style="font-family: 'Source Sans 3', system-ui, sans-serif; font-size: 12px; font-weight: 400; font-variant-numeric: tabular-nums">3</span></a>
<a href="#" style="display: flex; justify-content: space-between; gap: 8px; padding: 5px 8px 5px 22px; border-radius: 4px; color: #4A453E; text-decoration: none; font-family: 'Newsreader', Georgia, serif; font-size: 14px"><span>Open repairs by region</span><span style="font-family: 'Source Sans 3', system-ui, sans-serif; font-size: 12px; color: #6B645A; font-variant-numeric: tabular-nums">3</span></a>
```

### 7.8 Status bar items

Each item is a button (min-height 24, 12 px, `muted`) that opens its panel. Network indicator: shield in `accent`, count in `text`, explanation in `muted`. Repaired notice: wrench + "Repaired on open · Details" (Details underlined). Page label: plain text, "Page 3 · Repairs backlog" or "Page 3 of 24". If requests > 0 **(proposed)**: count stays `text`, explanation names the host ("2 requests · to files.example.org only"); a request to an unexpected origin turns the shield and count `warning`.

```html
<footer style="min-height: 28px; display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 4px 16px; padding: 0 16px; border-top: 1px solid #E4DED3; background: #F7F4EE; font-size: 12px; color: #6B645A">
  <button style="min-height: 24px; display: flex; align-items: center; gap: 7px; border: 0; background: transparent; padding: 0 4px; font-size: 12px; color: #6B645A">
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#1F5C63" stroke-width="1.5" aria-hidden="true"><path d="M12 3.5l7 2.5v5.5c0 4.3-3 7.6-7 9-4-1.4-7-4.7-7-9V6z"></path><path d="M9 12l2.2 2.2L15.5 10"></path></svg>
    <span style="color: #1F1D1A; font-variant-numeric: tabular-nums">0 network requests</span><span>· Offline-safe: nothing uploaded</span>
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

### 7.11 Inspector panel

Docked right, 288 px, `surface`. Sections stack with gap 22:

1. **Header:** title (Newsreader 20/500, line height 1.2) over a 12 `muted` context line ("Page 3 · Repairs backlog"); close icon button (16 px icon) top right.
2. **Subject preview:** for text annotations, the quoted text (Newsreader 15/1.5 italic `text-2`, curly quotes).
3. **Fields:** label (12/600 `text-3`, a real `<label>` when it labels an input) over the control, gap 8 (4 for read-only values).
4. **Actions:** pinned to the bottom (`margin-top: auto`), gap 8: secondary + danger ghost, 30 px high, 13 px.

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

Circles with `aria-label` (colour name) and `aria-pressed`; 24 px in the secondary toolbar (gap 4, preceded by a 12 `muted` "Colour" label), 28 px in the inspector (gap 8). Edge 1 px `control-border`; selected `outline: 2px solid #1F1D1A` (offset 1 / 2). On touch, wrap each in a 44 px button **(proposed)**.

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

**Banner (proposed)** when saving would *change* what a signature covers: a strip under the top bar (and secondary toolbar), 36 px min, `warning-tint`, `hairline` bottom, signature glyph and lead word in `warning`, 13 px `text`, actions right. It appears with the first edit that triggers it (not at save time) and stays until the edit is undone or saved. Wording: "**Saving will invalidate the signature.** This file is certified to allow no changes. Save a copy instead to keep the original signed." with buttons "Save a copy" (secondary) and "Details" (ghost).

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

### 7.17 Page indicator pill (phone)

```html
<div aria-live="polite" style="position: absolute; left: 50%; bottom: 100px; transform: translateX(-50%); height: 30px; display: flex; align-items: center; padding: 0 14px; border-radius: 15px; background: rgba(31,29,26,0.78); color: #FFFFFF; font-size: 13px; font-weight: 500; font-variant-numeric: tabular-nums; letter-spacing: 0.02em">3 / 24</div>
```

### 7.18 Dialog (proposed)

Built like the palette: `scrim`, centred card `max-width: 440px` (forms up to 560), `raised` = `surface` in light, `chip-border` edge, radius 6, palette shadow, padding 24, gap 16. Title Newsreader 20/500; body 14 `text`; actions right-aligned, gap 8, primary last (danger-filled for destructive). Close icon button top right unless the dialog asks a required question. `role="dialog"` (or `alertdialog` for destructive confirms), `aria-modal="true"`, `aria-labelledby` the title, focus trapped, Escape closes, focus returns to the opener. Phone: becomes a bottom sheet at the large detent with actions full width (44 px high).

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

### 7.19 Form controls (proposed, consistent with the page input and note field)

All: `control-border` 1 px, white fill, radius 4, 14 px text (16 on touch), label above (12/600 `text-3`) with gap 8, help text below (12 `muted`), error text below (12 `danger`, with `aria-describedby` and `aria-invalid="true"`; the border turns `danger`). Focus: the 2 px `accent` ring, offset 1.

| Control | Size | Details |
| --- | --- | --- |
| Text input | 32 high (desktop), 44 (touch), padding `0 10px` | Placeholder `muted`; never use placeholder as label |
| Textarea | padding `8px 10px`, line height 1.45, `resize: vertical` | From the inspector |
| Select | 32 high, padding `0 30px 0 10px`, chevron `<path d="M7 10l5 5 5-5"></path>` 16 px `text-3` at right 8 | Native `<select>` styled with `appearance: none` |
| Checkbox | 16×16 box, radius 3, in a ≥ 24 px row; checked: `accent` fill, white check `<path d="M6 12.5l4 4 8-9"></path>` stroke 2 | Label 14 px, gap 8 |
| Radio | 16×16 circle; selected: `accent` 2 px ring + 8 px `accent` dot | In a `role="radiogroup"` / `<fieldset>` with `<legend>` |
| Toggle (switch) | track 32×18 radius 9; off: white, `control-border`; on: `accent` fill; knob 12 px (off `control-border` colour, on white) | `<button role="switch" aria-checked>`; label left of it; use only for settings that apply at once |

```html
<label style="display: flex; align-items: center; gap: 8px; min-height: 24px">
  <input type="checkbox" checked style="width: 16px; height: 16px; margin: 0; accent-color: #1F5C63"> Single-key shortcuts
</label>
<div style="display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 24px">
  <span id="sw-l">Show selection toolbar</span>
  <button role="switch" aria-checked="true" aria-labelledby="sw-l" style="width: 32px; height: 18px; padding: 0; border: 1px solid #1F5C63; border-radius: 9px; background: #1F5C63; position: relative"><span style="position: absolute; top: 2px; right: 2px; width: 12px; height: 12px; border-radius: 50%; background: #FFFFFF"></span></button>
</div>
```

### 7.20 Toast and notice (proposed)

- **Toast:** short confirmation of an action that already happened ("Copied", "Highlights exported"). Bottom centre, 24 px above the status bar (phone: above the bottom bar, where the pill sits; it replaces the pill). `text` bg `#1F1D1A`, white 13 px (16.81), radius 6, padding `8px 14px`, floating shadow; an optional action is white underlined text ("Undo"). `role="status"`; 4 s, paused while hovered or focused; never the only place an error is reported.
- **Notice:** an inline strip inside a panel or under the bars for a lasting state (repaired file details, approximate reflow, offline). Tint background (`warning-tint`, `danger-tint`, `positive-tint` or `surface`), `hairline` edge, radius 4, padding `10px 12px`, 16 px icon in the status colour, title 600 + body 13 px `text`. No left-border accent stripe.

```html
<div role="status" style="display: flex; gap: 10px; padding: 10px 12px; background: #F0E8DC; border: 1px solid #E4DED3; border-radius: 4px; font-size: 13px">
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#8A5300" stroke-width="1.5" aria-hidden="true"><path d="M12 4l9 16H3z"></path><path d="M12 10v4M12 17v.5"></path></svg>
  <span><strong style="font-weight: 600">Approximate layout.</strong> This file has no tags, so lamina guessed the reading order.</span>
</div>
```

### 7.21 Empty state (proposed)

Centred in the region it fills, max-width 360, gap 12: optional 32 px stroke icon `muted`; title Newsreader 20/500 (Display 40/44 only on the home screen); one sentence 14 `muted`; at most one primary and one ghost action. Example: Bookmarks tab, "No bookmarks yet" / "Choose Add bookmark from a page’s More menu."

### 7.22 Progress (proposed)

- **Determinate bar:** 4 px track `hairline`, radius 2, `accent` fill (5.66 on the track), label above (12 `muted`, tabular: "Saving… 62%"), `role="progressbar"` with `aria-valuenow`.
- **Indeterminate:** the same bar with a 30% segment sliding (1.2 s, linear); with `prefers-reduced-motion`, a static 30% segment plus the label.
- **Spinner:** avoid; pages use placeholders, sidebars skeletons (`#ECE7DE` blocks, no shimmer under reduced motion).

### 7.23 Badge and tag (proposed)

Height 20, padding `0 8px`, radius 3, 12 px 600, tint background with the status colour as text: neutral `surface` + `chip-border` + `text-3`; accent `accent-tint` + `accent`; positive `positive-tint` + `positive` ("Signature valid"); warning `warning-tint` + `warning` ("Approximate"); danger `danger-tint` + `danger` ("Leak found"). Pair with an icon or word; never colour alone.

```html
<span style="display: inline-flex; align-items: center; gap: 4px; height: 20px; padding: 0 8px; border-radius: 3px; background: #E7EBE3; color: #2F6A45; font-size: 12px; font-weight: 600"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M12 3.5l7 2.5v5.5c0 4.3-3 7.6-7 9-4-1.4-7-4.7-7-9V6z"></path><path d="M9 12l2.2 2.2L15.5 10"></path></svg>Signature valid</span>
```

### 7.24 Secondary-toolbar tool toggle

From the Comment toolbar: 28 high, padding `0 10px`, gap 6, radius 4, 16 px icon + 13 px label, 1 px transparent border; selected = `accent` border, `accent-tint`, `accent` 600. Right end: a 12 `muted` context note ("1 comment on this page").

```html
<button aria-pressed="true" style="height: 28px; display: flex; align-items: center; gap: 6px; padding: 0 10px; border: 1px solid #1F5C63; background: #E1E6E3; color: #1F5C63; border-radius: 4px; font-weight: 600"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M14.5 4.5l5 5L11 18H6v-5z"></path><path d="M4 20.5h9"></path></svg>Highlight</button>
```

### 7.25 Page-appearance chips

From the style tile: 4-column grid gap 10; chip padding 8, gap 8, radius 6; preview 52 high, radius 3, "Aa" in Source Serif 4 20 px on the appearance's paper and ink; selected chip white with a 2 px `accent` border and a 13/600 label, others `surface` with `chip-border` and 13/400. `role="group"` + `aria-pressed`.

## 8. Patterns

### 8.1 Modes and their secondary toolbars

One toolbar per mode, under the top bar, never two. Tools are toggles (§7.24); colour swatches follow a divider; a context note sits at the right end.

| Mode | Secondary toolbar |
| --- | --- |
| Read | None (the default; page + top bar only) |
| Comment (from the files) | Highlight · Underline · Note · Ink · Shapes │ Colour (4 swatches) · … · "1 comment on this page" |
| Fill & Sign **(proposed)** | Previous field · Next field │ Text · Check · Date │ "Add a signature image" · "Sign with a certificate" · … · "3 of 8 required fields filled" |
| Organise **(proposed)** | Insert · Rotate left · Rotate right · Extract · Delete │ "Move to page…" · … · "2 pages selected". The canvas becomes the page grid |
| Edit **(proposed)** | Text · Image · Link │ Redact · Protect (tools that open their own toolbar: Redact = Mark · Review · Apply) · … · context note |

### 8.2 Selection → inspector

Selecting an annotation, field, page or object opens the docked inspector (it pushes the canvas, never covers it) and outlines the object on the page with the 2 px `accent` focus outline. Deselecting (Escape, click empty page) closes it. Text selection does **not** open the inspector; it shows the mini toolbar (desktop) or the action bar (touch). Creating an annotation selects it.

### 8.3 Focus mode

Hides the rail and the secondary toolbar (top and status bars stay); a selection brings the inspector back until deselected. Toggle: palette command "Focus mode" and `⌘\` / `Ctrl \` **(proposed)**; leaving it is the same shortcut or Escape with nothing selected.

### 8.4 Explicit save

No auto-save. The top bar shows the state ("No unsaved changes", "1 unsaved change", "Saving…", "Saved"); Save (primary) appears only with changes. Saving is incremental by default. Before a save changes anything about signatures, say so in the status-bar note or the banner (§7.13). Wording:

- "Signed file. Saving adds your changes after the signature; the signature stays valid." (from the files)
- "Saving will invalidate the signature. …" (banner, proposed)
- Closing with changes: dialog "Save changes to Q3-board-report.pdf?" with "Save", "Don't save", "Cancel".

### 8.5 Trust surfaces

- **Network indicator:** always in the status bar (desktop/web) and in More on the phone; opens a panel listing each origin and what the count proves.
- **Signature badge:** a badge (§7.23) in the status bar and the signature's inspector: "Signature valid", "Signed, changed after signing" (warning), "Signature not trusted" (danger) **(proposed wording)**.
- **Redaction report:** after Apply, a panel listing each check with positive/danger badges; Save is disabled with a reason ("Saving is blocked: 1 leak found") while any check fails.
- **Repaired file:** status-bar notice "Repaired on open · Details"; details in a panel, never a modal.

### 8.6 Page appearance vs UI theme

Two separate settings in View settings: **Theme** (Light, Dark, High contrast, Match system) recolours the chrome; **Page appearance** (Original, Sepia, Dark, High contrast) recolours page content in the renderer. Original is the default in every theme: draw white pages on the dark gutter in dark mockups. Photos and annotation colours never change.

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

## 9. Writing style

- **Sentence case** for everything except the product's mode names, which are proper nouns ("Fill & Sign", "Organise").
- **British spelling** (colour, organise), curly quotes and apostrophes (’ “ ”), the middle dot `·` to join short facts, `…` for commands that ask for more input ("Go to page…", "Move to page…").
- **Plain words, the user's object first:** "Delete highlight", not "Remove annotation object"; say what happens, not what the system does.
- **Numbers:** tabular, thousands with commas ("1,248"), percentages without a space ("110%"), dates as "28 Oct 2026, 16:04".
- **States say the fact:** "No unsaved changes", "1 unsaved change", "1 comment on this page", "24 pages".
- **Trust copy states proof, not reassurance:** "0 network requests · Offline-safe: nothing uploaded"; "Signed file. Saving adds your changes after the signature; the signature stays valid."; "Repaired on open · Details".
- **Hints are short imperatives:** "↑ ↓ to move", "↵ to run", "Type “go to 12” or “zoom 150” to pass a value".
- **Labels for icon-only buttons** are verb phrases: "Find in document", "Close inspector", "Back to files", "Resize sheet", "More selection actions".
- No exclamation marks, no "Oops", no blame ("This file has a damaged cross-reference table; lamina rebuilt it", not "Invalid file").
- Pricing, when shown: placeholders `[PLAN NAME]`, `[PRICE]`; never a lock icon on a feature the user is already using.

**Sample document convention.** Every mockup shows **Northwind Housing Trust — Q3 Board Report** (`Q3-board-report.pdf`, 24 pages, fictional). Page 3: running header "Northwind Housing Trust · Q3 Board Report", heading "2. Repairs backlog", the three paragraphs from the files, the chart "Open repairs by region, end of September" (North 412, East 288, South 351, West 197), footer "Page 3 of 24". The selected or highlighted sentence is "The backlog fell by 18% over the quarter, but the oldest 10% of jobs are now older than 90 days."; the highlight's author is "R. Osei", note "Ask for the 90-day list by region.", created "28 Oct 2026, 16:04". Outline: 1. Chair’s summary (1) · 2. Repairs backlog (3) › Open repairs by region (3) · 3. Rent collection (7) · 4. Surveyor capacity (11) · 5. Risk register (16) · Appendix A. Data notes (22). Invent new facts only inside this document's world, never statistics about real products; no lorem ipsum.

## 10. Authoring notes for the Design canvas

- **Reuse the snippets** in §7 and the icons in §5 verbatim; change copy, not structure. If a board needs a new component, build it from these tokens and add it here.
- Each artboard is one `.dc.html` file under `project/`, listed in `canvas.json` (`boards`, `order`, optional `notes`). Keep the file skeleton: `<script src="./support.js"></script>` in `<head>`, markup inside `<x-dc>`, fonts and base CSS in `<helmet>` (the §3 link line, plus `body{margin:0}`, `a{color:#1F5C63}a:hover{color:#16454B}`, `button,input,textarea{font:inherit;color:inherit}`), and logic in `<script type="text/x-dc" data-dc-script data-props='…'>` with a `Component extends DCLogic` whose `renderVals()` returns the template values. Loops and conditions use `<sc-for list="{{…}}" as="t" hint-placeholder-count="n">` and `<sc-if value="{{…}}" hint-placeholder-val="{{ true }}">`.
- **Artboard sizes:**
  - Web and desktop: a PAGE board, `"w": 1440`, `"h": 900`, `"expand": "fill"` in `canvas.json`, `$preview` `{"width":1440,"height":900}`; the root is fluid (`min-height: 100vh; display: flex; flex-direction: column`).
  - Phone: fixed `390 × 844`, root `width: 390px; height: 844px; position: relative; overflow: hidden`.
  - Tablet (if used): fixed `1024 × 1366`.
- **Never use % heights**; use flex, `min-height: 100vh` on page roots, or fixed px on fixed artboards. Widths may use % inside a sized parent (chart bars).
- Tweaks: at most 2–3 `data-props` per artboard (the files use `accent` with options `#1F5C63`, `#2D4A6B`, `#3E4A3C`, plus one boolean such as `showRail`, `showPalette` or an enum such as `pageAppearance`). Copy stays literal markup.
- Draw the macOS window frame for desktop boards (§6.1); no browser chrome for web boards; no fake device status bar on phones (the home indicator is allowed).
- Show one keyboard focus ring per board; use real elements and ARIA as in the snippets.
- No gradients, no left-border accent cards, no emoji, no shadows on docked panels.
