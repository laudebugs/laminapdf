# 0024 — App shell UX: a docked inverted-L layout, modes, one command registry, selection surfaces and page appearance

- Status: Accepted (2026-11-04, when U1a's PR merges)
- Date: 2026-11-04 (amended 2026-11-09: U2a's five selection commands in the appendix)

## Context

The reader (Round C), the desktop app (Round D) and the editor (Round E) share one UI, `@laminapdf/ui` over `@laminapdf/viewer` ([ADR 0009](0009-web-client-architecture.md)). Five todos build that UI's shell, from the top bar (C4a) and the command palette (U1b) to the design tokens (U6a) and native menus (D4b), and each needs the same answers before it starts. Changing those answers after the editor modes ship would mean redrawing every mode's toolbar, menu and phone layout.

The evidence for the answers is in the [product and UX research](../research/product/pdf-app-product-and-ux.md#docked-chrome-modes-and-a-palette-beat-floating-toolbars), and the [UI/UX plan §4–§5](../plans/ui-ux.md#4-principles) turns it into principles:

- **Floating chrome failed twice.** Figma's UI3 reattached its floating panels after user feedback, and Acrobat had to make its floating quick-tools widget movable because it covered page content.
- **Visible controls beat shortcuts.** About 80% of Office saves came from the toolbar icon and 20% from Ctrl+S, so a palette can't be the only path to an action.
- **The platform owns touch selection.** On iOS, scrolling hides the copy menu and code can't bring it back, so a toolbar drawn over it competes with something the app can't control.
- **CSS `invert()` makes photos into negatives,** so a dark page has to be recoloured in the renderer.

Three decisions already constrain this one. ADR 0009 puts the chrome in React 19 with React Aria Components, keeps the viewer framework-free, and holds the interaction state in `@laminapdf/ui`'s `ReaderController`. [ADR 0010](0010-platform-adapter.md) makes the UI branch on `Platform` capability flags, never on the platform, and hides a control that can never work while disabling one the user could unlock. The owner chose the **Paper** visual direction on 2026-10-29, and the [Paper design language](../design/paper/design-language.md) holds its values; this ADR fixes the rules those values plug into.

This ADR records the interaction model as seven binding decisions (§1–§7) and the accessibility rules that hold across them (§8), so every shell and editor round builds to one model.

## Decision

### 1. Layout: a docked inverted L

Every desktop and web window has the same five regions, after Linear's "inverted L" of a sidebar and a top header framing the main view:

```
┌───────────────────────────────────────────────────────────────────┐
│ top bar: file · mode switcher · page n / N · zoom · find · ⌘K · ⋯ │
├───────────────────────────────────────────────────────────────────┤
│ secondary toolbar (only in a mode other than Read, or while find) │
├────────┬─────────────────────────────────────────────┬────────────┤
│ left   │                                             │ inspector  │
│ rail   │            canvas (pages on gutter)         │ (only with │
│        │                                             │ a selection│
├────────┴─────────────────────────────────────────────┴────────────┤
│ status bar: network indicator · notes · page label                │
└───────────────────────────────────────────────────────────────────┘
```

| Region | Shows | Rule |
| --- | --- | --- |
| Top bar | Always | Slim: file actions, the mode switcher, the page indicator, zoom, find, the palette button, overflow |
| Secondary toolbar | In every mode but Read, and while a tool or find is open | One at a time (§2) |
| Left rail | By default at 1024 px and wider; collapsed below | Tabs (Pages, Outline, then contextual ones); width remembered locally |
| Inspector | Only while an annotation, field, page or object is selected | Docked on the right; pushes the canvas, never covers it |
| Status bar | Always | Network indicator first; at most one note |

- **Docked, not floating.** Only short-lived layers float over the page: the selection mini toolbar, menus, popovers, tooltips, the palette, dialogs and toasts. Panels never float and never cast shadows.
- **Stable.** Regions appear only under the rules in this table, so controls don't come and go with the pointer.
- **Focus mode** (`view.focus-mode`) hides the rail and the mode's secondary toolbar; an open find bar stays until it is closed. A selection brings the inspector back until it ends, as in Figma's Minimize UI. Focus mode is the user's choice to hide those homes, so §3's visible-home rule is checked in the normal layout. The same shortcut, or Escape, leaves focus mode.
- **Narrow windows.** Below 600 px the phone shell (§6) replaces this one. Between 600 and 1023 px the rail starts collapsed. A coarse pointer uses the touch rules at any width.

The design language's [§6.1](../design/paper/design-language.md#61-desktop-and-web-shell-inverted-l) gives the sizes; C4a builds the regions as slots, and U1c fills them.

### 2. Modes: each owns exactly one secondary toolbar

A mode is a task the user chose, and its toolbar is the only place its tools live. Read is the default and needs no toolbar.

| Mode | Command | Secondary toolbar | Ships in |
| --- | --- | --- | --- |
| Read | `mode.read` | None (the find bar while find is open) | Reader v0.1 |
| Comment | `mode.comment` | Annotation tools, colour, width | E4 |
| Fill & Sign | `mode.fill-sign` | Field navigation, fill tools, the two signing actions | E8, E10 |
| Organise | `mode.organise` | Page operations over the page grid | E2 |
| Edit | `mode.edit` | Content tools, and Redact, Protect and Form design as tools of Edit | E6, E7, E12 |

- **Registered, not hard-coded.** A mode is `{ id, title, icon, toolbar, enter(ctx), leave(ctx) }`, registered with `ReaderController` (ADR 0009 §1) by the round that adds it, so the reader ships the mode switcher with Read alone.
  - *As built ([U1c](../todos/done/U1c-inverted-l-shell-and-modes.md)):* modes register with `@laminapdf/ui`'s `ModeRegistry`, which sits beside the shell's layout store rather than in `ReaderController`. A mode is chrome; the controller holds the document's lifecycle and never draws a toolbar. `enter` and `leave` take no context, because each mode's closures already reach what it changes.
- **One secondary toolbar at a time.** A tool's toolbar or the find bar replaces the mode's toolbar while it is open, and its Done or Close brings the mode's back.
- **Read changes nothing.** No command in Read mode changes the file; a panel opened in Read offers the mode that edits what it shows ("Edit in Organise"). The selection mini toolbar's Highlight and Comment are the one exception: they create the annotation and switch to Comment.
- **Switching is explicit.** A mode changes through the mode switcher, the palette or a menu, never through a single-key shortcut.

### 3. One command registry

Every action the shell can take is a command in one registry in `@laminapdf/ui`. The palette, the toolbars, the context menus, the web overflow menu, the desktop's native menus and the shortcut help are views of that registry, so a label, shortcut or enabled state can't drift between them.

```ts
export interface Command<Args = void> {
  /** `namespace.verb-object`, kebab-case, one dot (`file.save-copy`); variants are args. */
  readonly id: CommandId;
  readonly title: string;
  /** Extra words the palette matches (`"rotate turn"`). */
  readonly keywords?: readonly string[];
  /** Menu and palette section (`file`, `view`, `navigate`, `find`, `help`, …). */
  readonly group: CommandGroup;
  /** Its key bindings, each in one scope; several when the design language lists several. */
  readonly shortcuts?: readonly Shortcut[];
  readonly icon?: IconName;
  /** A capability the command needs (ADR 0010): hidden or disabled when the flag is false. */
  readonly requires?: keyof Capabilities;
  /** `true`, or the reason it is disabled now ("Open a document first"). */
  enabled(ctx: CommandContext): true | string;
  /** Typed arguments: the palette parses them inline (`zoom 150`, `go to 12`). */
  readonly args?: ArgsSchema<Args>;
  run(ctx: CommandContext, args: Args): void | Promise<void>;
}

export interface Shortcut {
  /** Keys on macOS and elsewhere (`"Mod+K"`, `"?"`, `"F3"`); `Mod` is ⌘ or Ctrl. */
  readonly mac: string;
  readonly other: string;
  /** Where the keys act: everywhere, while the page has focus, in the find field, … */
  readonly scope: ShortcutScope;
  /** A capability the binding needs: `ownsKeyboard` for keys a browser keeps (⌘W, F1). */
  readonly requires?: keyof Capabilities;
}
```

- **Ids** follow the design language's rule ([§8.8](../design/paper/design-language.md#88-keyboard-and-shortcuts)): `namespace.verb-object`, one id per action across every surface, labels allowed to differ by platform (`file.save-copy` is "Download a copy" on the web and "Save a copy…" on the desktop). Telemetry counts (U5c) and the docs use the same ids.
- **Every command has a visible home besides the palette:** a toolbar button, a menu item or a context-menu item. The palette and shortcuts are accelerators; a test fails on a command that only the palette reaches.
- **Capabilities, per ADR 0010.** When a command's `requires` flag is false, the registry asks `Platform.howToEnable(flag)`. No answer means the platform can never have the capability, so every view leaves the command out. An answer means the user could unlock it (by installing the PWA, say), so the command shows disabled with that answer as its reason. A binding whose `requires` flag is false is simply not bound. Keys a browser keeps for itself need `ownsKeyboard`, a flag this ADR adds to ADR 0010 (`false` on the web, `true` in the desktop app), so no command checks the platform's name.
- **Shortcuts** are declared once, on the command, and a test checks them against each other and against the browser, OS and assistive-technology keys the design language lists. Single-key shortcuts work only while the page has focus and can be turned off in Settings.
- **The palette** (U1b) searches titles and keywords, lists recent commands first, shows each row's shortcut, and parses arguments inline. It follows the APG combobox pattern: focus stays in the input, and `aria-activedescendant` names the active row (§8).
- **Native menus** (D4b) are generated from the registry where `hasNativeMenus` is true; the web shows the same groups in its overflow menu.

The reader v0.1 command set is in the appendix, so U1b starts from it.

### 4. Selection surfaces follow the pointer

The viewer stays headless: it emits selection events with client rectangles and the selected text runs, and `@laminapdf/ui` draws every menu. Which surface appears depends on the pointer, never on the platform's name.

| Pointer | Text selected | Object selected |
| --- | --- | --- |
| Fine (mouse, trackpad, pen hovering) | A mini toolbar (`role="toolbar"`) above the first selected line, flipped below on collision. It appears on `pointerup`, never during a drag; Shift+F10 opens it and Escape closes it | The inspector (§1) |
| Coarse (touch) | A bottom action bar (Copy, Highlight, Comment, Share, More): it replaces the bottom bar in the phone shell and docks under the canvas in the desktop shell on a tablet. The OS selection menu is left alone | The inspector: docked in the desktop shell, as on a tablet; a sheet at its medium detent in the phone shell (§6) |

Every action in either surface also has a command (§3), so neither surface is ever the only path to it. Text selection never opens the inspector.

### 5. Page appearance is separate from the UI theme

Three settings recolour three different things, and none implies another:

- **Theme** (Light, Dark, High contrast, Match system) recolours the chrome.
- **Accent** (Teal, Blue, Plum, Green) recolours the chrome's accent and the text-selection tint.
- **Page appearance** recolours page content in the renderer:

| Page appearance | How | Never |
| --- | --- | --- |
| Original (default in every theme) | No change | — |
| Sepia | The renderer recolours paper and text | Raster images change |
| Dark | The renderer recolours the page's text and paths | CSS `invert()`, which turns photos into negatives |
| High contrast | The renderer maps page colours to the `forced-colors` system colours, as pdf.js does with `Canvas` and `CanvasText` | A hand-made palette |

Annotation colours never change in any appearance, because they carry meaning. The gutter around pages has its own token per theme. These rules fix the outcome, and U2c finds the mechanism in each backend (ADR 0004). PDFium's colour scheme recolours text and paths, and annotations too when they are drawn in the same pass, so annotations may need a pass of their own. The hayro backend has no such option today, so U2c either adds one or records the gap.

### 6. The phone shell: bars, sheets and viewer-owned gestures

Below 600 px wide the shell becomes a phone layout instead of a squeezed desktop one:

- **Top bar** with back, the title and overflow; it hides on scroll down and returns on scroll up or a tap.
- **Bottom bar** with five entries: Search, Annotate, Pages, Reader, More. The mode switcher moves into Annotate and More. The bar is a toolbar of actions with a More menu, as Apple's [toolbar guidance](https://developer.apple.com/design/human-interface-guidelines/toolbars) describes for iOS, not a tab bar, which Apple reserves for navigation ([Tab bars](https://developer.apple.com/design/human-interface-guidelines/tab-bars)).
- **Bottom sheets** replace panels and the inspector, with Apple's two detents: large, fully expanded, and medium, about half of that ([Sheets](https://developer.apple.com/design/human-interface-guidelines/sheets)). One sheet shows at a time. The grabber is a real button that cycles the detents, as both Apple's grabber and Material's drag handle do ([Bottom sheets](https://m3.material.io/components/bottom-sheets/guidelines)), and it is the single-pointer alternative to dragging a sheet.
- **Safe areas:** the page sets `viewport-fit=cover` and pads with `env(safe-area-inset-*)`, which iOS reports as zero otherwise.
- **Gestures belong to the viewer:** the canvas sets `touch-action: none`, and the viewer implements pan, pinch and double-tap zoom so browser zoom doesn't fight it. Long-press selects text. Drawing needs an explicit tool or a pen `pointerType`, so a one-finger pan never draws.
- **Panning has buttons too.** Because the viewer replaces the browser's own scrolling, SC 2.5.7 applies to panning (§8), so Next page, Previous page, Go to page and the zoom buttons stay one tap away.

Native mobile apps are out of scope here: ADR 0025, after the U3b spike, decides how they are built and inherits these rules.

### 7. Design tokens in three layers

`@laminapdf/tokens` (U6a) holds every visual value in three layers, and each layer may only read the one below:

| Layer | Holds | Read by |
| --- | --- | --- |
| Primitive | The raw palettes and scales: `color.warm.*`, `color.teal.*`, `space.*`, `radius.*`, `font.size.*` | Semantic tokens only |
| Semantic | The design language's names: `ground`, `surface`, `raised`, `field`, `gutter`, `text`, `text-2`, `muted`, `hairline`, `accent`, `focus`, `danger`, … | Components |
| Component | Only the values the design language fixes per component: bar heights, focus-ring width and offset, sheet radius, thumbnail sizes | That component |

- **Names.** Semantic and component tokens become CSS custom properties prefixed `--paper-` (`--paper-surface`, `--paper-focus-ring-width`), the prefix the docs site already uses. Primitives exist only as alias targets in `tokens.json`, so a component can't reach one.
- **Axes.** Theme and accent select values of the semantic layer (`[data-theme]`, `[data-accent]`); page appearance is a separate axis, set on the canvas (`[data-page-appearance]`) and read by the renderer as constants. The high-contrast theme maps semantic tokens to CSS system colours rather than defining a palette.
- **Values** come from the Paper design language. Where the design language and this ADR disagree on a name, the ADR wins and U6a corrects the design language.

### 8. Accessibility rules that hold across the shell

The shell targets WCAG 2.2 level AA. These rules come straight from the W3C texts and apply to every region, mode and surface above:

- **Targets:** every pointer target is at least 24 × 24 CSS px ([SC 2.5.8 Target Size (Minimum), AA](https://www.w3.org/TR/WCAG22/#target-size-minimum)), and the shell's own controls meet that size themselves rather than through the criterion's spacing exception. On touch, lamina adopts 44 × 44 CSS px, the size of [SC 2.5.5 Target Size (Enhanced), AAA](https://www.w3.org/TR/WCAG22/#target-size-enhanced) and Apple's default control size ([Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility)). Material asks for 48 × 48 dp ([Accessibility designing](https://m3.material.io/foundations/designing/structure)); native apps decide that in ADR 0025.
- **Dragging:** every drag has a single-pointer alternative ([SC 2.5.7 Dragging Movements, AA](https://www.w3.org/TR/WCAG22/#dragging-movements)): the sheet grabber button, "Move to page…" and move controls in Organise, inspector fields for page overlays, the page buttons for panning. A keyboard equivalent alone isn't enough: W3C's [Understanding document](https://www.w3.org/WAI/WCAG22/Understanding/dragging-movements.html) asks for controls a pointer can press.
- **Focus never hidden:** a focused control is never entirely hidden by the top bar, a toolbar, a sheet or a toast ([SC 2.4.11 Focus Not Obscured (Minimum), AA](https://www.w3.org/TR/WCAG22/#focus-not-obscured-minimum)). W3C's [Understanding document](https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum.html) names sticky headers, sticky footers and non-modal dialogs as the usual failures, so the canvas keeps scroll padding equal to the docked chrome and any open sheet. Toasts never cover a focusable control.
- **Single-key shortcuts** can be turned off and act only while the page has focus ([SC 2.1.4 Character Key Shortcuts, A](https://www.w3.org/TR/WCAG22/#character-key-shortcuts)).
- **Patterns** from the WAI-ARIA Authoring Practices (APG):
  - The palette follows the [combobox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/) with a listbox popup, as in APG's [list autocomplete example](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/examples/combobox-autocomplete-list/): DOM focus stays on the input, `aria-activedescendant` names the active row, and the script scrolls that row into view, which browsers don't do for an active descendant.
  - The mini toolbar and each secondary toolbar follow the [toolbar pattern](https://www.w3.org/WAI/ARIA/apg/patterns/toolbar/): one tab stop, arrow keys between controls, menu buttons allowed. A control that uses arrow keys itself, such as a text field, comes last, and a toolbar holds at most one.
  - The rail follows the [tabs pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/), and the Organise page grid is a layout grid in the [grid pattern](https://www.w3.org/WAI/ARIA/apg/patterns/grid/): one tab stop, arrow keys that may wrap, `aria-selected` on selected pages.
- **Primitives:** React Aria Components (ADR 0009) supplies `Toolbar`, `Tabs` and `GridList`. For the palette, U1b chooses between cmdk, which the UI/UX plan proposes, and React Aria's `ComboBox`, and checks the result against the APG combobox pattern (`role="combobox"`, `aria-expanded`, `aria-controls`, `aria-activedescendant`), adding what is missing.

## Consequences

- **One model to build against.** C4a, U1b, U1c, U6a and D4b can start in parallel, and each editor round adds a mode, its toolbar and its commands without touching the shell.
- **The registry costs some ceremony.** Even a one-off button needs a command, an id and a visible home. That cost buys generated menus, help and shortcut chips that can't drift, and a test that finds palette-only commands.
- **Page appearance needs renderer work** in both backends (U2c), and the two may differ. The nightly render diff (X3b) compares Original only today, so U2c adds the other appearances to it; Original stays the default.
- **Some layout values are still proposals** in the design language (the dark theme, the large detent, the breakpoints); U6a asks the owner to approve them before they become tokens.
- **Touch users get no floating toolbar,** so a selection action costs a tap on the bottom bar instead of one near the text. A floating toolbar would compete with the OS selection menu, which code can't control on iOS.
- **Read mode stays spare,** as the research recommends: the page, the top bar and the status bar, with tools appearing only when the user picks a mode, selects something or opens the palette.

## Alternatives considered

- **A ribbon.** It shows every tool at once and has the contextual tabs this ADR keeps. Rejected: it fills Read mode with editing tools, and the research's readers ask for less chrome. Modes keep the ribbon's good idea, one contextual toolbar.
- **Floating panels** (UI3's first release, Acrobat's quick-tools widget). Rejected: both vendors retreated from them after complaints that they covered the page.
- **Per-surface menus without a registry.** Simpler at first. Rejected: labels, shortcuts and enabled states drift between the palette, menus and toolbars, and native menus would need hand-written copies.
- **A palette as the main way in.** Rejected: most users act on visible controls, so the palette stays an accelerator over commands that already have a home.
- **A hard-coded mode list.** Rejected: the reader would ship empty modes, and every editor round would edit the shell. Registering a mode keeps Read alone in v0.1.
- **A floating selection toolbar on touch.** Rejected: it competes with the OS selection menu, which code can't control on iOS.
- **CSS `invert()` or a filter for dark pages.** Cheap. Rejected: it turns photos into negatives and shifts annotation colours.
- **Page appearance tied to the UI theme.** Rejected: many readers want dark chrome around original pages, and the high-contrast theme must follow system colours rather than a page palette.
- **A side drawer on phones.** Rejected: drawers hide navigation behind a tap, and the bottom bar and sheets keep the main tasks one tap away within thumb reach.
- **The AA minimum of 24 × 24 px on touch as well.** Rejected: Apple's minimum control size is 28 × 28 pt and its default 44 × 44 pt, so 24 px touch targets would be smaller than iOS's own floor.
- **Two token layers (primitive and semantic only).** Simpler. Rejected: component sizes such as bar heights and focus-ring offsets would be hard-coded in components, where U6f's visual tests can't see them change.

## Appendix: the reader v0.1 command set

U1b registered these 39 commands, U2a added the five below `selection.actions` (amended 2026-11-09), C7a added `view.settings` and `app.settings` (amended 2026-11-09), and C7b added `app.install` and `app.reload-update` from the design language's appendix (amended 2026-11-09), so the reader registers 48. Shortcuts follow the design language's table, and "page" means the page has focus. A "menu" is the command's group in the web's overflow menu and in the desktop app's menu bar. Three ids are new in this ADR and join the design language's appendix in this PR: `mode.comment` (§2), `view.set-zoom` (a percentage) and `view.rotate-display` (a quarter turn of the view, which never changes the file).

| Id | Title | Arguments | Shortcut (macOS / others) | Visible home |
| --- | --- | --- | --- | --- |
| `file.open` | Open a file | — | ⌘O / Ctrl O | Home, File menu |
| `file.open-url` | Open from URL | URL | — | Home |
| `file.open-recent` | Open recent | Recent entry | — | Home, file menu |
| `file.clear-recent` | Clear recent | — | — | Home |
| `file.unlock` | Unlock (password) | Password | — | Password prompt |
| `file.save-copy` | Download a copy / Save a copy… | — | — | File menu |
| `file.print` | Print | Page range | ⌘P / Ctrl P | File menu |
| `file.properties` | Document properties | — | — | File menu |
| `file.show-repairs` | Show repairs | — | — | Status-bar note |
| `nav.next-page` | Next page | — | Page Down (page) | Page indicator |
| `nav.previous-page` | Previous page | — | Page Up (page) | Page indicator |
| `nav.first-page` | First page | — | Home (page) | View menu |
| `nav.last-page` | Last page | — | End (page) | View menu |
| `nav.go-to-page` | Go to page… | Page label or number | — | Page indicator input |
| `nav.back` | Back | — | ⌘[ / Alt ← (desktop app) | View menu |
| `nav.forward` | Forward | — | ⌘] / Alt → (desktop app) | View menu |
| `view.zoom-in` | Zoom in | — | + (page); ⌘+ / Ctrl + (desktop app) | Zoom control |
| `view.zoom-out` | Zoom out | — | − (page); ⌘− / Ctrl − (desktop app) | Zoom control |
| `view.set-zoom` | Zoom to… | Percentage, 10–800 | — | Zoom control menu |
| `view.actual-size` | Actual size | — | 0 (page); ⌘0 / Ctrl 0 (desktop app) | Zoom control menu |
| `view.fit-width` | Fit width | — | — | Zoom control menu |
| `view.fit-page` | Fit page | — | — | Zoom control menu |
| `view.rotate-display` | Rotate view | `clockwise` or `counter-clockwise` | — | View menu |
| `view.page-layout` | Page layout | `single`, `two-up`, `continuous` | — | View settings |
| `view.page-appearance` | Page appearance | `original`, `sepia`, `dark`, `high-contrast` | — | View settings |
| `view.theme` | Theme | `light`, `dark`, `high-contrast`, `system` | — | View settings |
| `view.accent` | Accent colour | `teal`, `blue`, `plum`, `green` | — | View settings |
| `view.settings` | View settings | — | — | Top bar |
| `app.settings` | Settings | — | ⌘, / Ctrl , (desktop app) | View settings; Help menu |
| `app.install` | Install Lamina PDF… | — | — | More menu, while the browser offers an install |
| `app.reload-update` | Reload to update | — | — | Status bar's Update ready, while an update waits |
| `view.toggle-sidebar` | Show or hide sidebar | — | — | Top bar |
| `view.focus-mode` | Focus mode | — | ⌘\ / Ctrl \ | View menu |
| `find.open` | Find in document | — | ⌘F / Ctrl F | Top bar |
| `find.next` | Next match | — | ↵, F3 (find field) | Find bar |
| `find.previous` | Previous match | — | ⇧↵, ⇧F3 (find field) | Find bar |
| `edit.copy` | Copy | — | ⌘C / Ctrl C | Mini toolbar, context menu |
| `privacy.network-activity` | Network activity | — | — | Status bar |
| `palette.open` | Command palette | — | ⌘K / Ctrl K | Top bar |
| `help.shortcuts` | Keyboard shortcuts | — | ? (page); F1 (desktop app) | Help menu |
| `layer.close` | Close the open panel or dialog | — | Esc | Every layer's close button |
| `selection.actions` | Show selection actions | — | ⇧F10 / Shift F10 | Context menu |
| `edit.copy-exact` | Copy exact (the layout's line breaks kept) | — | — | Mini toolbar, context menu |
| `edit.select-column-mode` | Select by column (a drag selects a rectangle, as Alt-drag does) | — | — | Context menu |
| `selection.search` | Search for the selection | — | — | Mini toolbar, context menu |
| `selection.share` | Share the selection (where the device can share) | — | — | Context menu, the phone's selection bar |
| `selection.toggle-toolbar` | Show or hide the selection toolbar | — | — | View settings |
| `mode.read` | Read mode | — | — | Mode switcher |
