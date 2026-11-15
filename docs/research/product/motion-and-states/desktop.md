# Desktop app: cursors, interaction states and motion (research)

Scope: `apps/desktop` (Tauri 2.12.5, wry 0.57.1, tao 0.37.2; `Cargo.toml:163`, `Cargo.lock`) on `main` at `cd888aa2d` (v0.4.0). Shared chrome comes from `packages/ui`. This note covers what differs on the desktop. Paths are relative to the repo root. Nothing was built or run: every finding comes from reading the code, and the items marked **verify** need a check in a built app.

Supported webviews (ADR 0008): macOS 13+ (`docs/decisions/0008-desktop-shell.md:191`) running system WKWebView, so Safari 16.x–26 depending on updates; Windows running Evergreen WebView2 (the D1a spike measured 156, `0008:19`); Linux from Ubuntu 22.04 (`0008:33`) running WebKitGTK **4.1, the GTK 3 API** (`libwebkit2gtk-4.1-dev`, `.github/workflows/release.yml:187`).

---

## 1. Desktop-specific UI: cursors, states and motion

### 1.1 Where the cursor rules stand today

The shared chrome already follows the native rule almost everywhere. Buttons use `cursor: default`:

- `packages/ui/src/styles/button.css:12`: `.paper-button, .paper-icon-button { cursor: default }`. A pending button shows `progress` (`button.css:119`).
- `default` also appears in `tabs.css:21`, `toggle.css:19`, `choice.css:11`, `mode-switcher.css:22`, `popover.css:38`, `palette.css:104`, `settings.css:61,305`, `status-bar.css:18`, `thumbnail.css:15`, `outline-row.css:21,46`, `history.css:62`, `swatches.css:31`, `appearance-chips.css:21` and `notice.css:81`.
- The tab label is `cursor: default` (`document-tabs.css:65`).
- Resize and drag handles are right: `.paper-rail-handle { cursor: col-resize }` (`shell.css:22`), and the sheet grabber uses `grab`/`grabbing` (`sheet.css:70,74`, phone only).

**The outliers, the "wrong cursor" the owner sees:**

| Where | Rule | Why it is wrong |
|---|---|---|
| Recent-file rows on the start window | `recent.css:43` `.paper-recent-row { cursor: pointer }` (a missing row resets to `default`, `recent.css:77-79`) | The row is a list item that opens a document in place, like the rows in Finder Recents, Preview or Xcode's welcome window, which all show the arrow. It is the one pointer hand on the desktop start window. |
| Notice actions "Reopen", "Locate…", "Close" | `apps/desktop/src/app.css:32-41` `.desktop-link { … cursor: pointer }`, used at `app.tsx:169`, `file-changes.tsx:90-99` | These are buttons that run actions, styled as links. Under every native guideline below, an action gets the arrow, whatever it looks like. |
| Comment threads | `threads.css:54,76` `cursor: pointer` on the thread toggle and `button.paper-thread-part` | These are disclosure buttons, and they disagree with `button.css`. |
| Bottom bar | `phone.css:66` `cursor: pointer` | Phone layout only, so it is harmless on desktop, but it is the same inconsistency. |

**Text and chrome selection (verify).** No `user-select: none` exists anywhere in `packages/ui/src/styles` or `apps/desktop/src` (searched for `user-select`). Static chrome text therefore shows the I-beam and can be selected: the wordmark (`app.tsx:132`, `.desktop-wordmark`), the status bar text, the notice titles, the Settings headings and the privacy line. Two side effects follow: double-clicking a label selects it, and ⌘A or Ctrl+A outside the page selects the whole UI. A native app shows the arrow over its labels and never selects them. Button text is not affected, because `cursor: default` on buttons hides the I-beam there.

**Disabled controls.** These keep `default`, which is correct for native: Radix uses `not-allowed`, native toolkits don't.

### 1.2 Tab strip (`packages/ui/src/reader/document-tabs.tsx`, `styles/document-tabs.css`, `apps/desktop/src/tabs/use-tabs.tsx`)

