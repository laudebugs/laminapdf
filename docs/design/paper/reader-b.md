# Paper: Reader B

View settings and page appearance, the dark UI theme, keyboard shortcut help, the network indicator panel, reader mode, offline and install, focus mode, tablet reading and the phone sheets: screens R10–R18 of the [screen inventory](screens.md#r-reader-web-reader-v01-and-the-shared-viewer), drawn in the [Paper design language](design-language.md).

- **Canvas:** [lamina Paper — Reader B](https://claude.ai/artifact/KaCtzmB4hNEHgyJJQv2HBi) (one row per screen, a sticky note on the left of each row).
- **Platform:** web boards are drawn as the web app on Windows (shortcut chips use `Ctrl`). macOS uses the glyphs (`⌘K`, `⌘\`).
- **Sample content:** Q3-board-report.pdf, page 3 “2. Repairs backlog”. Page 4 (a table), section 3 text (“Rent collected in the quarter came to 98.1% …”), the host `files.northwind.example`, the open times and the request sizes are invented within the sample’s world.

## R10 View settings

**Purpose.** Set how the document looks in one place: the UI theme, the page appearance, the page layout and the zoom.

**Entry points.** On web, the *View settings* icon button in the top bar, between zoom and find. On the phone, More › View settings. Palette commands: `view.settings`, `view.theme.light` / `.dark` / `.high-contrast` / `.system`, `view.page-appearance.original` / `.sepia` / `.dark` / `.high-contrast`, `view.layout.single` / `.two-up`, `view.continuous`, `view.zoom.fit-width` / `.fit-page` / `.actual-size`. None of these has a shortcut; zoom keys are listed in R12.

**Layout (web).** A popover under the button, 360 px wide: `surface`, `chip-border` edge, radius 6, menu shadow, z 30. It is a non-modal `role="dialog"` titled “View” (Newsreader 18). It sits over the top-right of the canvas and never covers the top bar. Sections, top to bottom:

| Section | Control | Notes |
| --- | --- | --- |
| Theme | 4-way radio group: System · Light · Dark · High contrast | Help line names what “System” currently resolves to (“Following your system: light”) |
| Page appearance | 4 appearance chips (§7.25, 40 px previews) + radio “This document / All documents” | Help: “Photos and annotation colours stay as they are.” |
| Page layout | Radio Single page · Two pages (icons) + switch “Continuous scrolling” | Two pages shows facing pages; page 1 alone on the right, as books do (setting in the doc only, not drawn) |
| Zoom | Radio chips Fit width · Fit page · 75% · 100% · 150% · 200% | When the zoom matches no preset (e.g. 113% after pinching), none of the chips is selected |
| Footer | “Saved on this device only” · ghost “More settings…” | Opens C7a’s settings dialog (Appearance and accessibility section: reduced motion and the other options) |

**States.** The board shows Sepia selected for this document, with the page and thumbnails recoloured. *R10-page-dark* shows the result of choosing Dark: page text and vector paths are recoloured (bars `#AEB8BF`), the yellow highlight keeps its colour, the thumbnails follow the page, and a toast reads “Page appearance: Dark, for this document only · Undo”. In High contrast the page uses `forced-colors` system colours. When `forced-colors: active` is detected the page appearance switches to High contrast unless the user chose something else (U2c). If a page draws its text as images in Dark, a notice on that page says “Text on this page is part of an image, so it can’t be recoloured.” (not drawn).

**Behaviour.** Every control applies at once (live preview). Nothing in the file changes: this is a view preference, stored in `localStorage` (wrapped in try/catch) per document or globally. Undo is the toast, or choosing again. Changing the appearance re-renders only the visible tiles and keeps the scroll position (U2c). Escape or a click outside closes the popover. Focus returns to the View settings button.

**Accessibility.** The trigger has `aria-expanded` and `aria-controls`. Each group is a `radiogroup` labelled by its section label. Arrow keys move inside a group and Tab moves between groups. The appearance chips use `aria-pressed` and have a name as well as a colour. The board shows the focus ring on the selected Sepia chip.

**Platform differences.** On the phone (*R10-view-settings-phone*) it is a sheet at the medium detent, opened from More (More shows as active). Page appearance comes first, because it is what people change while reading. The theme radios are 44 px, zoom offers Fit width and Fit page only (pinch does the rest), and there is no two-up layout. The desktop app shows the same popover, and also puts these items in the native View menu (D4b).

**Artboards.** `R10-view-settings.dc.html`, `R10-page-dark.dc.html`, `R10-view-settings-phone.dc.html`.

**Todos.** [C7a](../../todos/ready/C7a-accessibility-and-theming.md), [U2c](../../todos/ready/U2c-page-appearance-modes.md).

## R11 Dark UI theme

**Purpose.** The F01 reader in dark chrome, built from the [proposed dark tokens](design-language.md#23-dark-ui-theme-proposed). The page stays Original (white) unless the user picks a page appearance.

**Entry points.** View settings › Theme › Dark, or System when the OS is dark (`prefers-color-scheme`). Command `view.theme.dark`.

**Layout.** The shell is the same as F01. The chrome is `ground #1E1D1A`, the Commands button and fields use `surface #262421` / `field #1E1D1A`, and the gutter is `#141311`. Dividers are `hairline #3A3732`, and the page input border is `control-border #7E766A`. Text uses `#ECE7DE`, icons `#CFC8BC`, muted text `#A39B8E`. Accent, focus and the network shield are `#7DBDC2`. The page shadow is the proposed dark one. The selection mini toolbar is `raised #2E2C28` with a `chip-border #4A463F` edge and a black shadow. The selection on the white page keeps the Original tint `#CEDBDD`, because the selection tint follows the page appearance, not the UI theme.

**States.** The board shows text selected with the mini toolbar, and the focus ring on Highlight. The `pageAppearance` tweak switches the page to Dark (selection tint `#425858`, thumbnails dark). High contrast UI is not drawn: it follows `forced-colors` system colours (plan §5.5).

**Behaviour.** Switching theme only swaps CSS custom properties: no re-render, no layout change. With reduced motion there is no transition.

**Accessibility.** Every pair meets the ratios in the dark token table: muted on ground 6.13, accent on ground 7.97, and controls bordered with `control-border` (≥ 3.11). `color-scheme: dark` makes native controls and scrollbars dark. The hover fill is `rgba(236,231,222,0.08)`.

**Platform differences.** Desktop: the window frame follows the OS appearance. Phone: the same tokens apply to the bars and sheets (not drawn).

**Artboards.** `R11-dark-theme.dc.html`.

**Todos.** [C7a](../../todos/ready/C7a-accessibility-and-theming.md).

## R12 Keyboard shortcuts

**Purpose.** Lists every shortcut for the active platform. The list is generated from the command registry (`useShortcutMap()`), never written by hand.

**Entry points.** `?` (page focus), palette command `help.shortcuts`, More › Keyboard shortcuts. The desktop app adds F1 and Help › Keyboard shortcuts. F1 is not bound on the web, where browsers use it for their own help.

**Layout.** A dialog over the dimmed shell (`scrim`), `max-width: 760px`. It is wider than the 440 default because the shortcuts sit in two columns. Its parts:

- **Header:** title “Keyboard shortcuts”, a subline naming the platform and the single-key rule, and a close button.
- **Search field:** focused on open.
- **Shortcut list:** groups General, Pages, Find, Zoom and reading, Selection. Each group has a Newsreader 16 heading over a hairline, and rows of name + `<kbd>` chips.
- **Footer:** “Commands without a shortcut are in the command palette.” and the **Single-key shortcuts** switch (WCAG 2.1.4). The switch is the same setting as in Settings.

Shortcuts shown (Windows/Linux; macOS swaps `Ctrl` for `⌘`):

| Command id | Name | Shortcut |
| --- | --- | --- |
| `palette.open` | Command palette | Ctrl K |
| `help.shortcuts` | Keyboard shortcuts | ? |
| `file.open` | Open a file | Ctrl O |
| `file.download-copy` | Download a copy | Ctrl S |
| `file.print` | Print | Ctrl P |
| `view.focus-mode` | Focus mode | Ctrl \ |
| `layer.close` | Close the open panel or dialog | Esc |
| `nav.next-page` / `nav.previous-page` | Next / previous page | Page Down / Page Up |
| `nav.first-page` / `nav.last-page` | First / last page | Home / End |
| `nav.back` / `nav.forward` | Back after following a link / Forward | Alt ← / Alt → (on the web these are the browser’s own Back and Forward: link jumps are pushed to history, C6b) |
| `find.open` | Find in document | Ctrl F |
| `find.next` / `find.previous` | Next / previous match | Enter / Shift Enter (in the find field) |
| `view.zoom-in` / `view.zoom-out` / `view.zoom.actual-size` | Zoom in / out / actual size | + / − / 0 (single keys, page focus; Ctrl + and Ctrl − stay the browser’s) |
| `view.reader-mode` | Reader mode | R (single key) |
| `edit.copy` | Copy | Ctrl C |
| `selection.actions` | Show selection actions | Shift F10 |
| `comment.highlight` / `comment.delete` | Highlight / delete highlight (Comment mode) | H / Backspace (⌫ on macOS) |

**States.** The full list is drawn, with the search field focused. *Filtered:* typing matches names and keywords, empty groups hide, and the match is set in 600. *No match:* “No shortcut matches ‘…’. Try the command palette.” plus a ghost “Open command palette” button. With single-key shortcuts off, their rows stay listed but are labelled “Off”, and the chips are drawn in the disabled colour.

**Behaviour.** Read-only. Escape closes the dialog and focus returns to the opener.

**Accessibility.** `role="dialog"`, `aria-modal`, labelled by the title and described by the subline. The list is real `<dl>`/`<kbd>`, and the search field filters live, announcing the count (“6 shortcuts”) politely.

**Platform differences.** macOS shows glyph chips. Desktop adds F1 and F6 (region cycling). Phone and tablet without a keyboard don’t list it; tablet with a hardware keyboard shows it from More.

**Artboards.** `R12-shortcut-help.dc.html`.

**Todos.** [U1b](../../todos/ready/U1b-command-registry-and-palette.md), [C7a](../../todos/ready/C7a-accessibility-and-theming.md).

## R13 Network activity panel

**Purpose.** Proves what left the device: every request since the document opened, grouped by origin and kind, and what the count proves (principle 7).

**Entry points.** The status-bar network indicator (a button; it gets the pressed fill and `aria-expanded="true"` while open). Palette `privacy.network-activity`. On the phone: More › Network activity, which shows the count as a subline.

**Layout.** A docked panel in the inspector slot (`<aside aria-label="Network activity">`, 320 px, `surface`). It pushes the canvas and never floats. While the panel is open, a selection opens the inspector in its place. Sections:

1. Header: “Network activity”, “Since you opened this file at 09:12”, close button.
2. Summary: the count in Newsreader 40 + “document requests”, one sentence of fact, and a badge.
3. **Document traffic:** a table of origin · requests · received bytes, with a purpose line per origin.
4. **App files** (“Not document traffic”): same-origin app assets (renderer WASM, scripts and styles, fonts). They are listed separately, and collapsed when there is document traffic. One classifier decides which group a request falls in (U4a).
5. **Counted from:** page, document worker, offline service worker. A feed that doesn’t exist yet reads “Offline service worker · not installed” with no check.
6. **What this proves:** two short paragraphs, including the limit (browser extensions aren’t visible).
7. Actions: “Copy report” (copies a plain-text summary) and the ghost “How to check it yourself”, which opens the privacy page section on developer tools.

**States.**

- *Local file* (R13-network-panel): 0 document requests, “No part of this file has left your device.”, positive badge “Nothing uploaded”.
- *File from a URL* (R13-network-remote): 5 range requests to `files.northwind.example`, accent badge “The file’s own host only”. The status bar reads “5 requests · to files.northwind.example only”.
- *Unexpected origin* (not drawn): any document request to an origin other than the file’s host turns the shield and count `warning`. A warning-tint notice heads the panel: “1 request went to an unexpected site: example-cdn.net. lamina didn’t make it on purpose; tell us.” The row gets a warning badge.
- *Blocked by policy* (not drawn): a CSP violation is listed as a row with a neutral badge “Blocked”. It doesn’t count as traffic, because nothing was sent.

**Behaviour.** Live: the counts update as requests happen, and the summary is announced politely at most once every 5 s. Read-only, so nothing in the file changes.

**Accessibility.** Real `<table>`s with captions and `scope` headers. The status item is keyboard-reachable and announced (“0 network requests, offline-safe: nothing uploaded, button, collapsed”). The board shows the focus ring on “How to check it yourself” (local) and on Close (remote).

**Platform differences.** The desktop app shows the indicator only if something networks (updates): U4a scope. Phone: the More row opens this panel as a sheet at the large detent.

**Artboards.** `R13-network-panel.dc.html`, `R13-network-remote.dc.html`.

**Todos.** [U4a](../../todos/ready/U4a-network-indicator-and-privacy-proof.md).

## R14 Reader mode

**Purpose.** Reflowed, adjustable text derived from the structure tree (tier 1). For untagged files the text comes from layout heuristics instead, labelled **Approximate** (tier 2). Everything runs on the device, with no page cap.

**Entry points.** The *Reader* toggle in the top bar (it replaces the zoom controls while active), the phone bottom-bar Reader button, palette `view.reader-mode`, and the single key `R`.

**Layout (web).** The canvas becomes a reading column (`max-width` from Line length, Source Serif 4 at 19 px / 1.65) on the chosen appearance’s paper. The top of the column holds a secondary “Back to page 3” button and the source badge (accent “From the document’s tags”, or warning “Approximate”). The Reader settings panel is docked in the inspector slot, toggled by the settings icon button next to Reader. The status bar shows “Reader mode · from tags / approximate”, and its page label follows the scroll position.

| Setting | Control | Values |
| --- | --- | --- |
| Text size | Stepper: “Smaller text” · output · “Larger text” | 14–32 px, steps of 1 (output is `aria-live`) |
| Line spacing | Radio | Tight · Normal · Loose |
| Line length | Radio | Narrow · Medium · Wide (about 55 / 68 / 85 characters) |
| Appearance | Chips | Original · Sepia · Dark (via CSS; High contrast follows `forced-colors`) |
| Reset to defaults | Ghost button | |

Reader settings are global and stored locally.

**Content rules.** Headings, lists, quotes and links come out as real HTML. Tables, figures and formulas are shown as page crops with their `/Alt` text, or with “The file gives no description of it” when there is none. Running headers and footers are dropped.

**States.**

- *Tagged* (R14-reader-tagged): settings panel open, focus on “Larger text”.
- *Approximate* (R14-reader-approximate):
  - A warning notice heads the column: “**Approximate layout.** This file has no tags, so lamina guessed the reading order from the page layout. Check figures and numbers against the page.” It offers “Back to page 3” and “How Reader mode works”.
  - Every block reveals **View on page n** on hover or focus (drawn focused). It returns to the block’s bbox on the page and briefly outlines it.
  - A page below the confidence threshold appears as a **Page image** block with one sentence of reason and “Open page 4”.
- *Not available* (not drawn): when the sanity checks fail and no tier 2 exists, the Reader toggle stays enabled. Turning it on shows an empty state instead of the column: “Reader mode isn’t available for this document” / “Its structure doesn’t match its layout, so reflowed text would be in the wrong order.” with a primary “Back to page”.
- *Loading* (not drawn): the first blocks appear as skeleton lines, with no spinner.

**Behaviour.** Entering keeps the position, as the block nearest the viewport top. Leaving (toggle, “Back to page”, Escape) returns to the same page region (U4b acceptance). The file never changes. Find works on the reflowed text and highlights in the column.

**Accessibility.** The reader view is a `main` region with `lang` from `/Lang`. Text scales to 200% without horizontal scrolling. Crops are `role="img"` with the alt text as their label. The Approximate notice is `role="status"`, announced once on entry.

**Platform differences.** On the phone (*R14-reader-phone*), the top bar shows Back (“Back to page 3”), the title with “Reader · from the document’s tags”, and a Reader settings button. Reader is active in the bottom bar (accent tint), and text is 18–20 px. Settings open as a sheet at the medium detent (*R14-reader-approximate-phone*, which also shows the Approximate notice) with 44 px controls and no line-length control. On the phone, tapping a block reveals “View on page n” under it. The tablet uses the web layout.

**Artboards.** `R14-reader-tagged.dc.html`, `R14-reader-approximate.dc.html`, `R14-reader-phone.dc.html`, `R14-reader-approximate-phone.dc.html`.

**Todos.** [U4b](../../todos/ready/U4b-reader-mode-from-structure-tree.md), [U4c](../../todos/ready/U4c-approximate-reader-mode.md).

## R15 Offline, install and update

**Purpose.** Install lamina as an app, keep reading with no connection, and take updates only when the user agrees (C7b).

**Entry points.** Install: More › “Install lamina…” (only where the browser fires `beforeinstallprompt`), the home screen footer, palette `app.install`. lamina never opens the dialog unprompted. Offline: automatic on `offline`. Update: automatic when the new service worker is waiting.

**Layout and states.**

- **Install dialog** (R15-install). A §7.18 dialog over the scrim titled “Install lamina on this computer”. It lists three facts with icons: Open with › lamina, works with no connection, files stay on the device. A muted note says the browser asks to confirm. Actions: “Not now” and “Install” (primary, focused). Install calls the deferred prompt. If the user cancels the browser prompt, nothing happens and the menu item stays. Once installed, the menu item is removed. Safari and Firefox have no install prompt: there, the item reads “Add to Dock / Home Screen…” and opens a short how-to dialog with the browser’s own steps.
- **Offline notice** (R15-offline). A strip under the top bar (`surface`, hairline, offline icon, `role="status"`) reading “**You’re offline.** lamina keeps working: files on this device open, render and search as usual. Opening a file from a web address needs a connection.” It can be dismissed (focus ring drawn). The status bar adds “· Offline”. The notice clears itself on `online`, without a toast. *Open from URL while offline:* the field shows the error “You’re offline. Connect to open a file from a web address.”
- **Update ready** (R15-update). A toast reads “A new version of lamina is ready. Reload to use it; you’ll come back to page 3.” with “Reload” and “Later”. Unlike other toasts it stays until acted on, because it waits on the user’s consent (`skipWaiting` only on Reload). After “Later”, the status bar keeps a quiet item, “Update ready · Reload”, for the rest of the session. If the open file can’t be reopened automatically after reload (not Chromium, no OPFS copy), the toast says “Reload to use it; you’ll need to open this file again.”

**Behaviour.** No file changes. Reload restores the document and page through recent files or OPFS where possible. In the edit rounds, Reload is disabled with “Save or discard your changes first” while there are unsaved changes.

**Accessibility.** The install dialog is a modal dialog with a labelled title, and focus returns to the opener. The offline strip and the update toast are polite status messages (4.1.3). The toast doesn’t steal focus, and F6 / Tab reach it.

**Platform differences.** The desktop app has its own updater (S06), and the offline notice doesn’t apply to it. On the phone, the offline notice is a strip under the top bar, and the update toast sits above the bottom bar in place of the page pill.

**Artboards.** `R15-install.dc.html`, `R15-offline.dc.html`, `R15-update.dc.html`.

**Todos.** [C7b](../../todos/ready/C7b-offline-pwa.md).

## R16 Focus mode

**Purpose.** Hides the rail and the secondary toolbar so the page has the window. A selection brings the inspector back until it is deselected (plan §5.1, design language §8.3).

**Entry points.** `Ctrl \` / `⌘\`, palette `view.focus-mode`, View settings (not drawn as a control; the palette and the shortcut are its homes alongside View menu on desktop).

**Layout.** Top bar and status bar stay, because the status bar carries the network indicator and notices. The rail and the secondary toolbar (here Comment’s) are hidden. The mode switcher stays, so the active mode is still visible. The status bar gains a **Focus mode** item (focus icon, “Focus mode · leave with `Ctrl \`”) that leaves focus mode when pressed.

**States.** Drawn: Comment mode with a highlight selected. The inspector is docked as usual (colour, author, note, created, Reply / Delete highlight), the highlight has its 2 px focus outline, and the top bar shows “1 unsaved change” and Save. *Nothing selected* (not drawn): the canvas spans the full width between the top and status bars.

**Behaviour.** Selecting opens the inspector, and deselecting closes it while focus mode stays on. Escape closes the innermost layer first: mini toolbar, then dialog, then selection, then focus mode. Mode switches keep focus mode on. The setting is per window, not stored, so it is off on the next open.

**Accessibility.** The status item has `aria-pressed="true"` (focus ring drawn on it). Entering and leaving are announced (“Focus mode on. Rail and toolbar hidden.”). Hidden regions are removed from the tab order.

**Platform differences.** Desktop: also in View › Focus mode, and the window can additionally go full screen with the OS control. Phone: not applicable (the phone shell is already minimal). Tablet: the same, using the tablet top bar.

**Artboards.** `R16-focus-mode.dc.html`.

**Todos.** [U1c](../../todos/ready/U1c-inverted-l-shell-and-modes.md).

## R17 Tablet reading

**Purpose.** The desktop shell with touch rules at 1024 × 1366: 44 px targets, the pen treated separately from fingers, and the rail as a sheet in portrait.

**Entry points.** Any coarse-pointer device of 600 px width or more. Portrait starts with the rail collapsed.

**Layout.**

- **Top bar:** 56 px, with 44 × 44 icon buttons and 56 px mode buttons. Zoom controls are dropped in favour of pinch. The file name sits over the save state.
- **Comment toolbar:** 56 px, with 44 px tool toggles and colour swatches wrapped in 44 px buttons. The right end reads “Pen draws highlights · finger scrolls”.
- **Page:** fitted to the width (880 px).
- **Status bar:** 44 px.

**States.**

- *Comment mode, pen hovering* (R17-tablet). With a hover-capable pen (`pointerType: "pen"`, no buttons pressed), a 2 px accent caret and a 14 px ring show where the highlight would start. Nothing is drawn until the tip touches, and the preview disappears when the pen leaves. Fingers keep pan, pinch and double-tap. The focus ring is on the status-bar network item.
- *Read mode, rail as sheet* (R17-tablet-sheet). The sidebar button opens the rail as a bottom sheet titled “Contents”, with tabs Pages · Outline · Bookmarks. Outline rows are 48 px, and the current section shows the accent tint and a collapse button. The page stays visible above. In landscape (1366 wide) the rail docks at 260 px instead, as on desktop.
- *Text selected:* the bottom action bar (§7.15) appears and no mini toolbar; not drawn.

**Behaviour.** Tapping an outline row jumps to the target and closes the sheet. The grabber cycles medium → large. Swiping down or Escape closes the sheet.

**Accessibility.** Every target is at least 44 px. The sheet is a modal dialog with a focus trap. Pen hover is a preview only, so no information depends on it. Every drag has a tap alternative (the grabber button).

**Platform differences.** iPadOS: Apple Pencil hover needs M2-or-later iPads, and other pens skip the preview. Android tablets: stylus hover works where the device reports it. Both use the system selection handles.

**Artboards.** `R17-tablet.dc.html`, `R17-tablet-sheet.dc.html`.

**Todos.** [U3a](../../todos/ready/U3a-responsive-mobile-and-touch.md).

## R18 Phone: outline, find, more

**Purpose.** The phone versions of the rail, find and the overflow menu. The phone view settings sheet is [R10](#r10-view-settings) (`R10-view-settings-phone.dc.html`).

**Entry points.** Bottom bar Pages opens Contents (last tab remembered), Search opens find, and More opens the More sheet. The top-bar overflow opens the same More sheet.

**Layout and states.**

- **Outline sheet** (R18-outline-phone). A medium-detent sheet titled “Contents” (with “24 pages”) and three 44 px tabs: Pages (F04’s grid), Outline, Bookmarks. Outline rows are 48 px, Newsreader 17, page numbers right-aligned. The current row uses the accent tint with `aria-current="location"`, and a 44 px chevron button expands or collapses each level (focus ring drawn). The Pages button in the bottom bar is active. Empty outline: “This document has no outline” / “Use Pages to move around.”
- **Find** (R18-find-phone):
  - *Find bar:* the top bar becomes a find bar. A 44 px field (16 px text, focused) holds “backlog” and its count “3 of 12”, with a clear button and a “Done” text button.
  - *Options:* below it, 44 px toggles Match case and Whole words (pressed).
  - *Matches on the page:* all matches use `selection-tint` with a 1 px accent underline, and the current match is accent with white text.
  - *Results bar:* the bottom bar becomes a results bar reading “3 of 12 matches · Page 3 · Repairs backlog”, with “All results” (opens the results list as a sheet) and 44 px Previous / Next.
  - *Other states:* no match shows “No matches” in the count plus the help line “Check the spelling or turn off Whole words”. On a scanned file with no text it says “This page has no text to search.”
- **More** (R18-more-phone). A medium-detent sheet with a `role="menu"` of 52 px rows:
  - View settings (current page appearance shown on the right)
  - **Network activity** (shield in accent, subline “0 requests · nothing uploaded”)
  - Document properties
  - Print…
  - Download a copy
  - Add to home screen…
  - Privacy and help

  The focus ring is drawn on the first row. More is active in the bottom bar.

**Behaviour.** Sheets keep the page visible above the medium detent. Escape, the close button, swiping down or tapping the active bottom-bar button closes a sheet. Find’s Done restores the top and bottom bars and clears the highlights but keeps the query for next time.

**Accessibility.** Sheets are modal dialogs with labelled titles and a focus trap, and the grabber is a button (2.5.7). The find count lives in the field’s description and in a polite live region. Matches are told apart by lightness and outline as well as hue. Menu rows have visible labels, so no extra names are needed.

**Platform differences.** iOS Safari: the find field uses 16 px text so the page doesn’t zoom, and the OS keyboard covers the results bar, which then rides above the keyboard. Android: the system back gesture closes the sheet or find first.

**Artboards.** `R18-outline-phone.dc.html`, `R18-find-phone.dc.html`, `R18-more-phone.dc.html` (+ `R10-view-settings-phone.dc.html`).

**Todos.** [U3a](../../todos/ready/U3a-responsive-mobile-and-touch.md).

## New components

| Component | Spec | Used in |
| --- | --- | --- |
| **View settings popover** | Non-modal `role="dialog"`, 360 px, `surface`, `chip-border`, radius 6, menu shadow, padding `16px 16px 14px`, gap 18. Sections use label 12/600 `text-3` over the control and help text 12 `muted`. Footer over a hairline | R10 |
| **Segmented radio group** | `role="radiogroup"` grid of equal buttons, 28 px (desktop) or 44 px (touch), gap 4, radius 4 (6 on touch). Off: white + `control-border`. On: `accent` border, `accent-tint`, `accent` 600 | R10, R14 |
| **Stepper** | Two 32 px (44–56 px touch) secondary buttons, “Smaller/Larger text” (glyph `A` at two sizes), around an `<output aria-live>` value | R14 |
| **Docked utility panel** | Inspector anatomy (§7.11) used for a panel that isn’t a selection: 288–320 px, opened by a toggle with `aria-expanded`. It replaces the inspector while open | R13, R14 |
| **Data table in a panel** | 13 px tabular, header row 12 `muted` over a hairline, rows 6 px padding over hairlines, visually hidden `<caption>` | R13 |
| **Status strip** | Full-width notice under the top bar: 36 px min, `surface` (or a status tint), hairline bottom, 16 px icon, bold lead word, dismiss button. Use it for lasting app states (offline); signature risk uses the §7.13 banner | R15 |
| **Persistent toast** | §7.20 toast with two text actions that stays until acted on. Only for something that waits on the user’s consent (update ready) | R15 |
| **Status-bar mode item** | Status item that shows an active view state and leaves it on press, with an inline shortcut chip | R16 |
| **Pen hover preview** | 2 px `accent` caret plus a 14 px ring (2 px `accent` over 60% white). `aria-hidden`, shown only for a hovering pen | R17 |
| **Find match marks** | All matches: `selection-tint` fill + 1 px `accent` underline. Current match: `accent` fill, white text, 2 px outline, `aria-current`. If R05 (Reader A) defines match marks differently, R05 wins | R18 |
| **Find bar (phone)** | Replaces the top bar: 60 px row with a 44 px field (count inside, clear button) and a “Done” text button, plus an options row of 44 px toggles. The bottom bar becomes a results toolbar with a count, “All results” and 44 px Previous/Next | R18 |
| **Menu sheet (phone)** | Sheet holding a `role="menu"` list of 52–56 px rows: 22 px icon, 16 px label, optional 13 px `muted` subline or value on the right | R18 |

New icons (24 grid, stroke 1.5), proposed for design-language §5:

| Name | Markup |
| --- | --- |
| View settings | `<circle cx="12" cy="12" r="7.5"></circle><path d="M12 4.5v15M12 8.5h5.5M12 12h7.5M12 15.5h5.5"></path>` |
| Single page | `<rect x="7" y="4.5" width="10" height="15" rx="1"></rect>` |
| Two pages | `<rect x="3.5" y="5.5" width="8" height="13" rx="1"></rect><rect x="12.5" y="5.5" width="8" height="13" rx="1"></rect>` |
| Page (view on page) | `<path d="M6.5 3.5h7l4 4v13h-11z"></path><path d="M13.5 3.5v4h4"></path>` |
| Offline | `<path d="M7 17.5h10a3.5 3.5 0 0 0 .5-7 5.5 5.5 0 0 0-10.6-1A4 4 0 0 0 7 17.5z"></path><path d="M4 4l16 16"></path>` |
| Reload / update | `<path d="M19 12a7 7 0 1 1-2.05-4.95"></path><path d="M19 4.5v4h-4"></path>` |
| Focus mode | `<path d="M4.5 9V4.5H9M15 4.5h4.5V9M19.5 15v4.5H15M9 19.5H4.5V15"></path>` |
| Download | `<path d="M12 4.5V15M8 11l4 4 4-4"></path><path d="M5 19.5h14"></path>` |
| Print | `<path d="M7 9V4.5h10V9"></path><rect x="4.5" y="9" width="15" height="7.5" rx="1"></rect><path d="M7 14h10v5.5H7z"></path>` |
| Properties (info) | `<circle cx="12" cy="12" r="8"></circle><path d="M12 11v5M12 8v.5"></path>` |
| Chevron up | `<path d="M7 14l5-5 5 5"></path>` |
| Add to home screen | `<rect x="7" y="3.5" width="10" height="17" rx="2"></rect><path d="M12 7.5v6M9.5 11l2.5 2.5 2.5-2.5"></path>` |
