# Plan: motion and interaction states

- **Status:** Draft (the owner chose the cursor, hover motion, overlay motion and reduced-motion rules on 2026-11-14, §3)
- **Date:** 2026-11-14
- **Asked for by:** the project owner (2026-11-14): make the apps "delightful to use"; "there are buttons that don't have the right cursor when hovering"; "handle all the different states of buttons/links etc and handle the animations to/from hover states correctly".
- **Changes:** the Paper design language ([§7.0 states](../design/paper/design-language.md), the reduced-motion checklist), `@laminapdf/tokens`' `motion` group, every interactive component in `@laminapdf/ui`, the web reader, the desktop app, the docs site and the website
- **Builds on:** [ADR 0024](../decisions/0024-app-shell-ux.md) §7 (tokens in primitive, semantic and component layers), [ADR 0010](../decisions/0010-platform-adapter.md) (capability flags, no platform checks), [ADR 0011](../decisions/0011-cloudflare-topology-and-privacy.md) §4 (the security headers, which the sites' `_headers` follow: stylesheets only, no inline styles), Round U6's components and gallery ([Paper design §6](paper-design.md#6-round-u6-paper-in-code))
- **Research:** [motion and interaction states](../research/product/motion-and-states/README.md): four notes with a file and line for every claim, and a measurement of the built reader
- **Todos:** [§7](#7-todos), Round U7

## 1. Goal

Every control in Lamina PDF should say plainly that it can be used, and answer at once when it is. Today the hand never shows over a button, many controls have no pressed state, hover snaps in and out, and menus and dialogs pop into place. This plan sets one rule for cursors, one state model with an order of precedence, a small set of motion tokens, and one meaning for reduced motion. Nine pull requests then apply them to the reader, the desktop app, the docs site and the website.

Delight here means responsiveness first: a state that changes as the pointer arrives, an overlay that grows from what opened it, a panel that slides instead of jumping, a page that fades in instead of popping. It never means decoration on a reading surface, and nothing a person does many times a minute (the palette, find next, zoom steps) waits for an animation.

## 2. What the apps do today (2026-11-14)

This section summarises the [research notes](../research/product/motion-and-states/README.md); each claim there gives its file and line.

### 2.1 Cursors

- **The reader.** Paper sets `cursor: default` on purpose on about 20 controls (`packages/ui/src/styles/button.css:12`, `toggle.css:19`, `tabs.css:21`, `choice.css:11`, `popover.css:38` and others), and `pointer` on three surfaces: recent rows, thread toggles and the phone's bottom bar. Measured in the built reader, none of 106 chrome controls shows the hand; none of those three was on screen in that run (no recent files, no Threads tab, a desktop-width window). Only the PDF's own links show it.
- **The tools.** Every text-layer span carries an inline `cursor: text` (`packages/viewer/src/layers/text-layer.ts:275`), so over text the Link tool's crosshair (`packages/ui/src/tools/tools.ts:157`) turns into an I-beam. The hand tool has the same fault and never shows `grabbing`.
- **The sites.** The docs and the website use `pointer` on most buttons, but Starlight's own search button keeps it while disabled (its `Search.astro`, which the docs import unchanged).
- **The desktop.** Chrome labels can be selected and show the I-beam, since nothing sets `user-select: none`.
- **The design.** The design language never names a cursor; the Paper design files draw the hand.

### 2.2 States

- **Pressed is missing** on toggles, tool toggles, the mode switcher, tabs, checkboxes, radios, the switch, swatches and thumbnails, and on both sites everywhere (`--paper-pressed` is unused there). §7.0 still marks hover, pressed and disabled as proposed.
- **Hover is missing** on checkboxes, radios, the switch, swatches, reader thumbnails (`apps/web/src/app.css:83-113`), the sites' code tabs, Starlight's tabs, the site select and the runner's buttons. A selected toggle loses its hover (`toggle.css:22-33`).
- **Hover sticks on touch.** Nine raw `:hover` rules in the reader (document tabs and their close button, history rows, threads, the rail handle, properties rows, reader blocks) and about eleven on the sites (the header, buttons, Copy as Markdown, the runner's divider) stay lit after a tap; nothing uses `@media (hover: hover)`. React Aria's `data-hovered` already ignores touch. On iOS, `-webkit-tap-highlight-color` is cleared only on buttons, the status bar and thumbnails, so toggles, tabs, swatches and rows flash grey under a tap.
- **Disabled looks enabled** in the reader and on the sites (`--paper-disabled` is unused on the sites).
- **Selection moves layout.** Tabs and the mode switcher widen when selected, because the label's weight changes.
- **No order of precedence** says what wins when a control is selected, hovered and focused at once.

### 2.3 Motion

- **Four animations in all**, two of them on layout: the inspector animates `inline-size` (`shell.css:58-68`) and the phone top bar `margin` (`phone.css:26-36`), both at a hard-coded 160 ms. The others are the indeterminate progress bar (`progress.css:39-55`, on the tokens) and the "View original page" outline's 2 s fade (`apps/web/src/app.css:130-153`, its duration also in `reader-origin.ts`).
- **No hover transitions, no overlay enter or exit.** No overlay uses React Aria's `data-entering` or `data-exiting`; menus, popovers, tooltips, dialogs, the palette, toasts and link previews pop in and out. The drawer and its scrim appear at once; the switch knob jumps. Pages and thumbnails swap from placeholder to picture in one frame.
- **No smooth scrolling.** Every jump sets `scrollTop` directly, which this plan keeps: a jump to a page or a search result should land at once.
- **Tokens.** The `motion` group has four timings (toast 4 s, pill fade 1.5 s, progress cycle 1.2 s, tooltip delay 500 ms), no state durations and no easings; `duration-pill-fade` is unused.
- **Reduced motion means three things.** The design language says "no transition" when the accent switches, and its accessibility checklist says "instant cross-fades". The reader's Settings › Reduce motion sets `transition: none !important` on everything (`settings.css:217-229`). The sites have no reduced-motion layer, and the motion they ship from Starlight's sidebar caret and Expressive Code's Copy ignores the preference.
- **The desktop.** The tab strip appears when a second tab opens, or when another window is open, and pushes the page down by its own height (`apps/desktop/src/tabs/use-tabs.tsx:116-118`); tabs open, close and reorder with no motion. Under Wayland, WebKitGTK on GTK 3 ignores GNOME's reduce-motion setting ([GTK issue 5813](https://gitlab.gnome.org/GNOME/gtk/-/issues/5813)), and the desktop Settings have no Reduce motion switch.
- **A desktop fault found on the way.** Tab dragging uses HTML5 drag and drop (`packages/ui/src/reader/document-tabs.tsx:289-293`), which Tauri turns off on Windows while its file-drop handler is on, so reordering and dragging tabs out may not work there at all; no end-to-end test drags a tab. It isn't motion, but it is the desktop's worst fault in this area, so the round fixes it (U7g).

## 3. Decisions

The owner chose 1–4 on 2026-11-14; the rest follow from them and from the research.

1. **The hand on everything clickable.** Every control that acts shows `cursor: pointer`, in the reader, the desktop app and both sites: buttons, icon buttons, toggles, tabs, the mode switcher, menu items, options, checkboxes, radios, switches, swatches, thumbnails, rows that open or select, and links. Disabled and blocked controls show the arrow. A pending control shows `progress`. Text fields and page text keep the I-beam; drag grips show `grab` and `grabbing`; resizers show `col-resize`; creation tools show their own cursor, which wins over the text layer's. (The owner chose this over the native convention, the arrow on buttons, which [Apple](https://developer.apple.com/design/human-interface-guidelines/pointing-devices), [Microsoft](https://learn.microsoft.com/en-us/windows/win32/uxguide/inter-mouse) and [GNOME](https://developer.gnome.org/hig/guidelines/pointer-touch.html) follow, as does the [CSS spec](https://www.w3.org/TR/css-ui-4/#cursor). The cursor tokens of §4.1 keep the two roles apart, so going back would be one line.)
2. **Hover and press fade, quickly.** A hover state fades in over 80 ms and out over 150 ms; press-in and keyboard focus are instant; release goes back to hover over 80 ms. Only colour, background, border, outline colour and opacity change, never anything that moves.
3. **Overlays fade with a slight scale.** Menus, popovers, tooltips, link previews and the mini toolbars fade in with a scale from 0.97 at their anchor over 120 ms and out over 80 ms. Dialogs and the rail drawer take 200 ms in and 150 ms out (the owner's choice), and so do the other panels, the inspector, toasts and sheets, which this plan adds to the same rule. The command palette stays instant.
4. **Reduce motion removes movement and keeps fades.** Under the system preference and under Settings › Reduce motion alike, nothing slides, scales, grows, shimmers or springs; short colour and opacity fades stay, since [WCAG 2.3.3](https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions) asks about motion, not about fades. Reduce motion never removes feedback: a state still changes, at once or by a fade. The Settings switch stops overriding every transition and joins the same rule, and the desktop gets the switch too (decision 8).
5. **No motion on switches of look.** Theme, accent, page appearance, density and direction apply without transitions: `:root` carries `data-motion-suppressed` from each switch until the frame after the new look draws, so nothing fades from the old colours.
6. **Every enter has an exit, and an exit never blocks.** Anything that animates in animates out, with the exit durations of §4.1. An element that is leaving is `inert` and lets clicks through from its first frame, so a second click or key isn't lost to a closing menu.
7. **Selection never moves layout.** A selected tab or segment keeps its width: the label reserves the bold width (a hidden bold copy sets the box), or the selection shows by colour and underline only.
8. **The desktop follows Linux's preference itself.** Since WebKitGTK may miss GNOME's setting, the desktop reads it through the platform (a capability flag, never a platform check in UI code, ADR 0010) and exposes Settings › Reduce motion as the web reader does.
9. **Links keep two states of their own.** The current page or tab shows by `aria-current` and an underline; visited links look like any other, in the apps and on both sites, since in a reference site or an app "visited" says nothing useful.
10. **Loading fades, never shimmers.** A page or thumbnail placeholder cross-fades into its picture over `duration-state-out`; skeletons keep their still blocks (the design language's rule). At high zoom the sharp detail canvas fades in over the blurry first render. Elsewhere the sharp picture replaces the blurry one at once: the viewer draws the sharp tiles into the page's own canvas, and a cross-fade there would need a second full-size canvas for every page, doubling the view's canvas memory for a 150 ms effect (U7d).

## 4. The system

### 4.1 Tokens

New entries in `packages/tokens/tokens.json`, in ADR 0024 §7's semantic layer, generated into `--paper-*` custom properties as the rest are. The values come from the research, which compared [Material 3](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs), [Carbon](https://carbondesignsystem.com/elements/motion/overview/), [Fluent 2](https://fluent2.microsoft.design/motion), Atlassian and Primer.

| Token | Value | Use |
| --- | --- | --- |
| `duration-state-in` | 80 ms | Hover in, release back to hover |
| `duration-state-out` | 150 ms | Hover out, a placeholder fading into its picture |
| `duration-enter-small` / `duration-exit-small` | 120 / 80 ms | Menus, popovers, tooltips, link previews, mini toolbars |
| `duration-enter-panel` / `duration-exit-panel` | 200 / 150 ms | Dialogs, the inspector, the rail drawer, toasts, sheet detents |
| `duration-view` | 250 ms | The Reader toggle's view transition (U7h); the ceiling for any UI motion |
| `duration-link-preview-delay` | 300 ms | Already the viewer's figure, now a token |
| `ease-standard` | `cubic-bezier(0.2, 0, 0, 1)` | Anything that stays on screen: state colours, detent snaps, reorders |
| `ease-enter` | `cubic-bezier(0, 0, 0.2, 1)` | Things appearing |
| `ease-exit` | `cubic-bezier(0.3, 0, 1, 1)` | Things leaving for good |
| `cursor-action` | `pointer` | Every control that acts (§3, 1) |
| `cursor-link` | `pointer` | Links, in chrome, on the sites and in pages: the same value today, a separate role so decision 1 can change for one without the other |
| `cursor-disabled` | `default` | Disabled and blocked controls |
| `cursor-pending` | `progress` | Pending controls |

The existing four timings stay. Instant changes (focus rings, press-in, the palette) use no transition at all rather than a zero-length token. Under reduced motion, `transform` and size changes resolve to none, and colour and opacity fades keep their durations (§3, 4).

### 4.2 The state model

Every interactive element styles the states it has, written in this order inside `@layer paper.components` so later rules win on what they set:

1. **Rest.**
2. **Value:** selected, checked, indeterminate, current (`aria-current`), open (its menu or popover showing). These set the base fill, text or underline. An open ghost trigger uses the `pressed` fill; an open status item uses `accent-tint` with an `accent` border.
3. **Interaction:** hover, then pressed, so pressed wins. Hover and pressed are translucent layers drawn over the value's fill (`background-image: linear-gradient(var(--paper-hover) 0 0)`), so selected + hover needs no token of its own; the primary button swaps solids instead (`accent` → `accent-strong` → `primary-pressed`). Hover comes only from React Aria's `data-hovered`, or from `:hover` inside `@media (hover: hover) and (pointer: fine)`. A drop target (`data-drop-target`) sits here too and wins over hover.
4. **Validity:** invalid sets the border and help text to `danger`; it never recolours the focus ring.
5. **Focus:** `data-focus-visible` draws the 2 px `focus` outline at its offset, in every state above, never transitioned.
6. **Availability**, last: disabled (the `disabled` text, `hairline` border, no hover, press or focus), blocked (`aria-disabled`: keeps focus and its ring, no hover, a readable label), pending (`aria-busy`, no hover or press fill, the progress in the label, the `progress` cursor), read-only (no hover fill, text still selectable).

§7.0 of the design language takes this order, and its proposed hover, pressed and disabled rows become rules. The gallery's `states.tsx` renders each combination a component supports (rest, hover, pressed, focus-visible, selected + hover, selected + focus, disabled, pending, invalid + focus), so U6f's visual tests catch drift.

### 4.3 Rules for motion

- Hover and press transition colour, background, border, outline colour and opacity, nothing else. Enter, exit and panel motion may add `transform`, and an overlay its shadow's opacity. A transition names its properties, never `all`.
- Durations and easings come from the tokens; a check fails on a literal `ms`, `cubic-bezier` or `ease` in `@laminapdf/ui`'s and the sites' CSS, as the existing tokens-only check does for colours.
- Enter and exit use React Aria's `data-entering` and `data-exiting` in the reader and desktop app, and `@starting-style` with `transition-behavior: allow-discrete` on the sites, which every target browser supports (Baseline 2024). Same-document view transitions (Baseline since October 2025) are for switching whole views, and only where §7 names them (the Reader toggle, U7h).
- Panels move by `transform` and `opacity` inside a box whose size is set at once: the inspector's dock takes its width on the first frame and its contents slide in; the phone top bar leaves by `translateY(-100%)`. Under reduced motion they fade instead.
- Controls that draw their own pressed state clear `-webkit-tap-highlight-color`, so a tap on iOS shows Paper's pressed fill and not a grey flash.
- Scroll-linked effects and motion on the page canvas are out: the page is for reading.

## 5. Web and desktop

| Area | Web | Desktop |
| --- | --- | --- |
| Cursors | The tokens | The same tokens; chrome labels also get `user-select: none` |
| Reduced motion | `prefers-reduced-motion` and Settings › Reduce motion | The same, plus the platform's own reading of the setting where the webview misses it (`reduceMotion` capability), and Settings › Reduce motion |
| Tabs | No change | Open, close and reorder with motion; the strip reserves its space so a second tab doesn't push the page down |
| Tab drag | No change | Pointer-driven, so it works on Windows, where HTML5 drag is off beside Tauri's file drop |
| Webview giveaways | No change | No webview context menu in chrome, the window's background colour set so dark mode doesn't flash white, `overscroll-behavior: none`, a dimmed inactive window. The owner asked for a desktop app that is delightful to use, and the research found these to be what makes it feel like a web page (its note, §4) |

## 6. Tests

- **Static:** the tokens-only check covers durations, easings and cursors (§4.3); a check fails on a raw `:hover` outside a `(hover: hover)` guard and on `transition: all`, in `@laminapdf/ui` and the sites.
- **Gallery:** each component's state combinations (§4.2) in `states.tsx`, screenshotted by the gallery's visual tests, with each control's computed cursor checked in its browser test (U7b, U7i); axe in three engines as today. The gallery's motion switch in its URL follows the new rule (U7a).
- **Built reader:** a Playwright test over the built reader records every control's computed `cursor` and fails on any that isn't the rule's, the measurement this plan began with (U7d); another opens each overlay with reduced motion on and off and checks that it fades, and that nothing moves under reduced motion (U7c).
- **Sites:** the same cursor check over the built docs and website pages (U7e).
- **Desktop:** a WebdriverIO scenario checks the chrome's cursors (U7f), and another drags a tab to reorder it and out to a new window, on each OS (U7g).
- **By hand:** every PR that changes what people see is tried in the built app, as a person uses it (CLAUDE.md).

## 7. Todos

| Todo | What | Depends on |
| --- | --- | --- |
| [U7a](../todos/done/U7a-motion-and-state-tokens.md) | The tokens, the state model and reduced-motion rule in the design language, `data-motion-suppressed`, the tap highlight, the gallery's motion switch, the static checks | None |
| [U7b](../todos/done/U7b-control-states-and-cursors.md) | The action controls in `@laminapdf/ui` (buttons, toggles, tabs, the mode switcher, menu items, options): the state model, cursors, hover fades, touch-safe hover, selection without layout shift | U7a |
| [U7i](../todos/done/U7i-choice-and-field-states.md) | Choices, fields and rows in `@laminapdf/ui` (checkboxes, radios, the switch and its knob, swatches, chips, the select, text fields, outline rows): the same | U7a |
| [U7c](../todos/done/U7c-overlay-enter-and-exit.md) | Menus, popovers, tooltips, link previews, dialogs, toasts, sheets, the drawer and scrim enter and leave | U7a |
| [U7d](../todos/done/U7d-reader-surfaces-motion.md) | The reader's surfaces: thumbnails, pages fading in, the inspector and phone bar without layout animation, tool cursors over the text layer, the reader's raw `:hover` rules, the built-reader cursor test | U7b, U7i |
| [U7e](../todos/done/U7e-sites-states-and-motion.md) | The docs site, the website and the site kit: states, cursors, links, touch guards, a reduced-motion layer, the runner's busy state, one focus ring | U7a |
| [U7f](../todos/ready/U7f-desktop-chrome-motion.md) | The desktop chrome: cursors and their check, tab motion, the strip's reserved space, unselectable labels, Settings › Reduce motion and the platform's reading of it, the webview giveaways | U7b, U7c |
| [U7g](../todos/done/U7g-pointer-tab-drag.md) | Tab reordering and dragging out by pointer events, tested on each OS | None |
| [U7h](../todos/ready/U7h-delight-pass.md) | The round's close: a view transition for the Reader toggle, copy and undo feedback, a by-hand pass over every screen of the apps and both sites against this plan | U7b, U7c, U7d, U7e, U7f, U7g, U7i |

U7g fixes a fault that needs no new tokens, so it can start at once; so can U7a.

## 8. Open questions

1. **Brand motion.** Should the home screen or the website's front page carry one signature motion (a page turning, a sheet sliding into place)? Not in this round; the owner decides after U7h, when the restrained baseline is in.
2. **The desktop's title bar.** Joining the native title bar and the tab strip into one bar on macOS needs an overlay title bar and a capability flag for the inset. That belongs to the desktop plan, not to this round.