Cursors are fine: the label has `default` and the close button is a UA `<button>`, which shows the arrow. The states and motion have gaps:

- **Hover only through `:hover`.** See `document-tabs.css:40-42` (tab) and `:104-107` (close button). The menu button uses React Aria's `[data-hovered]` (`:119`), so the strip mixes the two models. A plain `:hover` sticks after a touch tap on Windows touch laptops.
- **No pressed state** on the tab, the close button or the menu button (no `:active` or `[data-pressed]` rule).
- **The close button appears with `visibility` and no fade** (`document-tabs.css:90,98-102`). Chrome, Edge and Safari fade it in on hover.
- **Drag feedback is the browser's default.** The dragged tab drops to `opacity: 0.5` (`:49-51`) with no transition. The drag image is the webview's snapshot of the label. There is no insertion marker and no live reorder: the order changes only on drop, and only when the drop lands *on a label* (`document-tabs.tsx:281-293`, `drop()` at `:234-247`). A drop on the close button, in the gap after the last tab or on the menu button does nothing: Rust's `tab_drag_end` sees the cursor over its own window and does nothing (`src-tauri/src/tab_drag.rs:1-15`). Every native tab bar slides the neighbouring tabs apart while you drag (Safari, Finder, Chrome, Edge, Files/Nautilus).
- **Dragging out of the window (D4e, `tab_drag.rs`).** The design is sound: Rust polls the cursor every 50 ms and finds the window under it. The user sees something different, though (verify each point):
  - Over another Lamina window, that window gets `tab://drag-over`. It then shows a dashed accent border (`apps/desktop/src/app.css:58-68`, set at `use-tabs.tsx:177-179`), which appears and disappears with no fade. Its webview never calls `preventDefault` for another strip's drag type (`document-tabs.tsx:230-233`), so the OS shows the **"not allowed" cursor while the border says "drop here"**.
  - Outside every window, a drop with no target is a cancelled drag to the OS. macOS animates the drag image **sliding back to the tab**, which reads as "cancelled", and then a new window opens with the tab.
  - The new window appears where the cursor was, at `GRIP` (`tab_drag.rs:36`), with no motion linking it to the tab.
  - A cancelled drag (Escape) outside the window also moves the tab (`tab_drag.rs:13-15`, documented).
- **Windows: HTML5 drag may not work at all (verify, high priority).** Tauri replaces WebView2's drop handler whenever the drag-drop handler is on. Its own docs say: *"This is required to use HTML5 drag and drop APIs on the frontend on Windows since we replace the drag drop handler of WebView2"* (`tauri-2.12.5/src/webview/webview_window.rs:1094-1101`, `disable_drag_drop_handler`). The app keeps the handler on, because it opens dropped files in Rust (`docs/todos/done/D3a-open-from-os.md:38`, and `build_window` at `windows.rs:159-176` never calls `disable_drag_drop_handler`). The tab strip's reorder and drag-out both run on HTML5 `draggable`/`dragstart`/`drop` (`document-tabs.tsx:289-293`), and no e2e spec drags a tab (no `drag` in `apps/desktop/e2e`). On Windows, dragging tabs may therefore do nothing.
- **The strip shows and hides by tab count.** It shows only with at least two tabs, or one tab while other windows are open (`use-tabs.tsx:116-118`). Opening a second document therefore pushes the whole UI down by about 34 px (`--paper-document-tab-strip-height`), and closing back to one pulls it up, with no transition. That is a layout jump at the moment the user's attention is on the new document.
- **Overflow is hidden.** The list scrolls with `scrollbar-width: none` (`document-tabs.css:26`). Nothing calls `scrollIntoView` when a tab activates from the keyboard, the menu or `tab://activate` (no `scrollIntoView` in `document-tabs.tsx` or `apps/desktop/src`), so the active tab can sit off-screen.
- **Missing conventions:** middle-click to close (no `onAuxClick`), and keeping tab widths fixed while the pointer stays in the strip after a close, so the next close button lands under the cursor (Chrome and Safari both do this).

