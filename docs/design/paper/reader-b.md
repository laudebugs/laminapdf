# Paper: Reader B

View settings and page appearance, the dark UI theme, keyboard shortcut help, the network indicator panel, reader mode, offline and install, focus mode, tablet reading and the phone sheets: screens R10–R18 of the [screen inventory](screens.md#r-reader-web-reader-v01-and-the-shared-viewer), drawn in the [Paper design language](design-language.md).

- **Canvas:** [lamina Paper — Reader B](https://claude.ai/artifact/KaCtzmB4hNEHgyJJQv2HBi) (one row per screen, a sticky note on the left of each row).
- **Platform:** web boards are drawn as the web app on Windows (shortcut chips use `Ctrl`). macOS uses the glyphs (`⌘K`, `⌘\`).
- **Sample content:** Q3-board-report.pdf, page 3 “2. Repairs backlog”. Page 4 (a table), section 3 text (“Rent collected in the quarter came to 98.1% …”), the open times and the request sizes are invented within the sample’s world. A file opened from a web address comes from `files.example.org`, as in Reader A.
- **Top bar.** The web boards R10–R15 use the reader v0.1 top bar ([§7.1](design-language.md#71-top-bar), Reader A’s): sidebar toggle and file name on the left; Back · Forward (disabled, no history yet) · page `3 / 24` · zoom · Find · View (`view.settings`) · Reader (`view.reader-mode`) · Commands `Ctrl K` · More on the right. No mode switcher and no save state. R16 and R17 show the editor top bar, because an editor mode is the point of those boards.
- **Rail tabs.** `Pages · Outline · Attachments` at 220 px ([§8.10](design-language.md#810-rail-tab-set-proposed)); the sample file has attachments, as in Reader A. There is no user Bookmarks tab.
- **Network indicator.** “0 network requests for this document · Offline-safe: nothing uploaded”; for a remote file “5 network requests for this document · to files.example.org only” ([§7.8](design-language.md#78-status-bar-items)).

## R10 View settings

**Purpose.** Set how the document looks in one place: the UI theme, the accent, the page appearance, the page layout and the zoom.

**Entry points.** On web, the *View* icon button in the top bar (`aria-label="View settings"`, `aria-haspopup="dialog"`), between Find and Reader. On the phone, More › View settings. Palette commands: `view.settings`, `view.theme` (`light`, `dark`, `high-contrast`, `system`), `view.accent` (`teal`, `blue`, `plum`, `green`), `view.page-appearance` (`original`, `sepia`, `dark`, `high-contrast`), `view.page-layout` (`single`, `two-up`), `view.continuous`, `view.fit-width`, `view.fit-page`, `view.actual-size`. None of these has a shortcut; zoom keys are listed in R12.

**Layout (web).** A popover under the button, 360 px wide: `surface`, `chip-border` edge, radius 6, menu shadow, z 30. It is a non-modal `role="dialog"` titled “View” (Newsreader 18). It sits over the top-right of the canvas and never covers the top bar. Sections, top to bottom:

| Section | Control | Notes |
| --- | --- | --- |
| Theme | Radio group in a 2 × 2 grid of 32 px buttons: Light · Dark · High contrast · Match system ([§8.6](design-language.md#86-page-appearance-vs-ui-theme)) | Help line names what “Match system” currently resolves to (“Following your system: light”) |
| Accent | Radio group of four 32 px swatches: Teal · Blue · Plum · Green ([§2.8](design-language.md#28-accent-colours)), the selected one with the §7.12 ring, its name beside the row | Help: “Links, selection and focus. Signed in, Theme and Accent follow your account to every device.” In the High contrast theme the row is disabled with “High contrast uses your system colours” (not drawn) |
| Page appearance | 4 compact appearance chips in a 2 × 2 grid (40 × 32 preview left of a 13 px label, padding 6, gap 10) + radio “This document / All documents” | Help: “Photos and annotation colours stay as they are.” |
| Page layout | Radio Single page · Two pages (icons) + switch “Continuous scrolling” | Two pages shows facing pages; page 1 alone on the right, as books do (setting in the doc only, not drawn) |
| Zoom | Radio chips Fit width · Fit page · 75% · 100% · 150% · 200% | When the zoom matches no preset (e.g. 113% after pinching), none of the chips is selected |
| Footer | “Page settings stay on this device” · ghost “More settings…” | Opens C7a’s settings dialog (Appearance and accessibility section: reduced motion and the other options) |

**States.** The board shows Sepia selected for this document, with the page and thumbnails recoloured in the Sepia page content colours ([§2.4](design-language.md#24-page-appearance)). *R10-page-dark* shows the result of choosing Dark: page text and vector paths are recoloured with the Dark page content colours (heading `#F2EEE7`, muted `#ADA597`, bars `#A9B4BC`, chart labels `#C9C2B6`), the yellow highlight keeps its colour, the thumbnails follow the page, and a toast reads “Page appearance: Dark, for this document only · Undo”. In High contrast the page uses `forced-colors` system colours. When `forced-colors: active` is detected the page appearance switches to High contrast unless the user chose something else (U2c). If a page draws its text as images in Dark, a notice on that page says “Text on this page is part of an image, so it can’t be recoloured.” (not drawn).

**Behaviour.** Every control applies at once (live preview). Nothing in the file changes: these are view preferences. Page appearance, layout and zoom are stored in `localStorage` (wrapped in try/catch) per document or globally. Theme and Accent are app-wide: signed out they are stored on the device; signed in they are saved to the account, so every device that signs in gets the same look (§2.8, H02). The accent is applied before first paint and swaps CSS custom properties under `[data-accent]`, with no re-render. Undo is the toast, or choosing again. Changing the appearance re-renders only the visible tiles and keeps the scroll position (U2c). Escape or a click outside closes the popover. Focus returns to the View settings button.

**Accessibility.** The trigger has `aria-haspopup="dialog"`, `aria-expanded` and `aria-controls`, and shows the pressed fill while open. Each group is a `radiogroup` labelled by its section label; the accent swatches are `role="radio"` buttons named by their colour (“Plum”), with the help line as their description, so colour is never the only cue. Arrow keys move inside a group and Tab moves between groups. The appearance chips use `aria-pressed` and have a name as well as a colour. The board shows the focus ring on the selected Sepia chip.

**Platform differences.** On the phone (*R10-view-settings-phone*) it is a sheet at the medium detent, opened from More (More shows as active). Page appearance comes first, because it is what people change while reading. The appearance chips are the compact 2 × 2 chips (§7.25), the theme radios are 44 px in the same 2 × 2 grid, the accent swatches sit in 44 px targets under Theme, zoom offers Fit width and Fit page only (pinch does the rest), and there is no two-up layout. The desktop app shows the same popover, and also puts these items in the native View menu (D4b).

**Artboards.** `R10-view-settings.dc.html`, `R10-page-dark.dc.html`, `R10-view-settings-phone.dc.html`. The `accent` tweak of each board is also the selected swatch of the Accent row. On the phone the Accent row is below Theme, under the medium detent's fold (the sheet scrolls).

**Todos.** [C7a](../../todos/done/C7a-accessibility-and-theming.md), [U2c](../../todos/ready/U2c-page-appearance-modes.md).

## R11 Dark UI theme

**Purpose.** The F01 reader in dark chrome, built from the [proposed dark tokens](design-language.md#23-dark-ui-theme). The page stays Original (white) unless the user picks a page appearance.

**Entry points.** View settings › Theme › Dark, or System when the OS is dark (`prefers-color-scheme`). Command `view.theme` (`dark`).

**Layout.** The shell is the same as F01. The chrome is `ground #1E1D1A`, the Commands button and fields use `surface #262421` / `field #1E1D1A`, and the gutter is `#141311`. Dividers are `hairline #3A3732`, and the page input border is `control-border #7E766A`. Text uses `#ECE7DE`, icons `#CFC8BC`, muted text `#A39B8E`. Accent, focus and the network shield are `#7DBDC2`. The page shadow is `shadow-page-dark`. The selection mini toolbar is `raised #2E2C28` with a `chip-border #4A463F` edge and `shadow-float-dark`. Thumbnails use a proposed dark thumbnail shadow (design language [§2.3](design-language.md#23-dark-ui-theme)). The selection on the white page keeps the Original tint `#CEDBDD`, because the selection tint follows the page appearance, not the UI theme.

**Content and controls.** Reader v0.1 top bar in dark tokens (Sidebar `view.toggle-sidebar`, Back/Forward `nav.back`/`nav.forward` disabled, page input, zoom `view.zoom-out`/`view.zoom-in`, Find `find.open`, View `view.settings`, Reader `view.reader-mode`, Commands `palette.open`, More). Rail: Pages · Outline · Attachments (`view.show-pages`, `view.show-outline`, `view.show-attachments`), thumbnails with page 3 current. Selection mini toolbar: Copy (`edit.copy`), Highlight (`comment.highlight`, switches to Comment, §8.11), Comment. Status bar: network indicator (`privacy.network-activity`), the repaired-file notice, as on F01, and page label.

**States.** The board shows text selected with the mini toolbar, and the focus ring on Highlight. The `accent` tweak previews all four accents with their dark values from [§2.8](design-language.md#28-accent-colours) (dark `accent`/`focus` for the ring, the tab underline, the current thumbnail and the shield; the Original-page selection is that accent at 22% over white, the Dark-page selection is the §2.8 dark-page value). The `pageAppearance` tweak switches the page to Dark, drawn exactly as R10-page-dark (heading and figure caption `#F2EEE7`, chart labels `#C9C2B6`, selection tint `#425858`, thumbnails dark). High contrast UI is not drawn: it follows `forced-colors` system colours (plan §5.5).

**Behaviour.** Switching theme only swaps CSS custom properties: no re-render, no layout change. With reduced motion there is no transition.

**Accessibility.** Every pair meets the ratios in the dark token table: muted on ground 6.13, accent on ground 7.97, and controls bordered with `control-border` (≥ 3.11). `color-scheme: dark` makes native controls and scrollbars dark. The hover fill is `rgba(236,231,222,0.08)`.

**Platform differences.** Desktop: the window frame follows the OS appearance. Phone: the same tokens apply to the bars and sheets (not drawn).

**Artboards.** `R11-dark-theme.dc.html`.

**Todos.** [C7a](../../todos/done/C7a-accessibility-and-theming.md).

## R12 Keyboard shortcuts

**Purpose.** Lists every shortcut for the active platform. The list is generated from the command registry (`useShortcutMap()`), never written by hand.

**Entry points.** `?` (page focus), palette command `help.shortcuts`, More › Keyboard shortcuts. The desktop app adds F1 and Help › Keyboard shortcuts. F1 is not bound on the web, where browsers use it for their own help.

**Layout.** A dialog over the dimmed shell (`scrim`), 820 px wide (§7.18) because the shortcuts sit in two columns. Its parts:

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
| `file.print` | Print | Ctrl P |
| `view.focus-mode` | Focus mode | Ctrl \ |
| `layer.close` | Close the open panel or dialog | Esc |
| `nav.next-page` / `nav.previous-page` | Next / previous page | Page Down / Page Up |
| `nav.first-page` / `nav.last-page` | First / last page | Home / End |
| `find.open` | Find in document | Ctrl F |
| `find.next` / `find.previous` | Next / previous match | Enter, F3 / Shift Enter, Shift F3 (in the find field) |
| `view.zoom-in` / `view.zoom-out` / `view.actual-size` | Zoom in / out / actual size | + / − / 0 (single keys, page focus; Ctrl + and Ctrl − stay the browser’s) |
| `view.reader-mode` | Reader mode | R (single key, Read mode only) |
| `edit.copy` | Copy | Ctrl C |
| `selection.actions` | Show selection actions | Shift F10 |
| `comment.highlight` / `annot.delete` | Highlight / delete highlight (Comment mode) | H / Backspace, Delete (⌫ on macOS) |

`file.save-copy` (Download a copy) has no shortcut, so it isn’t listed; `Ctrl S` is Save, in edit sessions only ([§8.4](design-language.md#84-explicit-save)).

`R` is scoped: Reader mode with the page focused in Read mode only; in Comment it is Rectangle and in the Organise grid Rotate right ([§8.8](design-language.md#88-keyboard-and-shortcuts)). The collision test covers the three uses.

**States.** The full list is drawn, with the search field focused. *Filtered:* typing matches names and keywords, empty groups hide, and the match is set in 600. *No match:* “No shortcut matches ‘…’. Try the command palette.” plus a ghost “Open command palette” button. With single-key shortcuts off, their rows stay listed but are labelled “Off”, and the chips are drawn in the disabled colour.

**Behaviour.** Read-only. Escape closes the dialog and focus returns to the opener.

**Accessibility.** `role="dialog"`, `aria-modal`, labelled by the title and described by the subline. The list is real `<dl>`/`<kbd>`, and the search field filters live, announcing the count (“6 shortcuts”) politely.

**Platform differences.** macOS shows glyph chips. Desktop adds F1, Back / Forward (`nav.back` / `nav.forward`: Alt ← / Alt →, ⌘[ / ⌘]; on the web these are the browser’s own, link jumps are pushed to history, C6b) and F6 / Shift F6 (region cycling); F6 is desktop only ([§8.8](design-language.md#88-keyboard-and-shortcuts)), and the web’s candidate `Ctrl F6` is not listed until the collision test passes it. Phone and tablet without a keyboard don’t list it; tablet with a hardware keyboard shows it from More.

**Artboards.** `R12-shortcut-help.dc.html`.

**Todos.** [U1b](../../todos/done/U1b-command-registry-and-palette.md), [C7a](../../todos/done/C7a-accessibility-and-theming.md).

## R13 Network activity panel

**Purpose.** Proves what left the device: every request since the document opened, grouped by origin and kind, and what the count proves (principle 7).

**Entry points.** The status-bar network indicator (a button; it gets the pressed fill and `aria-expanded="true"` while open). Palette `privacy.network-activity`. On the phone: More › Network activity, which shows the count as a subline.

**Layout.** A docked panel (`<aside aria-label="Network activity">`, the 360 px wide docked panel of [§6.1](design-language.md#61-desktop-and-web-shell-inverted-l), `surface`). It pushes the canvas and never floats. While the panel is open, a selection opens the inspector in its place. Sections:

1. Header: “Network activity”, “Since you opened this file at 09:12”, close button.
2. Summary: the count in Newsreader 40 + “document requests”, one sentence of fact, and a badge.
3. **Document traffic:** a table of origin · requests · received bytes, with a purpose line per origin.
4. **App files** (“Not document traffic”): same-origin app assets (renderer WASM, scripts and styles, fonts). They are listed separately, and collapsed when there is document traffic. One classifier decides which group a request falls in (U4a).
5. **Counted from:** page, document worker, offline service worker. A feed that doesn’t exist yet reads “Offline service worker · not installed” with no check.
6. **What this proves:** two short paragraphs, including the limit (browser extensions aren’t visible).
7. Actions: “Copy report” (copies a plain-text summary) and the ghost “How to check it yourself”, which opens the privacy page section on developer tools.

**States.**

- *Local file* (R13-network-panel): 0 document requests, “No part of this file has left your device.”, positive badge “Nothing uploaded”.
- *File from a URL* (R13-network-remote): 5 range requests to `files.example.org`, accent badge “The file’s own host only”. The status bar reads “5 network requests for this document · to files.example.org only”.
- *Unexpected origin* (not drawn): any document request to an origin other than the file’s host turns the shield and count `warning`. A warning-tint notice heads the panel: “1 request went to an unexpected site: example-cdn.net. lamina didn’t make it on purpose; tell us.” The row gets a warning badge.
- *Blocked by policy* (not drawn): a CSP violation is listed as a row with a neutral badge “Blocked”. It doesn’t count as traffic, because nothing was sent.

**Content and controls.** Close (`layer.close`), “Copy report” (`privacy.copy-report`), “How to check it yourself” (`privacy.show-how-to-check`, opens the privacy page section), the App files disclosure (a button with `aria-expanded`, no command). The status-bar item is `privacy.network-activity` and shows the pressed state (`accent-tint` fill, 1 px `accent` border) while the panel is open.

**Behaviour.** Live: the counts update as requests happen, and the summary is announced politely at most once every 5 s. Read-only, so nothing in the file changes.

**Accessibility.** Real `<table>`s with captions and `scope` headers. The status item is keyboard-reachable and announced (“0 network requests for this document, offline-safe: nothing uploaded, button, collapsed”). The board shows the focus ring on “How to check it yourself” (local) and on Close (remote).

**Platform differences.** The desktop app shows the indicator only if something networks (updates): U4a scope. Phone: the More row opens this panel as a sheet at the large detent.

**As built (U4a).** Three differences from the boards. “How to check it yourself” opens its steps in the panel, since the privacy page it should open comes with the launch (C7c). The status item is a toggle: it opens the panel and closes it, and its pressed state is `aria-expanded`. The phone’s More › Network activity row waits for its sheet; the palette’s `privacy.network-activity` reaches the panel there.

**Artboards.** `R13-network-panel.dc.html`, `R13-network-remote.dc.html`.

**Todos.** [U4a](../../todos/done/U4a-network-indicator-and-privacy-proof.md).

## R14 Reader mode

**Purpose.** Reflowed, adjustable text derived from the structure tree (tier 1). For untagged files the text comes from layout heuristics instead, labelled **Approximate** (tier 2). Everything runs on the device, with no page cap.

**Entry points.** The *Reader* toggle in the top bar (pressed while active; the zoom controls hide), the phone bottom-bar Reader button, palette `view.reader-mode`, and the single key `R`.

**Layout (web).** The canvas becomes a reading column (`max-width` from Line length, Source Serif 4 at 19 px / 1.65) on the chosen appearance’s paper. The top of the column holds a secondary “Back to page 3” button and the source badge (accent “From the document’s tags”, or warning “Approximate”). The Reader settings panel is docked in the inspector slot (288 px); its subline says “Text for every document · Appearance for this one”. While Reader mode is on, the top bar’s View button becomes the Reader settings toggle (same place and icon, `aria-label="Reader settings"`, `aria-expanded`, pressed fill while open), so the top bar keeps one settings button. The status bar shows “Reader mode · from tags / approximate”, and its page label follows the scroll position.

| Setting | Control | Values | Command id |
| --- | --- | --- | --- |
| Text size | Stepper: “Smaller text” · output · “Larger text” | 14–32 px, steps of 1 (output is `aria-live`) | `view.reader-text-smaller`, `view.reader-text-larger` |
| Line spacing | Radio | Tight · Normal · Loose | `view.reader-line-spacing` (value) |
| Line length | Radio | Narrow · Medium · Wide (about 55 / 68 / 85 characters) | `view.reader-line-length` (value) |
| Appearance | Chips | Original · Sepia · Dark (via CSS; High contrast follows `forced-colors`) | `view.page-appearance` (value) |
| Reset to defaults | Ghost button | | `view.reader-reset` |

Text size, line spacing and line length are global and stored locally. **Appearance is the same setting as R10’s page appearance:** Reader mode starts from the document’s page appearance, and its chips change that setting (with the same “This document” scope as R10), so leaving Reader mode keeps the choice. U4b should say the same (left for the owner; this doc doesn’t edit todos).

Other controls: “Back to page 3” and Escape leave Reader mode (`view.reader-mode`); “View original page” on a block (`nav.view-original-page`); “Open page 4” on a page-image block (`nav.go-to-page`); “How Reader mode works” (`help.reader-mode`, opens the help page).

**Content rules.** Headings, lists, quotes and links come out as real HTML. Tables, figures and formulas are shown as page crops with their `/Alt` text, or with “The file gives no description of it” when there is none. Running headers and footers are dropped.

**States.**

- *Tagged* (R14-reader-tagged): settings panel open, focus on “Larger text”.
- *Approximate* (R14-reader-approximate):
  - A warning notice heads the column: “**Approximate layout.** This file has no tags, so lamina guessed the reading order from the page layout. Check figures and numbers against the page.” It offers “Back to page 3” and “How Reader mode works”.
  - Every block reveals **View original page** (U4c’s label; accessible name “View original page (page n)”) on hover or focus (drawn focused). It returns to the block’s bbox on the page and briefly outlines it.
  - A page below the confidence threshold appears as a **Page image** block with one sentence of reason and “Open page 4”.
- *Not available* (not drawn): when the sanity checks fail and no tier 2 exists, the Reader toggle stays enabled. Turning it on shows an empty state instead of the column: “Reader mode isn’t available for this document” / “Its structure doesn’t match its layout, so reflowed text would be in the wrong order.” with a primary “Back to page”.
- *Loading* (not drawn): the first blocks appear as skeleton lines, with no spinner.

**Behaviour.** Entering keeps the position, as the block nearest the viewport top. Leaving (toggle, “Back to page”, Escape) returns to the same page region (U4b acceptance). The file never changes. Find works on the reflowed text and highlights in the column.

**Accessibility.** The reader view is a `main` region with `lang` from `/Lang`. Text scales to 200% without horizontal scrolling. Crops are `role="img"` with the alt text as their label. The Approximate notice is `role="status"`, announced once on entry.

**Platform differences.** On the phone (*R14-reader-phone*), the top bar shows Back (“Back to page 3”), the title with “Reader · from the document’s tags”, and a Reader settings button. Reader is active in the bottom bar (accent tint), and text is 18–20 px. Settings open as a sheet at the medium detent (*R14-reader-approximate-phone*, which also shows the Approximate notice) with 44 px controls and no line-length control. On the phone, tapping a block reveals “View original page” under it. The tablet uses the web layout.

**Artboards.** `R14-reader-tagged.dc.html`, `R14-reader-approximate.dc.html`, `R14-reader-phone.dc.html`, `R14-reader-approximate-phone.dc.html`.

**Todos.** [U4b](../../todos/ready/U4b-reader-mode-from-structure-tree.md), [U4c](../../todos/ready/U4c-approximate-reader-mode.md).

## R15 Offline, install and update

**Purpose.** Install lamina as an app, keep reading with no connection, and take updates only when the user agrees (C7b).

**Entry points.** Install: More › “Install lamina…” (only where the browser fires `beforeinstallprompt`), the home screen footer, palette `app.install`. lamina never opens the dialog unprompted. Offline: automatic on `offline`. Update: automatic when the new service worker is waiting.

**Layout and states.**

- **Install dialog** (R15-install). A §7.18 dialog over the scrim titled “Install lamina on this computer”. It lists three facts with icons: Open with › lamina, works with no connection, files stay on the device. A muted note says the browser asks to confirm. Actions: “Not now” and “Install” (primary, focused). Install calls the deferred prompt. If the user cancels the browser prompt, nothing happens and the menu item stays. Once installed, the menu item is removed. The item is chosen by feature detection, not by browser name: with no `beforeinstallprompt` but a browser-level install path (Safari’s Add to Dock / Add to Home Screen), the item reads “Add to Dock / Home Screen…” and opens a short how-to dialog with the browser’s own steps; where there is no install path at all (desktop Firefox), the item is hidden.
- **Offline notice** (R15-offline). A strip under the top bar (`surface`, hairline, offline icon, `role="status"`) reading “**You’re offline.** lamina keeps working: files on this device open, render and search as usual. Opening a file from a web address needs a connection.” It can be dismissed (focus ring drawn). The status bar adds “· Offline” after the usual wording. The notice clears itself on `online`, without a toast. *Open from URL while offline:* the field shows the error “You’re offline. Connect to open a file from a web address.”
- **Update ready** (R15-update). A toast reads “A new version of lamina is ready. Reload to use it; you’ll come back to page 3.” with “Reload” and “Later”. Unlike other toasts it stays until acted on, because it waits on the user’s consent (`skipWaiting` only on Reload). While the update waits, the status bar carries the “Update ready” action item (§7.8: `accent-tint`, `accent` 600 label, Download icon), which opens a status popover with Reload; after “Later” the toast goes and the item stays for the rest of the session. If the open file can’t be reopened automatically after reload (not Chromium, no OPFS copy), the toast says “Reload to use it; you’ll need to open this file again.”

**Content and controls.** Install dialog: Close install dialog and “Not now” (`layer.close`), “Install” (`app.install`). Offline strip: “Dismiss offline notice” (`app.dismiss-offline`). Update toast: “Reload” (`app.reload-update`), “Later” (`app.dismiss-update`); the status item “Update ready” opens its popover, whose Reload is `app.reload-update`. None has a shortcut.

**Behaviour.** No file changes. Reload restores the document and page through recent files or OPFS where possible. In the edit rounds, Reload is disabled with “Save or discard your changes first” while there are unsaved changes.

**Accessibility.** The install dialog is a modal dialog with a labelled title, and focus returns to the opener. The offline strip and the update toast are polite status messages (4.1.3). The toast doesn’t steal focus, and Tab reaches it (F6 is desktop only).

**Platform differences.** The desktop app has its own updater (S06), and the offline notice doesn’t apply to it. On the phone, the offline notice is a strip under the top bar, and the update toast sits above the bottom bar in place of the page pill.

**As built ([C7b](../../todos/done/C7b-offline-pwa.md)).** The web reader ships the install dialog, the offline notice with Offline in the status bar, and the update toast with Reload and Later. Five parts differ from the boards:

- Install appears only where the browser offers one (`beforeinstallprompt`, Chromium). Safari’s Add to Dock how-to waits for a later todo, so Safari shows no item, like Firefox.
- With a document open, the toast says “you’ll need to open this file again” (reopening at the same page is [T5b](../../todos/ready/T5b-reopen-at-last-position.md)).
- The status bar’s Update ready reloads at once rather than opening a popover with Reload.
- Reload is never disabled, since the reader can’t change a file yet.
- Open from URL keeps its own error while offline; the field’s offline message comes with the URL form’s next change.

**Artboards.** `R15-install.dc.html`, `R15-offline.dc.html`, `R15-update.dc.html`.

**Todos.** [C7b](../../todos/done/C7b-offline-pwa.md).

## R16 Focus mode

**Purpose.** Hides the rail and the secondary toolbar so the page has the window. A selection brings the inspector back until it is deselected (plan §5.1, design language §8.3).

**Entry points.** `Ctrl \` / `⌘\`, palette `view.focus-mode`, View settings (not drawn as a control; the palette and the shortcut are its homes alongside View menu on desktop).

**Layout.** Top bar and status bar stay, because the status bar carries the network indicator and notices. The rail and the secondary toolbar (here Comment’s) are hidden. The mode switcher stays, so the active mode is still visible. The status bar gains a **Focus mode** item (focus icon, “Focus mode · leave with `Ctrl \`”) that leaves focus mode when pressed.

**Content and controls.** The editor top bar ([§7.1](design-language.md#71-top-bar)) in Comment mode: sidebar toggle unpressed, “1 unsaved change”, divider, Undo (“Undo add highlight”, `edit.undo`) and Redo (disabled, `edit.redo`), mode switcher, page 3 / 24, zoom 125%, Find, View (`view.settings`), Commands `Ctrl K`, Save, More. Canvas with page 3 at the wider fit the hidden rail frees. Docked inspector: Highlight, the quote, colour swatches, author R. Osei, note “Ask for the 90-day list by region.”, created time, Reply and Delete highlight (`comment.reply`, `annot.delete`). Status bar: network indicator (`privacy.network-activity`), the **Focus mode** item (`view.focus-mode`, chip `Ctrl \`), page label. A visually hidden polite live region carries the on/off announcement.

**States.** Drawn: Comment mode with a highlight selected. The inspector is docked as usual (colour, author, note, created, Reply / Delete highlight), the highlight has its 2 px focus outline, and the top bar shows “1 unsaved change” and Save. *Nothing selected* (not drawn): the canvas spans the full width between the top and status bars.

**Behaviour.** Selecting opens the inspector, and deselecting closes it while focus mode stays on. Escape closes the innermost layer first: mini toolbar, then dialog, then selection, then focus mode. Mode switches keep focus mode on. The setting is per window, not stored, so it is off on the next open.

**Accessibility.** The status item is a toggle named “Focus mode” with `aria-pressed="true"` and `aria-keyshortcuts="Control+Backslash"` (focus ring drawn on it); the state is announced once, by `aria-pressed`. Entering and leaving are announced (“Focus mode on. Rail and toolbar hidden.”). Hidden regions are removed from the tab order.

**Platform differences.** Desktop: also in View › Focus mode, and the window can additionally go full screen with the OS control. Phone: not applicable (the phone shell is already minimal). Tablet: the same, using the tablet top bar.

**Artboards.** `R16-focus-mode.dc.html` (prop `showInspector` off gives the nothing-selected state).

**As built ([U1c](../../todos/done/U1c-inverted-l-shell-and-modes.md)).** As drawn, with one difference: the status item reads "Focus mode" followed by its shortcut chip, and the words "leave with" are left out, since pressing the item is what leaves. The announcement says "Sidebar", not the board's "Rail", because the web reader calls it the sidebar everywhere else.

**Todos.** [U1c](../../todos/done/U1c-inverted-l-shell-and-modes.md).

## R17 Tablet reading

**Purpose.** The desktop shell with touch rules at 1024 × 1366: 44 px targets, the pen treated separately from fingers, and the rail as a sheet in portrait.

**Entry points.** Any coarse-pointer device of 600 px width or more. Portrait starts with the rail collapsed.

**Layout.** Both boards use the editor shell, because the tablet board shows Comment mode; the web reader v0.1 on a tablet uses the reader top bar with the same 44 px targets.

- **Top bar:** 56 px, with 44 × 44 icon buttons and 56 px mode buttons. Zoom controls are dropped in favour of pinch. The file name sits over the save state.
- **Comment toolbar:** the one tablet Comment toolbar of [Comment C09](comment-review.md#c09-tablet-stylus) and design language [§6.2](design-language.md#62-phone-shell-390--844-artboard): the C01 toolbar at touch size (56 px, 44 px toggles) until a pen is used, then the tool shelf ([§7.35](design-language.md#735-touch-bars-and-sheets-proposed)) for the rest of the session. A hovering pen counts as used, so this board shows the shelf (60 px): Pen · Highlighter (pressed) · Eraser · Lasso · Shapes · Text · Note │ colours │ widths │ “Draw with finger” switch (off).
- **Page:** fitted to the width (880 px).
- **Status bar:** 44 px.

**Content and controls.**

- *Top bar:* Back to files, Sidebar (`view.toggle-sidebar`, `aria-expanded`), file name over the save state, the five modes, Find (`find.open`), View settings (`view.settings`), Save (only with changes), More.
- *Tool shelf* (`role="toolbar"` “Pen tools”, as C09): Pen (`comment.pen`, alias of `comment.ink`), Highlighter (`comment.highlighter`, pressed), Eraser (`comment.eraser`), Lasso (`comment.lasso`), Shapes (`comment.shape`), Text (`comment.text-box`), Note (`comment.note`); the highlight colours in 44 px buttons (`role="group"` “Colour”, the set swaps with the tool, §2.7); widths; the “Draw with finger” switch.
- *Contents sheet:* grabber (`Resize sheet`), title “Contents” with “24 pages”, close, tabs Pages · Outline · Attachments (`view.show-pages`, `view.show-outline`, `view.show-attachments`), outline rows with expand/collapse buttons (`view.expand-outline`, `view.collapse-outline`) and links (`nav.go-to-outline-item`).
- *Status bar:* network indicator and page label as 44 px targets.

**States.**

- *Comment mode, pen hovering* (R17-tablet). With a hover-capable pen (`pointerType: "pen"`, no buttons pressed), a 2 px accent caret and a 14 px ring show where the highlight would start. Nothing is drawn until the tip touches, and the preview disappears when the pen leaves. Fingers keep pan, pinch and double-tap. The focus ring is on the status-bar network item.
- *Read mode, rail as sheet* (R17-tablet-sheet). The sidebar button opens the rail as a bottom sheet titled “Contents”, with tabs Pages · Outline · Attachments. Outline rows are 48 px, and the current section shows the accent tint and a collapse button. The page stays visible above. In landscape (1366 wide) the rail docks at 260 px instead, as on desktop.
- *Text selected:* the bottom action bar (§7.15) appears and no mini toolbar; not drawn.

**Behaviour.** Tapping an outline row jumps to the target and closes the sheet. The grabber cycles medium → large. Swiping down or Escape closes the sheet.

**Accessibility.** Every target is at least 44 px. The sheet is a modal dialog with a focus trap. Pen hover is a preview only, so no information depends on it. Every drag has a tap alternative (the grabber button).

**Platform differences.** The preview is feature-detected (a pen pointer reporting hover), never keyed to a device list: on iPadOS, Apple Pencil hover works only on the iPads and pencils that support it, and other pens skip the preview. Android tablets: stylus hover works where the device reports it. Both use the system selection handles.

**Artboards.** `R17-tablet.dc.html` (prop `penHover` toggles the preview), `R17-tablet-sheet.dc.html`. The outline’s second level under section 2 (“Open repairs by region” p. 3, “Oldest cases by region” p. 4) and the collapsed children of sections 3 and 4 are invented sample content.

**As built ([U3a](../../todos/done/U3a-responsive-mobile-and-touch.md)).** The web reader's shell on a tablet is the desktop one, with 44 px targets wherever the pointer is coarse. In portrait (834 px wide) the rail is the drawer of narrow windows rather than a sheet; landscape docks it. Pinch, pan and double-tap are the viewer's, but the zoom controls stay in the top bar. The pen's hover preview comes with Comment mode.

**Todos.** [U3a](../../todos/done/U3a-responsive-mobile-and-touch.md).

## R18 Phone: outline, find, more

**Purpose.** The phone versions of the rail, find and the overflow menu. The phone view settings sheet is [R10](#r10-view-settings) (`R10-view-settings-phone.dc.html`).

**Entry points.** Bottom bar Pages opens Contents (last tab remembered), Search opens find, and More opens the More sheet. The top-bar overflow opens the same More sheet.

**Layout and states.**

- **Outline sheet** (R18-outline-phone). A medium-detent sheet titled “Contents” (with “24 pages”) and three 44 px tabs: Pages (F04’s grid), Outline, Attachments. Outline rows are 48 px, Newsreader 17, page numbers right-aligned. The current row uses the accent tint with `aria-current="location"`, and a 44 px chevron button expands or collapses each level (focus ring drawn). The Pages button in the bottom bar is active. Empty outline: “This file has no outline” / “Use Pages to move through it.” (§7.21).
- **Find** (R18-find-phone). The phone find is [Reader A’s R05](reader-a.md#r05-find-in-document), the canonical one; this board is the same find with the Results sheet closed:
  - *Find bar:* the top bar becomes a 44 px search field (16 px text, focused) holding “backlog” and its count “3 of 9”, with **Done**.
  - *Matches on the page:* the find marks of [§2.6](design-language.md#26-find-marks): other matches `find-match` `#F6E3BC` with a 1 px `warning` bottom edge, the current match `find-current` `#EDBF6A` with a 2 px `accent` outline.
  - *Find toolbar:* the bottom bar becomes Previous · Results · Next (Results `aria-expanded="false"`). Match case and Whole words live in the Results sheet, as in R05.
  - *Other states:* no match (prop `noMatch`) shows “No matches” in the count, a strip “Check the spelling, or open Results and turn off Whole words.”, and Previous / Next disabled. On a scanned file with no text it says “This page has no text to search.” (not drawn).
- **More** (R18-more-phone). A medium-detent sheet with a `role="menu"` of 52 px rows:
  - View settings (current page appearance shown on the right)
  - **Network activity** (shield in accent, subline “0 network requests for this document”)
  - Document properties
  - Print…
  - Download a copy
  - Add to home screen…
  - Privacy and help

  The focus ring is drawn on the first row. More is active in the bottom bar. At the medium detent five rows show and the list scrolls to the last two (Add to home screen…, Privacy and help); the large detent shows all seven.

**Content and controls.** Contents: `view.show-pages`, `view.show-outline`, `view.show-attachments`, `view.expand-outline`, `view.collapse-outline`, `nav.go-to-outline-item`. Find: `find.open`, `find.previous`, `find.next`, `find.close` (Done); Results opens R05’s Results sheet, which holds `find.match-case` and `find.whole-words`. More: `view.settings`, `privacy.network-activity`, `file.properties`, `file.print`, `file.save-copy` (Download a copy), `app.install` (Add to home screen…), Privacy and help (opens the privacy page; no command id yet). None of these has a shortcut chip on the phone.

**Behaviour.** Sheets keep the page visible above the medium detent. Escape, the close button, swiping down or tapping the active bottom-bar button closes a sheet. Find’s Done restores the top and bottom bars and clears the highlights but keeps the query for next time.

**Accessibility.** Sheets are modal dialogs with labelled titles and a focus trap, and the grabber is a button (2.5.7). The find count lives in the field’s description and in a polite live region. The current match is told apart by its outline as well as its fill. Menu rows have visible labels, so no extra names are needed.

**Platform differences.** iOS Safari: the find field uses 16 px text so the page doesn’t zoom, and the find toolbar rides above the keyboard. Android: the system back gesture closes the sheet or find first.

**Artboards.** `R18-outline-phone.dc.html` (prop `emptyOutline` shows the empty state), `R18-find-phone.dc.html` (prop `noMatch` shows the no-match state with the help line), `R18-more-phone.dc.html` (page in Sepia, matching the View settings value) (+ `R10-view-settings-phone.dc.html`).

**As built ([U3a](../../todos/done/U3a-responsive-mobile-and-touch.md)).** The Contents sheet (Pages · Outline once C6a gives the rail its tabs) and the More sheet are as drawn, and the inspector is a sheet too. The page pill isn't built yet. The bottom bar shows Pages and More; Search joins it when find has a handler (C5b), and Annotate and Reader when their rounds land. The top bar keeps the file name and the overflow, with no Back, since the reader has no files view to go back to yet.

**Todos.** [U3a](../../todos/done/U3a-responsive-mobile-and-touch.md).

## New components

Merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29; see its decisions table.

## Proposed design-language additions

Merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29; see its decisions table.
