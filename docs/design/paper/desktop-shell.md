# Paper: Desktop shell

Screens S01–S10 of the [screen inventory](screens.md): what the Tauri desktop app adds around the shared reader — the start window, document tabs and session restore, native menus on macOS and Windows/Linux, a file that changes on disk, printing, updates, the settings window, recovery after a crash, the download page on the web, and the Windows window frame. Everything follows the [Paper design language](design-language.md); new parts are listed under [New components](#new-components).

- **Canvas:** [lamina Paper — Desktop shell](https://claude.ai/artifact/RMvUvGA8RAG3AHmdCZYnfw) (private until the owner shares it).
- **Plan:** [desktop app](../../plans/desktop-app.md), rounds [D3](../../plans/desktop-app.md#d3-files), [D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing), [D5](../../plans/desktop-app.md#d5-update-service) and [D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta); layout rules from the [UI/UX plan](../../plans/ui-ux.md) §4–§5.
- **Sample files:** `Q3-board-report.pdf` (the shared sample, 24 pages) plus fictional neighbours in the recent list and tabs: `Damp-survey-North.pdf`, `Tenancy-agreement-template.pdf`, `Contractor-framework-2026.pdf`, `Surveyor-rota-October.pdf`, `Rent-arrears-policy.pdf`, `Asset-register-Q3.pdf`, `Q2-board-report.pdf` (missing) and `Annual-accounts-2025.pdf` (missing). Versions: installed `0.1.2` on the Beta channel, update `0.2.0`.

Conventions on this canvas:

- **Frames.** macOS boards draw the window frame from design language §6.1 (title bar 30 px, three 12 px controls, radius 10). The Windows boards (S03 right, S10) draw a 32 px title bar with a small document glyph, the title left-aligned and three 46×32 caption buttons (minimise, maximise, close) drawn as plain 1 px strokes, radius 8. The Linux board (S07 Advanced) draws a 36 px title bar with a centred title and one round close button. These are schematic, not copies of any OS’s assets.
- **Window title** is `<file name> — lamina` ([D4a](../../todos/ready/D4a-windows-tabs-and-session-restore.md)); with no document, `lamina`.
- **Native menus and the system print dialog are drawn by the OS.** The boards draw them schematically in Paper tokens to show structure, labels, shortcuts and states; real colours, fonts and spacing come from the system.
- **Command ids** use the registry’s `area.verb-object` form ([U1b](../../todos/ready/U1b-command-registry-and-palette.md)). Every command is in the palette and, per [D4b](../../todos/ready/D4b-native-menus.md), in a native menu; only those shown with a shortcut have one.
- **Shortcuts.** macOS glyphs run together (`⇧⌘N`), Windows/Linux use words (`Ctrl Shift N`). The desktop app owns some keys the web app must avoid (design language [§8.8](design-language.md#88-keyboard-and-shortcuts) restricts `Mod W/T/N/Q` in the web app only): `⌘W`/`Ctrl W` close a tab and `⌘Q`/`Ctrl Q` quit, as D4b specifies, because no browser sits around the desktop window. lamina never binds `⌘N`/`⌘T` (New window is `⇧⌘N`; there is no empty tab to create) or `Mod G`.

## S01 Start window

**Purpose.** What a window shows when no document is open: open a file, drop one, or go back to a recent file at the page you left.

**Entry points.** Launching lamina with “Reopen last session” off or nothing to restore; closing the last tab on Windows/Linux keeps the window only if it was the last window (otherwise the window closes); on macOS, clicking the Dock icon with no windows (`RunEvent::Reopen`) or File › New window (`window.new`, `⇧⌘N` / `Ctrl Shift N`).

**Layout.** Window frame; a slim top bar with “No document open” in place of the file name, Commands (`⌘K`) and More — no mode switcher, page or zoom (nothing to act on). No rail, no inspector. The canvas region uses `ground`, not `gutter` (there are no pages). Status bar: network indicator and the version and channel (“lamina 0.1.2 · Beta”).

**Content and controls.**

| Control | Does | Command id |
| --- | --- | --- |
| “Open a PDF” (Display 40) and one sentence | Says the two ways in | — |
| Open file… (primary) + “or press ⌘O” hint | Opens the system file dialog (multiple selection); each file opens in a tab | `file.open` (`⌘O` / `Ctrl O`) |
| Drop hint (dashed `control-border` box) | Decorative hint; the whole window is the drop target | — |
| Recent files grid | One card per entry (max 20): thumbnail of the last page viewed, file name (Newsreader 15, ellipsis), folder name (not the full path), “Page 3 of 24 · Today, 09:40”. Clicking opens the file at that page and zoom | `file.open-recent` |
| Missing entry | Dashed placeholder with the warning glyph, name in `muted`, “File not found”, **Remove** | `file.remove-recent` |
| Clear list | Empties lamina’s list and lamina’s entries in the OS recent list (Jump List on Windows) | `file.clear-recent` |
| Privacy line | “Recent files are kept on this computer only. Turn them off in Settings › General.” | `app.settings` |

**States.**

- **Recent files** (S01-start): six entries, one missing.
- **First run** (S01-start-empty): empty state “No recent files yet / Files you open appear here, with the page you were reading.” and a quiet notice “lamina is not your default PDF app.” with **Not now** and **Open system settings…** (plan open question 5: lamina never makes itself the default silently). “Not now” hides the notice for good; Settings › General keeps the button.
- **Remember recent files off:** the Recent section is replaced by one line: “lamina is not keeping recent files. Turn this on in Settings › General.”
- **Dragging files over the window** (not drawn): a full-window overlay on `scrim` with a centred `surface` card, the drop icon and “Drop to open 2 PDFs”; non-PDF files say “Only PDF files open in lamina” and dropping them shows a toast with the file name only ([D3a](../../todos/ready/D3a-open-from-os.md)). The same overlay appears over a document window (drop opens new tabs).
- **Unreadable file** from the list or a drop: toast “Couldn’t open Annual-accounts-2025.pdf. It may have been moved.” plus the missing state on its card.

**Behaviour.** Opening routes through `route_open` like every OS open: a file already open focuses its tab instead (unless duplicates are allowed). Double-clicking a PDF in Finder/Explorer/Files while lamina runs opens it in the front window as a new tab; the start window becomes that tab. Nothing in this window changes a file.

**Accessibility.** Focus order: top bar → Open file… → recent cards (one tab stop per card, arrow keys move in the grid) → Remove buttons → Clear list → privacy link → status bar. Cards are buttons named by their text (“Q3-board-report.pdf, Board papers, Page 3 of 24, Today 09:40”); Remove has `aria-label="Remove Annual-accounts-2025.pdf from recent files"`. Drag and drop always has the Open file… alternative. Each card’s context menu (Shift+F10 or right-click): Open, Show in Finder / Show in folder, Remove from list.

**Platform differences.** macOS: the app keeps running with no windows; the Dock menu and File › Open recent list the same files. Windows: the Jump List mirrors the list; closing the start window quits when it is the last window. Linux: the list is also written to `recently-used.xbel`. Web: the equivalent is R01 (home), which keeps recent files in OPFS and shares the card component, not the storage.

**Artboards:** `S01-start.dc.html`, `S01-start-empty.dc.html`.

**Todos:** [D3a](../../todos/ready/D3a-open-from-os.md), [D3b](../../todos/ready/D3b-recent-files-and-file-watching.md).

## S02 Window with document tabs and session restore

**Purpose.** Many documents per window: a tab strip under the title bar, an overflow menu when tabs don’t fit, and a notice after the last session comes back.

**Entry points.** Opening a second file in a window (the strip appears from two tabs; with one tab it is hidden and the title bar carries the name); relaunching with “Reopen last session” on.

**Layout.** Window frame → **tab strip** (34 px, on the title-bar colour) → top bar → restore notice (only after a restore) → rail · canvas · inspector → status bar. The active tab joins the top bar visually (it takes `ground` and drops its bottom rule).

**Content and controls.**

| Control | Does | Command id |
| --- | --- | --- |
| Tab | Activates the document. Label 13 px, ellipsis; active tab `text` 600, others `muted`, hovered tab `hover` fill | `tab.activate` |
| Close (24×24, 14 px cross) | Shown on the active and hovered tab and on keyboard focus; closes the tab (asks first if there are unsaved changes, O03) | `tab.close` (`⌘W` / `Ctrl W`) |
| “+3” overflow button | Appears when tabs don’t fit at 120 px; opens a menu of all tabs in this window with each tab’s page, the active one checked, missing files flagged “Not found”, and Move tab to new window | `tab.list` |
| + (Open a file in a new tab) | File dialog; files open as tabs here | `file.open` |
| Drag a tab | Reorders; dropping outside the window detaches it into a new window at the drop point; dropping on another window’s strip moves it there, keeping page and zoom | `tab.move-to-new-window`, `tab.move-to-window` |
| Restore notice | `role="status"` strip on `surface`: “Restored your last session. 8 tabs, each at the page you were reading. 1 file was not found.” · **Show missing file** (activates it) · Dismiss | `session.show-missing` |
| Next / previous tab | Keyboard and Window menu | `tab.next` (`⌃Tab`, `⇧⌘]` / `Ctrl Tab`), `tab.previous` (`⌃⇧Tab`, `⇧⌘[` / `Ctrl Shift Tab`) |

**States.**

- **Tabs + overflow menu open** (S02-tabs): five tabs visible, menu listing all eight.
- **Restored tab whose file is missing** (S02-tab-missing): the tab shows the warning glyph; the canvas holds an empty state “Q2-board-report.pdf was not found / It was in ~/Documents/Board papers when you last had it open. It may have been moved, renamed or deleted. / When you locate it, lamina opens it at page 9, where you left it.” with **Close tab** and **Locate…** (primary). Rail toggle disabled, no mode switcher, page or zoom. On macOS a renamed file is usually found through its bookmark and never shows this state.
- **Encrypted document moved to another window:** no state change — it stays unlocked ([D4a](../../todos/ready/D4a-windows-tabs-and-session-restore.md)).
- **Remember recent files off:** no session is stored; relaunch shows S01.

**Behaviour.** Session state is saved on quit and every 30 s. Restore reopens windows with their geometry, tabs in order, and each tab’s page, scroll offset and zoom. The notice disappears on Dismiss or after the first navigation; it never blocks reading. Locate… opens a file dialog filtered to PDFs; picking a file with a different size and date asks “This file looks different from the one you had open. Open it anyway?”.

**Accessibility.** The strip is `role="tablist"` with roving tabindex: one Tab stop, arrow keys move, Home/End jump, Delete closes the focused tab. Close buttons are named “Close <file name>”. The overflow button is `aria-haspopup="menu"` with `aria-label="All tabs, 8 open"`; its menu uses `menuitemradio`. Dragging has menu alternatives (Move tab to new window, Move tab to window ›). The restore notice is announced once (`role="status"`).

**Platform differences.** macOS: closing the last tab closes the window but the app stays running; the Window menu lists windows and tabs. Windows/Linux: closing the last tab of the last window shows S01 (closing the window then quits). Wayland may place detached windows where the compositor chooses. Web: no tabs (one document per browser tab).

**Artboards:** `S02-tabs.dc.html`, `S02-tab-missing.dc.html`.

**Todos:** [D4a](../../todos/ready/D4a-windows-tabs-and-session-restore.md).

## S03 Native menus

**Purpose.** Every command has a visible home in a native menu, generated from the command registry so menus, palette and shortcut help never disagree (UI/UX principles 3 and 4).

**Entry points.** The macOS menu bar; on Windows/Linux the in-window menu bar (Alt or F10 focuses it; Alt + the underlined letter opens a menu).

**Layout.** macOS (S03-menus-macos): system menu bar above the window with **lamina · File · Edit · View · Go · Tools · Window · Help**; File open with **Open recent ›** expanded. Windows/Linux (S03-menus-windows): a 28 px menu bar between the title bar and the top bar, access keys underlined; **View** open with **Page layout ›** expanded.

**Content.** Labels follow the design language’s sentence case (the registry’s titles), not the Title Case of the macOS HIG; see open points.

| Menu | Items (shortcut macOS / Windows-Linux) |
| --- | --- |
| lamina (macOS only) | About lamina · Check for updates… · Settings… (`⌘,`) · Services · Hide lamina (`⌘H`) · Hide others · Show all · Quit lamina (`⌘Q`) |
| File | Open… (`⌘O` / `Ctrl O`) · Open recent › (files, Clear menu) · New window (`⇧⌘N` / `Ctrl Shift N`) · Close tab (`⌘W` / `Ctrl W`) · Close window (`⇧⌘W` / `Ctrl Shift W`) · Export a copy… · Save (`⌘S` / `Ctrl S`, editor; disabled with no changes) · Print… (`⌘P` / `Ctrl P`) · Quit (`Ctrl Q`, Windows/Linux only) |
| Edit | Copy · Select all (predefined) · Find… (`⌘F` / `Ctrl F`) · Find next · Find previous (no shortcut: `Mod G` is avoided; Enter and Shift+Enter work in the find bar) · Settings… (`Ctrl ,`, Windows/Linux only) |
| View | Zoom in (`⌘+` / `Ctrl +`) · Zoom out (`⌘−` / `Ctrl −`) · Actual size (`⌘0` / `Ctrl 0`) · Fit width · Fit page · Sidebar (check) · Focus mode (`⌘\` / `Ctrl \`, check) · Page layout › (Single page, Continuous, Two pages) · Page appearance › · Theme › · Full screen (macOS predefined `⌃⌘F`; no shortcut on Windows/Linux) |
| Go | Next page · Previous page · First page · Last page · Go to page… · Back (`⌘[` / `Alt ←`) · Forward (`⌘]` / `Alt →`) |
| Tools | Commands… (`⌘K` / `Ctrl K`) · Find in document · Document properties… · Network activity · Reader mode |
| Window | Minimise (`⌘M`, macOS predefined) · Zoom (macOS) · Next tab · Previous tab · Move tab to new window · Move tab to window › · list of windows |
| Help | lamina help · Keyboard shortcuts · Report an issue · View licences · Check for updates… and About lamina (Windows/Linux only) |

**States.** Items without a document are disabled (Print, Close tab, Export a copy, zoom, Go). Zoom in/out disable at the zoom limits. Check items reflect the focused window (Sidebar, Focus mode, page layout). Open recent rebuilds when the recent list changes and is empty (“No recent files”, disabled) when the setting is off.

**Behaviour.** Choosing an item emits the command id to the focused window, which runs it through the same registry as the keyboard and palette. When a menu accelerator exists, the web view does not also handle that key (no double zoom steps on Windows/Linux). Command ids: `file.open`, `file.open-recent`, `file.clear-recent`, `window.new`, `tab.close`, `window.close`, `file.export-copy`, `file.save`, `file.print`, `app.quit`, `app.settings`, `app.check-for-updates`, `app.about`, `edit.copy`, `edit.select-all`, `find.open`, `find.next`, `find.previous`, `view.zoom-in`, `view.zoom-out`, `view.actual-size`, `view.fit-width`, `view.fit-page`, `view.toggle-sidebar`, `view.focus-mode`, `view.page-layout`, `view.page-appearance`, `view.theme`, `view.full-screen`, `nav.next-page`, `nav.previous-page`, `nav.first-page`, `nav.last-page`, `nav.go-to-page`, `nav.back`, `nav.forward`, `palette.open`, `doc.properties`, `network.show`, `reader.open`, `tab.next`, `tab.previous`, `tab.move-to-new-window`, `tab.move-to-window`, `help.open`, `help.shortcuts`, `help.report-issue`, `help.licences`.

**Accessibility.** Native menus are accessible through the OS. The schematic drawing uses `role="menubar"`, `menu`, `menuitem`, `menuitemcheckbox`, `menuitemradio` and `aria-keyshortcuts`; the highlighted item (accent fill, white text) is the keyboard focus. Windows/Linux access keys are unique per menu.

**Platform differences.** macOS: one app-wide menu bar; app menu holds About, Settings, Quit; Check for updates sits in the app menu. Windows: menu bar per window; Settings at the end of Edit; About and Check for updates in Help. Linux: the GTK menu bar is visible by default in each window (same placement as Windows). Web: the More menu and the palette carry the same commands; browser-reserved keys are not used.

**Artboards:** `S03-menus-macos.dc.html`, `S03-menus-windows.dc.html`.

**Todos:** [D4b](../../todos/ready/D4b-native-menus.md).

## S04 File changed on disk

**Purpose.** Keep the window in step with the file when another app saves, replaces, moves or deletes it, and never lose the user’s unsaved work silently.

**Entry points.** The file watcher (no user action). The setting “Reload when the file changes on disk” (Settings › General, on by default).

**Layout.** A strip under the top bar (and secondary toolbar, if any), like the signed-file banner in design language [§7.13](design-language.md#713-save-note-and-save-banner-signed-file): `warning-tint`, `hairline` bottom, warning glyph, lead sentence in `warning` 600, actions at the right. The page stays readable below.

**Content and controls.**

| Case | What shows | Controls and commands |
| --- | --- | --- |
| Changed, **no** unsaved changes (reader default) | Silent reload keeping page and zoom, then a toast “Reloaded: Q3-board-report.pdf changed on disk” (`role="status"`, 4 s). If the page count shrank below the current page, the last page shows | — (`file.reload-from-disk` runs automatically) |
| Changed **with** unsaved changes (S04-file-changed) | “**Q3-board-report.pdf changed on disk.** Another app saved a new version at 10:12 while you had 1 unsaved change here.” Second line: “Reload shows the new version and discards your change. Keep mine leaves your view as it is; saving then replaces the other version.” | **Compare** (`file.compare-with-disk`) · **Keep mine** (`file.keep-mine`) · **Reload from disk** (`file.reload-from-disk`) |
| Moved or deleted (S04-file-missing) | “**File was moved or deleted.** lamina is still showing the version it opened. Locate the file to keep it linked, or close the tab.” | **Close tab** (`tab.close`) · **Locate…** (`file.locate`) |
| Changed but unreadable (a build still writing) | Notice “File changed but could not be read; retrying.” in the same strip, no buttons; it clears on the next good reload | — |

**States.** Conflict with unsaved changes (drawn, S04-file-changed) · moved or deleted (drawn, S04-file-missing) · silent reload with toast · unreadable, retrying · setting off (banner with **Reload** · **Ignore** for every change) — each described in the table above and under Behaviour.

**Behaviour.**

- **Compare** opens the on-disk version in a new read-only tab beside the current one (“Q3-board-report.pdf — on disk”), so the user can read both; the banner stays until Keep mine or Reload. A visual diff is out of scope here.
- **Keep mine** dismisses the banner; the top bar keeps “1 unsaved change”. The next Save writes a full save (not incremental, since the base changed) and the save note says so: “The file changed on disk. Saving replaces that version.” (O02 owns the wording.)
- **Reload from disk** discards the op log for this document after a confirm only if there are more than 20 changes; undo history starts fresh.
- Moved/deleted: lamina keeps serving the open handle, so reading continues; Locate… relinks to the chosen path; nothing is written.
- With the setting off, the banner appears for every change (even without unsaved changes) with **Reload** and **Ignore**.

**Accessibility.** The conflict strip is `role="alert"` (it asks for a decision); focus is not moved into it, but F6 reaches it first after the top bar. Buttons have plain labels; the second line is the description of the group.

**Platform differences.** Same on every OS; the watcher handles atomic saves (rename over the file) as a change, not a delete. Web: files opened from disk can’t be watched; no S04 states.

**Artboards:** `S04-file-changed.dc.html`, `S04-file-missing.dc.html`.

**Todos:** [D3b](../../todos/ready/D3b-recent-files-and-file-watching.md).

## S05 Print

**Purpose.** Print faithfully through the system print dialog, with lamina only reporting progress and offering a fallback.

**Entry points.** File › Print… (`file.print`, `⌘P` / `Ctrl P`), the palette, More › Print. Disabled with no document.

**Layout.** S05-print: the system dialog (drawn schematically as a sheet under the macOS title bar over `scrim`): page preview with its own pager on the left; Printer, Copies, Two-sided, Pages (All 24 pages · Current page (3) · Pages [5–12], “8 pages”), Scale (Fit to paper · Actual size), and the line “Orientation follows each page. Prints the page as it looks in Original appearance, with comments.”; Cancel · Print. S05-print-progress: back in the window, the status bar’s middle shows the printer glyph, “Printing pages 5–12 · 3 of 8 sent”, a 140 px determinate bar and **Cancel printing**.

**Content and controls.** `file.print` opens the system dialog with the current page and page count; `print.cancel` (**Cancel printing**, ghost text button in the status-bar item); `print.compatibility` (the toast action “Try compatibility printing” after a native failure); Settings › Advanced › Printing (`print.mode`: System printing / Compatibility printing, drawn in S07-settings-advanced). The dialog’s own controls (printer, copies, pages, scale) belong to the OS; lamina only pre-fills them.

**States.**

- **System dialog open** (OS-owned; lamina passes the current page and the document’s page count).
- **Spooling** (drawn): non-modal; reading and scrolling continue.
- **Cancelled:** the item disappears; toast “Printing cancelled”.
- **Done:** the item disappears; toast “Sent 8 pages to Office printer, first floor”.
- **Native path failed:** notice in the status-bar slot replaced by a toast with action: “Couldn’t print with the system printer path. Try compatibility printing” → runs the web-view print path once; Settings › Advanced can make it the default.
- **Page appearance not Original or Dark theme:** prints in Original; the dialog line says so (no separate warning, unlike the web R09 where the browser dialog can’t carry it).

**Behaviour.** Pages render in the sandboxed engine at the printer’s resolution, one at a time; Cancel stops rendering within a second. No headers, footers or margins are added. Printing never changes the file.

**Accessibility.** The progress bar is `role="progressbar"` labelled by its text; Cancel printing is a real button reachable with F6 → status bar. Completion is announced by the toast.

**Platform differences.** The dialog differs per OS (macOS sheet, Windows print dialog, GTK print dialog on Linux) and offers what the printer supports (duplex, collation). Web: R09 uses the browser’s print with a lamina pre-print panel.

**Artboards:** `S05-print.dc.html`, `S05-print-progress.dc.html`.

**Todos:** [D4c](../../todos/ready/D4c-native-printing.md).

## S06 Updates

**Purpose.** Keep lamina up to date without interrupting reading, and let the user (or an administrator) control channel and checks.

**Entry points.** Automatic (10 s after start, then daily); lamina › Check for updates… (macOS) or Help › Check for updates… (Windows/Linux) (`app.check-for-updates`); Settings › Updates.

**Layout.**

- **Document windows** (S06-update-ready): one status-bar item at the right, “Update ready”, `accent` 600 on `accent-tint`, with the download glyph. It appears only once the update is downloaded and verified. Clicking opens a small non-modal popover above it: “lamina 0.2.0 is ready” (Title), “Beta channel · you have 0.1.2”, “It installs the next time you quit. Restart now to install it; your windows and tabs come back where you left them.”, **What’s new in 0.2.0** link, **Later** · **Restart to update**.
- **Settings › Updates** (S06-update-settings): status card (“Downloading lamina 0.2.0”, 62% bar, “You have 0.1.2. The new version installs when you quit, or when you choose Restart to update.”), Automatic updates switch, Channel select (Beta, Stable), Last checked + **Check now**, a nightly note with a link, and what each check sends.

**Content and controls.** `app.restart-to-update`, `app.update-later` (closes the popover; the item stays), `app.check-for-updates`, `settings.update-enabled`, `settings.update-channel`, `help.whats-new`.

**States.**

| State | Settings card | Status bar |
| --- | --- | --- |
| Up to date | “lamina 0.1.2 is up to date” · Check now | nothing |
| Checking | “Checking for updates…” indeterminate bar; Check now disabled | nothing |
| Downloading (drawn) | determinate bar and % | nothing |
| Ready | “lamina 0.2.0 is ready” · **Restart to update** (primary) | “Update ready” |
| Error (only after Check now) | `danger-tint` notice “Couldn’t reach the update server. lamina will try again tomorrow.” | nothing (automatic failures are silent) |
| Updates off | card says “Automatic updates are off”; Check now still works | nothing |
| Managed by organisation | controls disabled; badge “Managed by your organisation” and the pinned channel | as policy allows |
| Nightly build | Channel shows “Nightly” read-only with “Nightly builds are a separate install” | as usual |

**Behaviour.** Downloads run in the background and are signature-checked before “ready”. Restart to update with unsaved changes first shows the close dialog (O03) for each document with changes. The update check counts in the network indicator: “1 network request · to the update server only”. Turning updates off stops every update request.

**Accessibility.** The status-bar item is a button with `aria-haspopup="dialog"` and `aria-expanded`; the popover is a non-modal `role="dialog"` labelled by its title, Escape closes it and returns focus. Progress uses `role="progressbar"`; reaching “ready” is announced politely once.

**Platform differences.** Same UI on every OS; installers differ (Windows passive installer; AppImage replaces itself; deb/rpm installs show “Updates come from your package manager” and hide the switch, as only the AppImage self-updates). Web: R15 shows “Update available” for the offline app.

**Artboards:** `S06-update-ready.dc.html`, `S06-update-settings.dc.html`.

**Todos:** [D5b](../../todos/ready/D5b-updater-in-app.md).

## S07 Settings window

**Purpose.** One place for every desktop setting the beta needs. Settings apply immediately (no Save button), so switches are used for on/off settings (design language §7.19).

**Entry points.** lamina › Settings… (macOS) or Edit › Settings… (Windows/Linux), `⌘,` / `Ctrl ,` (`app.settings`); links from S01, S04, S06 and S08. A single window (label `settings`); opening it again focuses it.

**Layout.** Its own window, 880 × ≥ 640 px, resizable. Left: a 200 px `ground` list of sections in a `<nav aria-label="Settings sections">` (links with `aria-current="page"` on the open section, `accent-tint` fill); right: the section on `surface`, padding `28px 32px`, a Title (Newsreader 20) and groups with Heading (Newsreader 18). Rows: label (14) and help (12 `muted`) on the left, the control right; rows separated by `hairline`.

**Sections and controls.**

| Section | Controls (setting key) | Source |
| --- | --- | --- |
| General (drawn) | Open files in: A new tab in the front window / A new window (`open.target`) · Reopen last session (`session.reopen`) · Reload when the file changes on disk (`watch.reload`) · Allow the same file in two tabs (`tabs.allow-duplicates`) · Remember recent files (`recent.enabled`) · Clear recent files · Default PDF app: Open system settings… | D3b, D4a, D7b |
| Appearance (drawn) | Theme chips: Match system, Light, Dark, High contrast · Page appearance chips (§7.25): Original, Sepia, Dark, High contrast · When a document opens: Zoom (Fit width, Fit page, Actual size, Last used for the file), Page layout (Continuous, Single page, Two pages), Show the sidebar | C7a, U2c |
| Updates | See S06 | D5b |
| Privacy (drawn) | Send crash reports (off) + See a sample report · Share how often each command is used (off) + “Exactly what would be sent” disclosure with the payload · Reset anonymous ID · Read the privacy policy | D7a, U5c, D5b |
| Shortcuts (drawn) | Single-key shortcuts (on) · Filter shortcuts · the platform’s shortcut table generated from the registry, grouped by menu; single-key shortcuts in their own group | U1b, C7a |
| Advanced (drawn, Linux frame) | Graphics (Linux only): Automatic / Compatibility / Safe mode, Restart lamina, `--safe-graphics` hint · Printing: System printing (recommended) / Compatibility printing | D7b, D4c |
| About | Version and channel (“lamina 0.1.2 · Beta”), build line (release or nightly), Copy version info, View licences, lamina help, the code licence line (MIT or Apache-2.0) | D7b |

**States.**

- **Dependent settings:** with Remember recent files off, Reopen last session is disabled with the help text “Needs Remember recent files”.
- **Restart needed:** changing Graphics shows “Takes effect after a restart” and enables **Restart lamina**.
- **Crash-loop fallback** (drawn): `warning-tint` notice “Graphics switched to Compatibility. lamina closed twice just after starting, so it turned off the faster renderer. Choose Automatic to try it again.”
- **Managed by organisation:** affected controls disabled with the badge “Managed by your organisation”.
- **Usage counts on:** the disclosure stays available; the network indicator lists the requests when they happen.

**Behaviour.** Every change is written at once (atomic write) and broadcast to open windows. Turning off Remember recent files clears the list and deletes the stored session; Clear recent files asks nothing (it can’t lose work) and shows a toast with **Undo** for 4 s. Reset anonymous ID creates a new random ID; toast “New anonymous ID created”.

**Accessibility.** Focus order: section list → section content top to bottom. Switches are `role="switch"` with `aria-checked`, labelled by the row label and described by its help text; radio groups are `fieldset`/`legend` or `role="radiogroup"`; chips are buttons with `aria-pressed`. The shortcut table has a caption and group header rows (`scope="colgroup"`). Single-key shortcuts can be turned off (WCAG 2.1.4).

**Platform differences.** macOS: frame with traffic lights, opened from the app menu. Windows: standard caption buttons; opened from Edit. Linux: Graphics group shown only here. The web app has no settings window; its View settings (R10) cover Appearance, and Privacy items live in the web app’s own settings panel.

**Artboards:** `S07-settings-general.dc.html`, `S07-settings-appearance.dc.html`, `S07-settings-privacy.dc.html`, `S07-settings-shortcuts.dc.html`, `S07-settings-advanced.dc.html` (and `S06-update-settings.dc.html` for Updates).

**Todos:** [D7b](../../todos/ready/D7b-settings-renderer-switch-and-download-page.md), [D7a](../../todos/ready/D7a-crash-reporting.md), [U5c](../../todos/ready/U5c-opt-in-usage-telemetry.md).

## S08 Crash recovered

**Purpose.** After lamina quits unexpectedly, bring the user’s work back safely and ask — once, without pressure — whether to send an anonymous report.

**Entry points.** The next launch after a main-process crash; the dialog appears over the start window before any tab reopens.

**Layout.** Dialog (§7.18) over `scrim`, 520 px: title “lamina quit unexpectedly”; “Choose which tabs to reopen. If one file seems to cause the problem, leave it closed.”; the session’s tabs as check boxes with their page; the tab that was open in the front when lamina quit is **unchecked** and badged “Open when lamina quit” (`warning` badge); below a hairline: **Send an anonymous crash report** (unchecked) with what it contains, **Always send reports after a crash** (enabled only once the first is checked), **View report**; actions **Start without tabs** · **Reopen 2 tabs** (primary; the count follows the check boxes). The report preview (S08-crash-report) replaces the dialog content at 600 px: the exact JSON in a scrollable, focusable block, green badges “No file names or paths”, “No document content”, “No anonymous update ID”, “Sent to lamina’s own server and deleted after 30 days.”, **Copy report** · **Don’t send** · **Send report**.

**Content and controls.** `session.reopen-selected`, `session.start-empty`, `crash.view-report`, `crash.send-report`, `crash.dont-send`, `crash.copy-report`, `settings.crash-reports-always`.

**States.**

- **Dialog** (drawn) — shown at most once per install for the report question; later crashes show only the tab choice while reporting is off.
- **Report preview** (drawn).
- **Reporting already on:** the report part reads “A crash report was sent.” with a link to Settings › Privacy.
- **No session to restore** (session off or empty): the dialog shows only the report part, titled the same.
- **Engine crash while running** (not a dialog): the document process restarts and the page reloads in place; toast “The document view restarted after a problem. You’re on page 3.”; the report question, if never asked, appears as a notice in the status bar, not a modal.
- **Offline:** Send queues the report (up to 10, dropped after 30 days); the dialog closes normally.

**Report preview details.** The preview opens from **View report** and replaces the dialog content; **Back to tabs** (top left) returns to the tab choice with its check boxes as they were. The JSON shows only D7a’s allow-listed fields (report ID, app version, channel, OS name and version, arch, process kind, renderer, sandbox level, uptime bucket, crashing-thread frames, module names and versions); a line under it says the report ID is random and made for this report only. **Copy report** is a ghost button left of **Don’t send** · **Send report**. Send report also ticks “Send an anonymous crash report” on the tab choice.

**Behaviour.** Reopening restores page and zoom per tab. Reports are built from an allow-list and scrubbed locally; nothing is sent unless Send or Always is chosen. Neither choice changes any document.

**Accessibility.** `role="dialog"` with `aria-modal`, labelled by the title and described by the first sentence; initial focus on the primary button; Escape = Start without tabs is **not** used (Escape keeps the dialog: it asks a required question). The report block is focusable (`tabindex="0"`) so keyboard users can scroll it.

**Platform differences.** Same on all OSes; report fields name the OS and process. Web: crash reporting is out of scope (ADR 0011).

**Artboards:** `S08-crash-recovered.dc.html`, `S08-crash-report.dc.html`.

**Todos:** [D7a](../../todos/ready/D7a-crash-reporting.md).

## S09 Download page (web)

**Purpose.** Get the right installer for the visitor’s computer, with everything needed to trust it.

**Entry points.** `<domain>/download`; links from the web reader’s More menu (“Get the desktop app”) and the site header.

**Layout.** Web page, no app shell: site header (wordmark, Web reader · Download · Privacy), a centred 960 px column: Display heading “lamina for your desktop” and one sentence; channel tabs **Beta** · **Stable · not released yet**; the recommended download card; **All downloads** table; four short sections (Check your download, System requirements, What the app sends, Opening it the first time); footer with the licence line and the nightly link.

**Content and controls.**

| Control | Does |
| --- | --- |
| Recommended card | “For your Mac with Apple silicon”, “lamina 0.1.2 beta · .dmg · [SIZE] MB · signed and notarised”, “Guessed from your browser. Not right? Every download is listed below.”, **Download for macOS** (primary link) and Copy SHA-256 |
| All downloads table | System · File (link) · Processor · Size · Copy SHA-256 per file; macOS dmg (Apple silicon, Intel), Windows setup .exe (x64, Arm) and .msi (“for managed installs”), Linux AppImage (“updates itself”), .deb, .rpm; Linux Arm files under “more files”; links to SHA256SUMS, SHA256SUMS.asc and the signing key |
| Check your download | `shasum -a 256 …` and `gh attestation verify … --repo [OWNER]/lamina` |
| System requirements | macOS 13+, Windows 11 (Windows 10 22H2 untested), Ubuntu 22.04+ or supported Fedora; x64 and Arm |
| What the app sends | Daily update check (can be turned off); crash reports and usage counts only if turned on; privacy policy link |

Sizes, hashes and the publisher name are placeholders (`[SIZE]`, `[PUBLISHER NAME]`, `[OWNER]`) filled from `downloads/{channel}.json` at runtime.

**States.** Detected OS (drawn: macOS arm64). Unknown OS or a phone: the card becomes “Choose your system below” with no primary download, and phones see “lamina for desktop runs on macOS, Windows and Linux. On this device, use the web reader.” Fetch of the release list failed: notice “Couldn’t load the download list. Try again, or get the files from the release page.” Stable not released: tab disabled with its reason in the label.

**Behaviour.** Copy buttons copy the full hash and show a toast “SHA-256 copied”. Nothing is tracked; the page makes one request for the release list to the downloads host (the network indicator design applies only inside the app).

**Accessibility.** Real links for downloads, buttons for copy with names that include the file (“Copy SHA-256 of lamina_0.1.2_aarch64.dmg”); the table has column headers and scrolls inside its own box below 640 px; channel tabs follow §7.5. Reflows to one column on a phone.

**Platform differences.** Web only. On a phone (not drawn) the card stacks above the table and the table scrolls horizontally.

**Artboards:** `S09-download.dc.html`.

**Todos:** [D7b](../../todos/ready/D7b-settings-renderer-switch-and-download-page.md).

## S10 Windows window frame

**Purpose.** Show the reader as it looks on Windows: the same Paper shell, a Windows frame and an in-window menu bar.

**Entry points.** Any document window on Windows (and, with the differences below, Linux). The board shows the moment after the user pressed Alt.

**Layout.** Title bar 32 px (document glyph, “Q3-board-report.pdf — lamina” left-aligned, Minimise · Maximise · Close at the right, 46×32 each) → menu bar 28 px (File, Edit, View, Go, Tools, Window, Help; drawn just after pressing Alt, so access keys are underlined and File has the focus ring) → tab strip (two tabs) → top bar with `Ctrl K` → Pages rail · page 3 → status bar. Read mode, no secondary toolbar.

**Content and controls.**

| Part | Content | Commands |
| --- | --- | --- |
| Title bar | Document glyph (accent), “Q3-board-report.pdf — lamina”; caption buttons Minimise · Maximise · Close (46×32, 10 px stroke glyphs) | OS window commands; Close = `window.close` (asks about unsaved changes first, O03) |
| Menu bar | File, Edit, View, Go, Tools, Window, Help with underlined access keys (Alt F, Alt E, Alt V, Alt G, Alt T, Alt W, Alt H); contents as S03 | the S03 menus |
| Tab strip | Q3-board-report.pdf (active, close button) · Damp-survey-North.pdf · Open a file in a new tab (+) | `tab.activate`, `tab.close` (`Ctrl W`), `file.open` |
| Top bar | Sidebar toggle, file name, “No unsaved changes”, mode tabs (Read selected), page 3 / 24, 110%, Commands `Ctrl K` | `view.toggle-sidebar`, `mode.read` … `mode.edit`, `nav.go-to-page`, `palette.open` (`Ctrl K`) |
| Body | Pages rail (page 3 current) and page 3 “2. Repairs backlog” | — |
| Status bar | Network indicator “0 network requests · Offline-safe: nothing uploaded”, “Page 3 · Repairs backlog” | `privacy.network-activity` |

**States.** Menu bar focused after Alt (drawn: File has the focus ring, access keys underlined) · at rest (access-key underlines hidden until Alt, per the OS setting) · a menu open (S03-menus-windows) · window maximised (radius 0, Maximise glyph becomes Restore: two offset squares) · inactive window (title text `muted`) · one tab (strip hidden, as on macOS).

**Behaviour.** Alt or F10 moves focus to the menu bar; arrows move between menus; Escape returns focus to the page. Caption buttons follow the OS (snap layouts on hover of Maximise are the OS’s). The title bar is the OS’s own (no custom-drawn chrome) so it behaves natively with accessibility tools.

**Accessibility.** Menu bar `role="menubar"` with `aria-keyshortcuts` per item; caption buttons have names (Minimise, Maximise, Close). The same F6 region cycle as macOS adds the menu bar as the first region.

**Platform differences.** Linux: frame drawn by the desktop (GNOME: centred title with a close button; KDE: its own controls), GTK menu bar below it, otherwise identical. macOS: no in-window menu bar (S03). Shortcut chips show `Ctrl` words.

**Artboards:** `S10-windows-frame.dc.html`.

**Todos:** [D4b](../../todos/ready/D4b-native-menus.md).

## New components

- **Document tab strip** (S02, S10). 34 px row on the title-bar colour `#EFEBE3`, `#E0D9CD` rule below. Tabs `flex: 0 1 220px`, min 120, padding `0 4px 0 12px`, 1 px `#E0D9CD` dividers; label 13 px (active `text` 600, others `muted`); active tab on `ground` with `margin-bottom: -1px` so it joins the top bar; hover `hover` fill. Close button 24×24 with a 14 px cross, shown on active, hovered and focused tabs. Overflow button “+N” with chevron (26 px, pressed = toggle-selected style) opening a tab menu; an add button (26 px, plus icon). `role="tablist"` with roving tabindex. Hidden with one tab.
- **Menu** (S02 overflow, S03 schematic native menus). `surface`, 1 px `chip-border`, radius 6, padding 4, menu shadow, z 30. Items 26–30 px, padding `0 10px` (34 px left indent when a check column exists), 14 px; shortcut as plain `muted` text right-aligned (not chips, matching native menus); highlighted item `accent` fill with white text; check items use the Check icon; separators 1 px `hairline` with 4 px/6 px inset; overline header optional. Submenu chevron: `<path d="M10 7l5 5-5 5"></path>` **(new icon: Chevron right)**.
- **Full-width conflict strip** (S04). The §7.13 banner pattern generalised: `warning-tint`, lead sentence in `warning` 600, optional second line 12 px `text-3`, actions right (ghost · secondary · secondary). `role="alert"` when it asks for a decision, `role="status"` when it informs (restore notice on `surface`).
- **Status-bar progress item** (S05) and **status-bar action item** (S06). Progress: glyph, label in `text`, 140×4 bar (§7.22), ghost text button. Action: `accent-tint` pill-free button (radius 4) with `accent` 600 label and icon, opening a non-modal popover (`surface`, `chip-border`, radius 6, menu shadow, padding 20, 340 px) anchored above the status bar.
- **Settings window layout** (S06, S07). 200 px section list on `ground` (32 px link rows, current = `accent-tint` + `accent` 600), content on `surface` with padding `28px 32px`; setting rows (label + 12 px help left, control right, 12 px vertical padding, `hairline` between).
- **Theme chips** (S07 Appearance). Same anatomy as §7.25 page-appearance chips, with a 52 px preview of the chrome ground and a small white page (Match system splits light and dark).
- **Crash report block** (S08) and **code sample** (S07 Privacy, S09). White, 1 px `hairline` (or `control-border` when focusable), radius 4, padding `10–12px`, system monospace stack `ui-monospace, 'SF Mono', Menlo, Consolas, monospace` at 12 px (no webfont), `pre` scrolls.
- **Drop hint** (S01). 1 px dashed `control-border` box on `surface`, radius 6, with the new **Download / drop** icon `<path d="M12 4.5V15M8 11l4 4 4-4"></path><path d="M5 15.5v4h14v-4"></path>` (also used for Update ready and the download button).
- **New icons:** Printer `<path d="M7 9V4.5h10V9"></path><rect x="4.5" y="9" width="15" height="7" rx="1"></rect><path d="M7 13.5h10v6H7z"></path>`; Document (Windows title bar) `<path d="M6 3.5h8l4 4v13H6z"></path><path d="M14 3.5v4h4"></path>`; Chevron right and Download as above. Window caption glyphs (minimise, maximise, close) are drawn on a 10×10 grid with 1 px strokes and are mockup-only.
- **Window frames for Windows and Linux** (mockups only): see Conventions above.

### Open points for review

- **Menu label case.** Sentence case follows design language §9 but differs from the macOS HIG (Title Case). Decide per platform when D4b lands; the registry can hold one title and the macOS menu builder can title-case it.
- **Go and Tools menus** are in the screen inventory but not in D4b’s list; their contents here are a proposal.
- **Close tab `⌘W` and Quit `⌘Q`** are desktop-only exceptions to the web collision list (see Conventions); the shortcut help must show them only in the desktop app.
- **Crash dialog merges two questions** (which tabs to reopen, whether to report) that D7a describes as one consent dialog; D7a’s wording (“Send”, “Don’t send”, “Always send”) is kept in the report preview.
- **Compare** (S04) opens a second read-only tab; a visual diff would need its own todo.

## Proposed design-language additions

Parts these boards use that design language does not yet define (none edits design-language.md; the owner decides):

- **Drawn check box and radio** (S07 Advanced, S08). 16 px; check box radius 3, radio round; off: white with `control-border`; on: `accent` fill with a white Check icon (check box) or an 8 px `accent` dot (radio); disabled: `ground` fill, `chip-border` edge, label `muted`. Built as `role="checkbox"`/`role="radio"` buttons with `aria-checked`, label inside the button so the whole row (≥ 24 px) is the target.
- **Option row with help** (S07 Advanced). Radio label 14 px, help 12 px `muted` indented 24 px under it, rows split by `hairline`; the group is a `role="radiogroup"` labelled by the section heading.
- **Disabled secondary button** (S07 Advanced “Restart lamina”). `ground` fill, `chip-border` edge, `muted` label, `aria-disabled="true"` with the reason as its description.
- **Status notice** in a settings section (S07 Advanced crash-loop fallback): the §7.21 notice in `warning-tint` with the Warning icon, title in `warning` 600.
- **Positive badge list** (S08 report preview): §7.23 positive badges with a Check icon, as a labelled list of what a report leaves out.
- **Linux window frame** (S07 Advanced, mockup only): 36 px title bar, centred 13/600 title, one 24 px round close button at the right on a 10% `text` fill.
- **Web site header and footer** (S09): 56 px header with the Newsreader 22 wordmark and three nav links (current = `accent` 600 with a 2 px underline); footer 12 px `muted` with the licence line. No app shell.
- **Large primary link** (S09 “Download for macOS”): 40 px high primary button styling on an `<a>`, with the Download icon.