### 1.3 Home screen and recent files (`apps/desktop/src/recent.tsx`, `packages/ui/src/styles/recent.css`)

- The rows show the `pointer` cursor (above). Their hover state is a background swap with no transition (`recent.css:46-48`). The focus ring comes from `[data-focus-visible]`, which is correct. There is no pressed state.
- The start screen mounts in place of the viewer and back with no cross-fade (`app.tsx:157-161,174`). `useShownParts` (`app.tsx:389-399`) already avoids a flash between tabs, which is good.

### 1.4 Settings (`apps/desktop/src/settings.tsx`)

- The desktop registers only the **Files** section (`settings.tsx:60-66`). The web reader's Appearance section, with theme, accent and the **Reduce motion** setting (`packages/ui/src/reader/appearance.tsx:1-39,75,158`, CSS at `settings.css:217-230` under `[data-motion="reduce"]`), is not wired up on the desktop: `AppearanceSettings` is used only in `apps/web` and `apps/gallery`. This matters more on Linux; see §3.1.
- The dialog itself is the shared `SettingsDialog`. On macOS, preferences are a separate window, opened with ⌘, and titled "Settings" since macOS 13. A modal web dialog inside the document window is a web pattern.

### 1.5 File-change notices (`apps/desktop/src/file-changes.tsx`)

- The actions are `.desktop-link` buttons with the `pointer` cursor (`file-changes.tsx:90-99`). They have no hover, pressed or focus-visible styles of their own: `app.css:33-41` sets only the colour and the underline, so focus falls back to the UA outline.
- **Missing style:** `MissingRestoredTab` renders `<div className="desktop-missing">` (`file-changes.tsx:148`), and no CSS anywhere defines `.desktop-missing` (searched `apps` and `packages`). A restored tab whose file is gone shows an unstyled notice where the document would be.
- Notices mount and unmount instantly. The "reloaded" toast is the shared `ToastRegion`, which has no transition rules (`toast-region.css` holds no `transition` or `animation`).

### 1.6 Native menus (#560, `src-tauri/menus.json`, `src-tauri/src/menu.rs`, `src/menus/use-native-menus.tsx`)

Native menus get the OS's cursor, highlight and fade for free, so there is nothing to fix there. The webview's own popover menus are a different matter: the tab-strip menu (`document-tabs.tsx:116-130`) and the top bar's "more" menu (`app.tsx:133-136`). They open and close instantly, while native macOS menus fade out (about 200 ms) and WinUI flyouts slide and fade in (see §4).

### 1.7 Window chrome (`src-tauri/src/windows.rs`, `tauri.conf.json`)

- **Native decorations.** `build_window` sets only the title, size, minimum size and position (`windows.rs:162-176`), and `tauri.conf.json` has `"windows": []`. When the strip shows, the title bar and the tab strip are two bars (title bar, strip, top bar). Safari, Finder, Chrome and Edge put the tabs in the title bar.
- **Title.** `"{name} — Lamina PDF"` (`windows.rs:208-213`, `use-tabs.tsx:230-231`). macOS document windows show the document name alone, with a proxy icon and the "edited" dot. Windows uses "name - App".
- **No window background colour.** No `background_color` and no hide-until-painted step, except for the physical-corner case at `windows.rs:172`. In dark mode, a new window, a tab dragged out or a session restore probably flashes the webview's white before `--paper-ground` paints (verify). Tauri 2.12 has `WebviewWindowBuilder::background_color` (`webview_window.rs:1248`).
- **No inactive-window state.** When a window loses focus, native chrome greys out: the selected tab, accent fills, the sidebar selection. `on_window_event` tracks `Focused` (`lib.rs:316-331`) but sends the webview nothing it could style with.
- **The webview's own context menu is still on (verify).** Nothing suppresses `contextmenu` outside the tabs and links (searched `contextmenu` and `AreDefaultContextMenusEnabled`). On Windows, WebView2's default menu (Back, Refresh, Save as, Print, …) appears on right-click in empty chrome. AreDefaultContextMenusEnabled defaults to TRUE ([learn.microsoft.com, ICoreWebView2Settings](https://learn.microsoft.com/microsoft-edge/webview2/reference/win32/icorewebview2settings)), and wry 0.50.5 added a Windows option to turn those menus off ([tauri wry 0.50.5 notes](https://v2.tauri.app/release/wry/v0.50.5)). On macOS, WKWebView shows "Reload". Either way it is the biggest "this is a web page" tell.
- **Overscroll (verify).** No `overscroll-behavior` on `html` or `body` (`app.css:2-13`; the only one is `sheet.css:126`). WKWebView can rubber-band the whole window on a trackpad swipe.

---

## 2. Platform conventions: which cursor for what

| Source | Buttons / controls | Links | Text | Drag / move | Resize |
|---|---|---|---|---|---|
| **Apple HIG, Pointing devices (macOS)** [link](https://developer.apple.com/design/human-interface-guidelines/pointing-devices) | **Arrow**: "Standard pointer for selecting and interacting with content and interface elements." | **Pointing hand**: "The content beneath the pointer is a URL link to a webpage, document, or other item." | I-beam: "Selection and insertion of text is possible" | Open hand: "Dragging to reposition content within a view is possible"; closed hand while dragging; "Operation not allowed" for a refused drop | resize-left-right, up-down and edge variants |
| **Microsoft, Mouse interactions (Windows App SDK / WinUI)** [link](https://learn.microsoft.com/en-us/windows/apps/develop/input/mouse-interactions) | "**Always use the arrow cursor for clickable elements.** don't use the pointing hand cursor for links or other interactive elements. Instead, use hover effects." Also: "Don't use the cursor for hover feedback, the feedback provided by the element is sufficient." | Arrow (even links, in current guidance) | "Use the text cursor for selectable text." | "Use the move cursor when moving is the primary action"; grasping hands for panning a canvas | the resize cursors "when an object is resizable" |
| **Microsoft, Windows 7 UX guide (Win32)** [link](https://learn.microsoft.com/en-us/windows/win32/uxguide/inter-mouse) | "command buttons already have a strong affordance, so they don't need a hand pointer." | "**Use the hand (or 'link select') pointer only for text and graphic links.** … The hand pointer must mean 'this target is a link' and nothing else." | Text select | "unavailable" pointer over an invalid drop target | splitters, row and column resize |
| **GNOME / GTK 4** ([HIG, Pointer & touch](https://developer.gnome.org/hig/guidelines/pointer-touch.html) doesn't specify cursors) | GtkButton sets no cursor, so it shows the **default arrow** ([gtkbutton.c](https://gitlab.gnome.org/GNOME/gtk/-/raw/main/gtk/gtkbutton.c)) | **GtkLinkButton** calls `gtk_widget_set_cursor_from_name(…, "pointer")` ([gtklinkbutton.c:382](https://gitlab.gnome.org/GNOME/gtk/-/raw/main/gtk/gtklinkbutton.c)) | text (GtkLabel when selectable, entries) | toolkit default | toolkit default |
| **CSS Basic UI 4** [link](https://www.w3.org/TR/css-ui-4/#cursor) | `default`: "The platform-dependent default cursor. Often rendered as an arrow." | `pointer`: "The cursor is a pointer that indicates a link." | `text`: "Indicates text that may be selected." | `grab`: "something can be grabbed (dragged to be moved)"; `grabbing`; `move` | `col-resize` etc. |
| **Radix Themes** [docs](https://www.radix-ui.com/themes/docs/theme/cursors) | "By default, interactive elements that don't link to another page use the regular arrow cursor." Tokens: `--cursor-button: default`, `--cursor-menu-item: default`, `--cursor-switch: default` ([@radix-ui/themes 3.2.1 tokens/base.css:1072-1080](https://unpkg.com/@radix-ui/themes@3.2.1/tokens/base.css)) | `--cursor-link: pointer` | n/a | slider thumb `default` | n/a |
| **Adobe Spectrum / React Aria** | Spectrum buttons keep the arrow in every state; a button with `href` shows the pointer ([spectrum.adobe.com/page/button](https://spectrum.adobe.com/page/button), via search, page moved). React Aria ships no styles, so the cursor is the app's choice ([react-aria.adobe.com/styling](https://react-aria.adobe.com/styling)) | pointer only for `href` | n/a | n/a | n/a |
| **Tailwind v4 preflight** (de facto web default) [upgrade guide](https://tailwindcss.com/docs/upgrade-guide) | "Buttons now use `cursor: default` instead of `cursor: pointer` to match the default browser behavior." | UA `pointer` on `a[href]` | n/a | n/a | n/a |
| **GitHub Primer CSS** [@primer/css 21.5.1 buttons/button.scss:17,32](https://unpkg.com/@primer/css@21.5.1/buttons/button.scss) | `.btn { cursor: pointer }`, `default` when disabled | pointer | n/a | n/a | n/a |
| **Material 3 (material-web)** [@material/web 2.4.0 button/internal/_shared.scss:21,75](https://unpkg.com/@material/web@2.4.0/button/internal/_shared.scss) | `:host { cursor: pointer }`, `default` when disabled | pointer | n/a | n/a | n/a |

Background: [hidde.blog, "Some pointers on default cursors"](https://hidde.blog/some-pointers-on-default-cursors/) sums up the arrow-for-buttons case.

**Reading.** All three desktop platforms agree: the arrow for buttons, menu items, tabs, switches and list rows; the pointing hand only for links that navigate; the I-beam only for selectable text; grab or resize cursors for handles. Current Windows guidance goes further and drops the hand even for links. Only web-first design systems (Primer, Material) put `pointer` on buttons, and the trend is away from it (Tailwind v4, Radix). Lamina's own `button.css:12` already chose `default`.

### Recommended rule (one rule for both shells, no platform check)

1. **`cursor: default` on every control that runs an action or changes state.** That covers buttons, icon buttons, tabs, menu items, switches, chips, list rows (recent files) and disclosure toggles, whether they *look like* links or not ("Reopen", "Locate…", the thread toggles).
2. **`cursor: pointer` only on real navigation:** an `<a href>`, or a React Aria `Link` with `href`, that goes to a URL, a document location or another app or page. In the PDF, that means link annotations (the viewer's link layer).
3. **`text` only where text can be selected:** the PDF text layer, inputs and the find field. Chrome labels get `user-select: none` (and `-webkit-user-select: none` for WebKit), so they show the arrow. Notice and error bodies stay selectable, so error text can be copied.
4. **Handles:** `col-resize`/`row-resize` for splitters, `grab` → `grabbing` for movable content (sheet, canvas pan), the arrow on tabs (native tab bars don't show a hand), `progress` only where no other busy feedback exists, and `default` for disabled controls (not `not-allowed`).
5. **One token set, after Radix's model.** Define `--paper-cursor-button: default`, `--paper-cursor-link: pointer`, `--paper-cursor-text: text` and `--paper-cursor-handle-*` in `packages/tokens`, and make every rule read them. A Biome or stylelint check that bans a literal `cursor: pointer` outside the link selectors keeps the rule from drifting.

**Why not a platform-dependent rule.** ADR 0010 allows capability flags only. The web reader is also a desktop-class pointer app that users run in a browser on the same three OSes, and Radix and Tailwind show that `default` on buttons is fine on the web. A capability such as `prefersNativeCursors` would split the CSS and the gallery screenshots for no gain, so the same rule should serve both shells.

---

## 3. Platform motion support in Tauri 2's webviews

### 3.1 `prefers-reduced-motion`

| Webview | Follows the OS? | Notes |
|---|---|---|
| WebView2 (Chromium, Evergreen) | Yes: Windows 10 "Show animations in Windows", Windows 11 Accessibility › Visual effects › "Animation effects" ([MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-motion)) | Same engine as Edge, so the behaviour matches Chrome. |
| WKWebView (macOS 13+) | Yes: System Settings › Accessibility › Display (Motion on 26+) › Reduce motion (MDN, same page) | Shared with Safari. |
| WebKitGTK 4.1 (GTK 3) | **Partly.** The media query arrived in WebKitGTK 2.41.4 ([release](https://webkitgtk.org/2023/05/17/webkitgtk2.41.4-released.html)), so 2.42 stable, and follows `gtk-enable-animations` ([WebKit bug 255097](https://bugs.webkit.org/show_bug.cgi?id=255097), fixed in 263777@main) | **Doesn't follow GNOME's setting on GTK 3 under Wayland** ([GTK issue 5813](https://gitlab.gnome.org/GNOME/gtk/-/issues/5813), cited in comment 3 of the WebKit bug), and Tauri's `webkit2gtk-4.1` is GTK 3. An Ubuntu report says the same ([LP 2062655](https://bugs.launchpad.net/bugs/2062655)). A toggle reaches the page only after a reload (bug 286265, linked from 255097). KDE Plasma's "Animation speed: Instant" maps through GTK settings, if at all. |

**Consequence.** On Linux the OS setting can't be trusted, which makes the in-app **Reduce motion** setting the reliable switch. The desktop doesn't wire it up (§1.4). Add the shared `AppearanceSettings` to the desktop Settings, kept in the main process's preferences so it is set before the first paint and shared across windows. Have it set `data-motion` on `:root`. Every new animation must use both gates, the media query *and* `[data-motion="reduce"]`. Today only `progress.css:52`, `shell.css:58`, `phone.css:26` and `tokens.css:638-643` use the media query, and the `[data-motion]` layer (`settings.css:219-230`) covers both.

### 3.2 View Transitions API (same-document)

- Chrome and Edge 111+, so every Evergreen WebView2 has it ([caniuse](https://caniuse.com/view-transitions)).
- Safari 18.0+ (caniuse). macOS 13 and 14 get WebKit 18 or later only when Safari is updated, so a macOS 13 machine stuck on Safari 16 or 17 has none.
- WebKitGTK: the 2.48 highlights list "Cross-document view transitions, and many improvements overall to the View Transitions support" ([webkitgtk.org 2.48](https://webkitgtk.org/2025/04/08/webkitgtk-2.48.html)), and WPE 2.46 lists View Transitions ([wpewebkit 2.46](https://wpewebkit.org/blog/2024-wpewebkit-2.46.html)). On Ubuntu 22.04 the version depends on the security-update stream. **Verify** on the CI images with `'startViewTransition' in document`.
- **Rule:** always feature-detect (`document.startViewTransition?.(…) ?? update()`). Treat it as progressive enhancement, never needed for correctness, and keep it off during a reduced-motion state. Use it for the start screen ↔ document swap and for tab switches. Avoid it while tiles are streaming in, because a view transition snapshots the page and freezes the canvas during the transition.

### 3.3 `@starting-style` and `transition-behavior: allow-discrete`

- `@starting-style`: Chrome and Edge 117+, Safari 17.5+ ([caniuse](https://caniuse.com/mdn-css_at-rules_starting-style)).
- `transition-behavior`: Chrome and Edge 117+, Safari 17.4+ ([caniuse](https://caniuse.com/mdn-css_properties_transition-behavior)).
- WebView2 has both. WKWebView has them with Safari 17.5 or later, which macOS 13 reaches when updated. WebKitGTK 2.44 or later should match WebKit's 17.4/17.5 features (inference; verify).
- They suit popovers, toasts, notices and the close button fading in from `display: none` or `visibility`. Without support the element simply appears, so the fallback is free.

---

## 4. What would make the desktop app feel native and delightful

Motion budget, from Fluent ([Timing and easing](https://learn.microsoft.com/en-us/windows/apps/design/motion/timing-and-easing)):

- 83 ms (`ControlFasterAnimationDuration`) for hover and pressed colour changes.
- 167 ms (`ControlFast`) for small enter and exit motion.
- 250 ms (`ControlNormal`) for larger moves.
- Easing: "Fast Out, Slow In" `cubic-bezier(0,0,0,1)` for entering, `cubic-bezier(1,0,1,1)` for exiting.

macOS uses about the same order of magnitude: menus fade out in about 0.2 s, and sheets and popovers take about 0.25 s. Today the tokens define no easing and no short durations (`tokens.css:70-73` has only toast, pill fade, progress cycle and tooltip delay), and **no hover in the shared UI has a transition**: `transition` appears only in `phone.css:28`.

- **Hover and press.** Add `--paper-duration-hover` (about 80 ms) and `--paper-ease-out`, and put a background and colour transition on buttons, tabs, rows and menu items, gated by motion. Add `[data-pressed]` styles everywhere (React Aria supplies them). Per Microsoft, hover feedback is the element's job, not the cursor's.
- **Window open and close.** Set `background_color` from the resolved theme (Rust knows the preference) or keep the window hidden until the first frame, then `show()`, as the cold-start gate already marks it (`main.tsx:66-68`). This removes the white flash. Leave the OS's own window animations alone: macOS zoom, Windows' DWM fade and GNOME's open animation all apply to native windows for free.
- **Tabs.**
  1. Keep the strip's height reserved, or animate it in (height plus fade, 167 ms), so a second tab doesn't jump the layout.
  2. New tabs grow from zero width, and closed tabs shrink away (FLIP with `transform`, or `@starting-style` for the entry).
  3. **Live reorder:** use pointer events with `setPointerCapture` for the drag within the strip, and slide the neighbours with transforms.
  4. Keep Rust's cursor polling for leaving the window, and make a drop outside the window *open the new window at the cursor without the OS's snap-back*. A pointer-driven drag has no OS drag image, so no snap-back and no "not allowed" cursor over the other window.
  5. The pointer-driven drag also fixes the WebView2 HTML5 problem without turning off Rust's file drops.
  6. Later, like Chrome and Edge, detach into a real window that follows the cursor while the button is still down.
  7. Fade the close button in. Middle-click closes. Keep widths frozen during repeated closes. Scroll the active tab into view. Fade the cross-window drop border (`app.css:61-68`) in and out.
- **Drag-out feedback.** While a tab is over another window, highlight that window's *tab strip* with an insertion marker where the tab will land, rather than an inset border round the whole window. Over empty desktop, show a "new window" preview (a small translucent ghost window, or just the tab's chip under the cursor).
- **Scrollbars.** Keep the native webview scrollbars for the document. WKWebView draws macOS overlay scrollbars that follow System Settings, and WebKitGTK draws GTK-like ones. Don't hide or restyle them on desktop. WebView2 draws Chromium's scrollbars, not WinUI's thin overlay ones. A `scrollbar-color`/`scrollbar-width: thin` pair from the tokens is the only safe tweak (Chromium 121+). Give the hidden tab-strip scroll (`document-tabs.css:26`) edge fades or chevrons so the overflow is visible.
- **Title bar.** On macOS, a transparent title bar with the strip and top bar under the traffic lights is the big native win: Tauri has `title_bar_style(TitleBarStyle::Overlay)`, `hidden_title(true)`, `traffic_light_position` and `data-tauri-drag-region`. Expose it to the UI as a capability (for example `titleBarInset: { start: px }`) and a CSS variable, never as a platform check (ADR 0010). On Windows, keep the native frame. A custom caption loses Snap Layouts unless done carefully, so treat it as a later option. On Linux, keep server-side decorations. In the title, use the document's name alone on macOS (in Rust, in `windows.rs:208`, which ADR 0010 allows). Proxy icon and edited dot need AppKit calls (later).
- **Inactive windows.** Emit focus changes to the webview, set `:root[data-window-inactive]`, and grey the accent fills and the active tab, as AppKit does.
- **Kill the web tells.** Suppress the default `contextmenu` outside text fields and selections (WebView2 `AreDefaultContextMenusEnabled = false` through wry, and `preventDefault` in JS). Set `overscroll-behavior: none` on `html` and `body`. Turn off WebView2's browser accelerator keys (F5, Ctrl+P, Ctrl+F, Ctrl+R) that the app doesn't own. Apply `user-select: none` to chrome.
- **Settings.** Give Settings its own window on macOS (⌘,), or at least a sheet-like entrance. Bring the Appearance section, with Reduce motion, to the desktop.

---

## Recommendation: the cursor rule

**One rule for both shells, in tokens, with no platform check:**

- `default` (the arrow) for every action control: buttons, link-styled buttons, tabs, menu items, switches, list rows.
- `pointer` only for real navigation links (`href`) and PDF link annotations.
- `text` only over selectable text, with `user-select: none` on chrome labels.
- `col-resize`/`row-resize` for splitters, `grab` → `grabbing` for movable content, `progress` only without other busy feedback, `default` for disabled controls.

This matches the Apple HIG, Microsoft (both guides), GTK, the CSS UI spec, Radix, Spectrum and Tailwind v4, and Lamina's own `button.css:12`. Fix the outliers: `recent.css:43`, `apps/desktop/src/app.css:40`, `threads.css:54,76` and `phone.css:66`. Add a lint rule that bans a literal `cursor: pointer` outside link selectors.

## Top 6 desktop problems

1. **Tab drag runs on HTML5 drag and drop, which Tauri disables on Windows while its file-drop handler is on.** `document-tabs.tsx:289-293` against `windows.rs:159-176` (no `disable_drag_drop_handler`) and Tauri's own note at `webview_window.rs:1096`. Reorder and drag-out may not work on Windows at all, and no e2e test covers them. On macOS and Linux the same mechanism gives contradictory feedback: a "not allowed" cursor over a window that shows "drop here", and a snap-back before a successful move. Fix: a pointer-driven tab drag.
2. **No motion or press states anywhere in the chrome.** There are no hover transitions (only `phone.css` has any), no `[data-pressed]` styles on the tabs, close button or rows, and no enter or exit for popovers, toasts, notices or the close button. The strip appearing at two tabs jumps the layout by about 34 px (`use-tabs.tsx:116-118`). There is no live reorder, insertion marker or open/close animation for tabs.
3. **Wrong cursors and selectable chrome.** `pointer` on the recent rows (`recent.css:43`), on link-styled action buttons (`app.css:40`) and on the thread toggles. No `user-select: none`, so labels show the I-beam and can be selected.
4. **Reduced motion is unreliable on Linux, and the desktop has no in-app switch.** WebKitGTK on GTK 3 under Wayland ignores GNOME's setting (GTK #5813) and doesn't update live, and the desktop Settings show only Files (`settings.tsx:60-66`), with no Appearance or Reduce motion. Every new animation needs both gates.
5. **Web tells in the window.** The webview's default context menu (WebView2: Back, Refresh, Save as, Print), possible white flash on window open in dark mode (no `background_color`, `windows.rs:162-176`), no `overscroll-behavior`, no inactive-window styling, and the title "name — Lamina PDF" on macOS.
6. **The two-bar title area and hidden overflow.** The native title bar sits above a separate tab strip (no overlay title bar on macOS, which needs a capability flag for the inset). The strip hides its scrollbar and never scrolls the active tab into view (`document-tabs.css:26`). And `.desktop-missing` has no CSS at all (`file-changes.tsx:148`), so a restored tab whose file is gone shows an unstyled notice.
