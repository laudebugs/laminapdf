# Paper: Reader A

Screens R01–R09 of the [screen inventory](screens.md): opening a file (home, URL, recent files), loading, the password prompt, repaired and damaged files, find, the rail tabs, links and history, document properties, and print and download. These are the web reader v0.1 screens; the desktop app shares the same viewer and differs only where noted. Everything follows the [Paper design language](design-language.md); new parts are listed under [New components](#new-components) and [Proposed design-language additions](#proposed-design-language-additions).

- **Canvas:** [lamina Paper — Reader A](https://claude.ai/artifact/MWpEwMqZSWUBYSMuCi1Fz2) (private until the owner shares it).
- **Plans:** [web reader §4 functional](../../plans/web-app.md#functional) and [privacy and security](../../plans/web-app.md#privacy-and-security); layout rules from the [UI/UX plan](../../plans/ui-ux.md) §4–§5.
- **Sample file:** `Q3-board-report.pdf`, Northwind Housing Trust, 24 pages, page labels `1`–`21` then `A-1`–`A-3` for Appendix A (pages 22–24), 3 attached files (1.3 MB, the same set as [O14](pages-navigation-extraction.md#o14-attachments-panel)). Other files (`Tenant-survey-2026.pdf` and `Asset-register-summary.pdf` in R01’s recent list, `Northwind-asset-survey-2026.pdf` on `R02-loading-download`, `Minutes-september.pdf` on `R04-unrecoverable`) and hosts (`files.example.org`, `www.northwind-housing.example`) are fictional and use reserved example domains. R03, R04-repaired and R09-print-restricted show variants of the sample file (encrypted, damaged, print-restricted); R08 shows the plain file.

Conventions on every board in this area:

- **Read-only reader top bar (v0.1).** The reader v0.1 ships Read only ([UI/UX §5.1](../../plans/ui-ux.md#51-desktop-and-web-layout-r1-r2)), so its top bar has **no mode switcher and no save state**; the editor adds both. Left: sidebar toggle, file name (with “Loading pages…” or “Locked” only while those apply). Right: Back · Forward (always present, disabled until there is history, drawn with `aria-disabled="true"` and the `disabled` colour; `nav.back`, `nav.forward`) · page input `n / N` · zoom −/%/+ · Find · **View** (icon button opening Reader B’s view settings popover, `aria-haspopup="dialog"`, `view.settings`) · **Reader** (toggle, `aria-pressed`, `view.reader-mode`) · Commands (`Ctrl K`) · More. This is the “Reader v0.1” variant of [design language §7.1](design-language.md#71-top-bar), in that order; View opens Reader B’s popover (R10) and Reader toggles Reader B’s reader mode (R14). Dialogs sit over this full shell, dimmed and inert (the boards draw the shell `aria-hidden`; the app also makes it `inert`).
- **Rail tabs** follow the cross-area rule: `Pages · Outline`, then contextual tabs. The sample has attachments, so the rail reads `Pages · Outline · Attachments` at 220 px. R02, R05 and R06 show the rail; the other boards collapse it (Sidebar `aria-pressed="false"`) to give the dialog or the page room. While find is open, `Results` is added; with four tabs the rail is 280 px, the selected tab is always visible and tabs beyond the third go into a **More tabs** button. There is no user Bookmarks tab: “Outline” is the PDF’s bookmarks.
- The status bar counts requests made for the open document: “0 network requests for this document” (“9 network requests for this document · to files.example.org only” for a remote file); app files are listed only in the network panel (R13).
- Shortcut chips use the Windows/Linux form (`Ctrl K`); macOS shows `⌘K`. The status bar always carries the network indicator.
- Command ids use the registry’s `area.verb-object` form and the shared namespaces (`file.*`, `view.*`, `nav.*`, `find.*`; [U1b](../../todos/ready/U1b-command-registry-and-palette.md)). A command without a listed shortcut has none and shows no chip. Buttons marked “dialog-local” (Cancel, Close, copy buttons inside one dialog) act only inside their dialog and are not registry commands.

## R01 Home / open

**Purpose.** The first screen of the web reader when no file is open: open a PDF from this device, by URL or from the recent list, and see in one line what happens to the file.

**Entry points.** Loading the app without a `#url=` fragment; closing the last document; More › Open another file… (`file.open`); the phone’s Back to files.

**Layout.** Top bar without document controls: the wordmark “lamina” (Newsreader 18/500) left; Commands and More right. No rail or secondary toolbar. The body is `ground`, one centred column (max 720): Display heading, a drop zone, the Recent list and the privacy line. Status bar: network indicator, “No file open”.

**Content and controls.**

| Control | Does | Command id |
| --- | --- | --- |
| Drop zone (“Drop a PDF anywhere in this window”) | The whole window accepts a dropped file; the zone only explains it. Hint: paste with `Ctrl V` | `file.open` (drop, paste) |
| Choose a file (primary) | `showOpenFilePicker` where available, else the file input | `file.open` |
| Open from URL (toggle, `aria-expanded`) | Reveals the inline form: “PDF address” input (type `url`) and Open. Help: “lamina downloads the file straight from that site. Our server never sees the address.” Opening sets `#url=<encoded>` with `buildFragment` ([C4c](../../todos/ready/C4c-remote-files-encryption-privacy.md)) | `file.open-url` |
| Recent rows | Newsreader file name over “This device / host · pages · last page”, date right. A local file whose handle can’t be reused says “Choose the file again to reopen” | `file.open-recent` |
| Clear list (ghost) | Empties the recent list in this browser | `file.clear-recent` |
| Privacy line | Shield + “lamina reads PDFs inside this browser tab and has no upload server. The status bar counts every network request, so you can check.” + link “How lamina handles your files” | — |

**States.**

- *Idle* (drawn, with the URL form open and focused).
- *Drag over* (described, web only; the phone has no drop zone): the whole window gets an `accent-tint` overlay with a 2 px dashed `accent` inset border and “Drop to open Q3-board-report.pdf”; a non-PDF shows “That isn’t a PDF. lamina opens PDF files only.” in the zone and is ignored.
- *No recent files*: the Recent section is hidden (no empty state needed on a first visit).
- *Bad URL*: the field error under the input (§7.19): “Use an address that starts with https://” or C4c’s “This link’s document address isn’t encoded; ask the sender for a link made with the share button, or paste the address in Open URL”.
- *Site blocks reading* (drawn, `R01-open-url-blocked`): when the fetch fails with a CORS or network error, the canvas shows an error page instead of the document: “This site doesn’t let other sites read the file”, the address with the host in 600, the address in an Address box (§7.27, white), **Open the link in a new tab** (primary, `rel="noreferrer noopener"`) and **Try again** (ghost), and under them one line “Already downloaded it? Choose the downloaded file” whose link-styled button opens the file picker (`file.open`), so the page keeps one primary and one ghost action (§7.21). The status bar shows the one refused request (“1 network request for this document · to files.example.org only (refused by the site)”). No proxy is offered.

**Behaviour.** Opening replaces the home screen with the reader (R02). Remote files fetch from the worker with ranges; the address stays in the fragment only. Recent files are stored locally (OPFS/IndexedDB, [C7b](../../todos/ready/C7b-offline-pwa.md)); nothing about them leaves the browser.

**Accessibility.** Focus order: Commands, More, Choose a file, Open from URL, the URL field, Open, Clear list, recent rows, privacy link. The drop zone is a labelled `section`, not a focus stop; every drop has a button alternative. Recent rows are buttons whose name is the file name plus its meta line. The error page is `role="alert"` (announced on arrival) and focus moves to its primary action, **Open the link in a new tab** (drawn with the ring).

**Platform differences.** *Web:* drawn. *Desktop:* replaced by the start window (S01) with OS recent files and thumbnails. *Phone* (drawn): the home screen is the one phone screen without a document, so its top bar shows the left-aligned wordmark (Newsreader 20/500) and More instead of the centred title and Back of the reading shell; full-width 48 px **Choose a file** and **Open from URL** (the URL form opens as a sheet); recent rows 64 px; the privacy line sits at the bottom and leads with “0 network requests.”; no drop zone, no bottom bar until a file is open. *Tablet:* the web layout with 44 px targets.

**Artboards.** `R01-home.dc.html`, `R01-home-phone.dc.html`, `R01-open-url-blocked.dc.html`.

**Todos.** [C4b](../../todos/ongoing/C4b-web-app-shell.md), [C4c](../../todos/ready/C4c-remote-files-encryption-privacy.md); recent files need [C7b](../../todos/ready/C7b-offline-pwa.md).

## R02 Loading states

**Purpose.** Make opening feel immediate and stable: every page has its true size at once, then fills in (plan [§5.6](../../plans/ui-ux.md#56-perceived-performance-r6-r7)).

**Entry points.** Any open (R01, a `#url=` link, a recent file); scrolling or zooming into pages not yet drawn.

**Layout.** The read-only reader shell with the rail on Pages. The file name is followed by “Loading pages…” until the visible pages are sharp. The drawn board uses 60% zoom, scrolled so the end of page 1, page 2 (current) and the top of page 3 show at once.

**Content and controls.** No new controls; everything stays usable while pages load (scroll, page input, zoom, find, which searches pages as their text arrives).

**States.**

| Step | Looks like |
| --- | --- |
| Placeholder | A white page at its true aspect with the page shadow, nothing else; `aria-busy="true"`, named “Page 3, loading”. No spinner |
| Low-res pass | The cached small bitmap stretched to size: soft, readable layout (drawn as the blurred lower half of page 2) |
| Sharp tiles | 256–512 px tiles replace the soft image tile by tile; old-zoom tiles stay until new ones arrive (no white flash) |
| Rail skeletons | Thumbnails not yet rendered are plain `#ECE7DE` blocks with their page number; no shimmer under reduced motion |
| Whole-file download (drawn, `R02-loading-download`) | When the host has no range support, nothing can be shown until the file is complete: a centred card “Downloading the whole file”, why in one sentence, a determinate bar with “29.8 of 48.0 MB” and “62%”, and **Cancel** (returns to R01) |
| Large local file | Same as the first row; local files never need a download step |

**Behaviour.** The network indicator counts range requests live (“9 network requests for this document · to files.example.org only”); for a local file it stays at 0. Cancelling a download aborts the fetch; nothing is cached.

**Accessibility.** `aria-busy` on pages and the rail list until drawn; the progress bar is `role="progressbar"` with `aria-valuenow`; the bar announces at 25% steps. Reduced motion: no fades between low-res and sharp.

**Platform differences.** *Desktop:* local files only, so only placeholders and tiles. *Phone:* same pipeline with lower tile caps; the page pill shows the page number while placeholders fill in.

**Artboards.** `R02-loading.dc.html`, `R02-loading-download.dc.html`.

**Todos.** [C3b](../../todos/done/C3b-viewer-virtualiser-and-zoom.md).

## R03 Password prompt

**Purpose.** Open an encrypted file: ask for its password once, explain that it isn’t kept, and say plainly when it is wrong.

**Entry points.** Opening a file whose security handler needs a user password (`password-required`); re-prompt after a worker restart. Files that only have an owner password open without a prompt (their restrictions show in R08 and, for printing, in R09).

**Layout.** Dialog (§7.18, 440 px) over the dimmed reader shell. The top bar shows the file name with “Locked” and a disabled page input “/ 24” (`aria-disabled="true"`); the canvas shows blank page placeholders.

**Content and controls.**

| Control | Does | Command id |
| --- | --- | --- |
| Lock icon + title “Enter the password” | `aria-labelledby` of the dialog | — |
| Body | “Q3-board-report.pdf is encrypted. Enter its password to show the pages.” | — |
| Password field | `type="password"`, `autocomplete="current-password"`; focused on open | — |
| Show (text toggle inside the field) | Reveals the password. The visible label stays “Show”; its state is `aria-pressed` (name “Show password”, which contains the visible word) | — |
| Help | “lamina uses it once to open the file on this device and doesn’t save it.” | — |
| Cancel | Closes the file, back to R01 | dialog-local |
| Open (primary, Enter) | Sends the password to the worker | `file.unlock` |

**States.** *Required* (drawn). *Checking:* Open shows “Opening…” and is disabled for up to a second. *Wrong password* (drawn): `danger` field border, `aria-invalid`, and under the field a `role="alert"` message with the warning icon: “That password didn’t open the file. Passwords are case-sensitive; check Caps Lock and try again.” The field keeps focus with its text selected. No attempt limit. *Unsupported handler* (`unsupported-security-handler`): no field; the dialog says “This file uses a kind of encryption lamina can’t open yet.” with Close.

**Behaviour.** The password goes once to the worker (lamina and PDFium) and is not held by the main thread; a worker crash re-prompts. The file is not changed and nothing is uploaded; the network indicator stays at 0 for a local file.

**Accessibility.** Focus is trapped in the dialog; Escape is Cancel; focus returns to the opener (or the home screen). The error is linked by `aria-describedby` and announced once.

**Platform differences.** *Desktop:* same dialog in the window; the OS keychain is not used. *Phone* (drawn): a sheet at the large detent without a grabber (a required question), 44 px field with 16 px text, and **Open** then **Cancel** full width directly under the help text, so they stay above the on-screen keyboard. *Tablet:* the desktop dialog with 44 px controls.

**Artboards.** `R03-password.dc.html`, `R03-password-wrong.dc.html`, `R03-password-phone.dc.html`.

**Todos.** [C4c](../../todos/ready/C4c-remote-files-encryption-privacy.md).

## R04 Repaired / damaged file

**Purpose.** Tell the reader, without alarm, that lamina had to repair a damaged file to show it, and what it changed; and when nothing can be recovered, say so with a way forward.

**Entry points.** The status-bar notice “Repaired on open · Details” (§7.8), shown whenever opening raised repair warnings (ADR 0003 codes); the palette command “Show repair details” (`file.show-repairs`).

**Layout.** Repaired: the details open in a **docked panel** on the right (the inspector slot, 288 px, `surface`), never a modal; the canvas narrows. The notice shows a pressed state while the panel is open: `accent-tint` fill, 1 px `accent` border, `aria-expanded="true"`. Unrecoverable: the canvas holds an error page; no rail.

**Content and controls (details panel).**

| Part | Content |
| --- | --- |
| Header | “Repaired on open” over the file name; Close (“Close repair details”, dialog-local) |
| Notice (`warning-tint`, wrench) | “**This file is damaged.** lamina repaired it in memory so you can read it. The file on your device hasn’t changed, and other readers may show it differently.” |
| What lamina did | One row per warning kind, a plain sentence over the code in `muted` 12: “Rebuilt the table that locates each object in the file” `xref-repaired`; “Measured 3 page streams whose stated length was wrong” `stream-length-recovered · pages 3, 4, 9`; “Left out 1 outline entry that pointed to no page” `outline-item-skipped` |
| Not run | Unsupported features found in the file, e.g. “This file contains JavaScript. lamina never runs it.” |
| Actions | **Copy details** (secondary, `file.copy-repair-details`; copies codes, counts and the laminapdf version as text) · “About repairs” (link) |

Codes come from `fixtures/codes.json`; each code has one plain sentence in the UI string table. More than five kinds collapse to the first five and “Show all 9”.

**Unrecoverable** (drawn): error page in the canvas, `danger` warning icon, “This file can’t be opened”, “lamina couldn’t find the document’s list of pages, even after trying to repair it. It may be incomplete: ask the sender for a new copy, or download it again.”, the code and bytes read in `muted` (“missing-root · 1.8 MB read”), **Try another file** (primary, back to R01) and **Copy error details**. The status bar reads “0 network requests for this document · Offline-safe: nothing uploaded” (a local file). Other fatal codes use the same page with their own sentence (`not-pdf`: “This file isn’t a PDF.”; `limit-exceeded`: “This file is larger or more complex than lamina can open safely.”).

**Behaviour.** Repairs live in memory only; Download a copy (R09) still returns the original bytes. The panel closes with Escape or its close button and returns focus to the notice. The repaired notice uses the status bar’s middle slot, below a signed-file note in priority.

**Accessibility.** The panel is `aside` labelled by its title; rows are a list. The error page is `role="alert"`; focus moves to its primary action, **Try another file** (drawn with the ring).

**Platform differences.** *Desktop:* same. *Phone:* the notice lives in More (“Repaired on open”) and opens the details as a sheet at the medium detent.

**Artboards.** `R04-repaired.dc.html`, `R04-unrecoverable.dc.html`.

**Todos.** [C4b](../../todos/ongoing/C4b-web-app-shell.md).

## R05 Find in document

**Purpose.** Find text across the whole document without blocking scrolling, with a live count, next/previous, match case and whole words, and a list of every result. This find bar is the canonical one for every area.

**Entry points.** The top-bar Find button (pressed: `accent-tint`, 1 px `accent` border, `aria-expanded`); `Ctrl F` / `⌘F` when focus is in the reader (the browser keeps its own find elsewhere); the palette “Find in document” (`find.open`); phone: the bottom bar’s Search.

**Layout.** A **find bar** in the secondary-toolbar slot under the top bar (`role="search"`). In the reader that slot is otherwise empty; in editor modes the find bar replaces the mode toolbar while it is open, so there is still only one bar. The rail adds a **Results** tab while find is open: with Attachments that makes four tabs, so the rail is 280 px and shows `Pages · Outline · Results` plus a **More tabs** button holding Attachments. Matches are painted on the pages.

**Content and controls.**

| Control | Does | Command id / shortcut |
| --- | --- | --- |
| Find field (300 px, search icon, count inside) | Searches as you type (debounced); count “3 of 9”, “40+” while counting, “No matches” | `find.open` (`Ctrl F` / `⌘F`) |
| Previous match / Next match | Move and scroll the current match into view | `find.previous` (`Shift Enter`, `Shift F3`), `find.next` (`Enter`, `F3`) |
| Match case · Whole words | Toggle buttons (§7.24); re-run the search | `find.match-case`, `find.whole-words` |
| Context note | “Searching… page 14 of 24”, then “Searched all 24 pages” | — |
| Close find | Clears highlights, removes the Results tab, returns focus to where it was | `find.close` (`Esc`) |
| Results tab | Matches grouped under “Page 3 · Repairs backlog” (page label + outline section), each a link with a snippet and the match in 600; the current one has `accent-tint` and an `accent` inset ring, `aria-current="true"` | — |
| Replace (editor only) | Opened from the Edit toolbar’s **Replace…** button or ⌥⌘F / `Ctrl H` on desktop ([A06](add-edit-content.md)): this bar plus a second row ([§7.30](design-language.md#730-find-bar-and-results-proposed)): a 300 px replace field, **Replace** · **Skip** · **Replace n checked** (primary) and a note. The reader v0.1 has no Replace row | `find.open-replace` |

Match colours on the page are the `find-match` and `find-current` tokens, defined per page appearance (design language [§2.6](design-language.md#26-find-marks)). On Original: other matches `#F6E3BC` with a 1 px `warning` bottom border (so they differ from a yellow highlight), the current match `#EDBF6A` with a 2 px `accent` outline.

**States.** *Typing* (count “…”); *counting* (“40+”, results stream in by page); *done* (drawn: “3 of 9”, “Searched all 24 pages”); *no matches* (count “No matches”, Results tab shows “No matches for “backlogs”.” with a ghost “Turn off Whole words” when a toggle is on); *scanned pages without text*: the context note adds “4 pages have no text to search”.

**Behaviour.** The worker searches from the current page forward with wrap-around at background priority; a new query cancels the old one. Each Next/Previous or result click is a history jump (R07), so the board, reached with Next from page 1, has Back enabled as “Back to page 1”. Nothing in the file changes.

**Accessibility.** The count is a polite live region, throttled to one announcement per second. Focus stays in the field while Enter moves through matches. Results are links in page order. The current match is not shown by colour alone (outline, count, `aria-current` in the list). On the phone the Results sheet is **non-modal** (`section` labelled by its title, no focus trap): Previous · Results · Next stay usable under it and the page keeps scrolling; Done or Close results closes it. Sheets that ask a question or hold a form (password, URL, properties) stay modal; see design language [§7.16](design-language.md#716-bottom-sheet).

**Platform differences.** *Desktop:* same; `⌘F`/`Ctrl F` always belong to lamina. *Phone* (drawn): the top bar becomes a 44 px search field (16 px text, count inside) with **Done**; the bottom bar becomes a find toolbar (Previous · Results · Next); Results is a scrolling sheet at the medium detent with **Match case** / **Whole words** as 44 px toggles and 44 px result rows. *Tablet:* desktop layout, Results as a sheet when the rail is collapsed.

**Artboards.** `R05-find.dc.html`, `R05-find-phone.dc.html`.

**Todos.** [C5b](../../todos/ready/C5b-find-in-document.md).

## R06 Rail tabs

**Purpose.** Move through a long document by its outline, its pages and its labels, with attached files one tab away.

**Entry points.** The Sidebar button in the top bar (`view.toggle-sidebar`); clicking a tab; palette “Show outline” (`view.show-outline`), “Show pages” (`view.show-pages`), “Show attachments” (`view.show-attachments`); the outline also opens on load when the file says so (`/PageMode /UseOutlines`).

**Layout.** Left rail, 220 px, segmented tabs (§7.5): **Pages · Outline**, plus **Attachments** because the sample file has embedded files ([§8.10](design-language.md#810-rail-tab-set-proposed); O14’s 272 px rail is a Pages and navigation board follow-up).

**Content and controls.**

| Tab | Content | Command id |
| --- | --- | --- |
| Outline (drawn) | The PDF’s bookmarks as a WAI-ARIA **tree** (not the §7.7 link list): rows Newsreader 15, level 2 at 14 `text-3`; a 16 px chevron (down = expanded, right = collapsed) before rows with children; page label right. Current section `accent` 600 on `accent-tint`, `aria-current="location"`, tracked as you scroll (deepest entry at or above the top of the view). Click or Enter goes to the destination with its fit mode | `nav.go-to-outline-item`; `view.expand-outline`, `view.collapse-outline` (palette) |
| Pages (drawn on the appendix) | Thumbnails (§7.6) labelled with the **page label** (“A-1”), name “Page A-1, 22 of 24”. The current page has the thumbnail outline and a 600 label; keyboard focus is a separate rounded ring around the whole thumbnail and label with the `hover` fill (drawn on A-2). The top-bar page input widens to fit the label and shows the physical number beside it: `A-1` `(22 / 24)`; typing a label or a number goes there (a label wins; when both match, a hint under the field shows both). The status bar reads “Page A-1 (22 of 24) · Data notes” | `nav.go-to-page` |
| Attachments (drawn; after v0.1, [E3f](../../todos/ready/E3f-attachments-and-associated-files.md)) | The full panel is [O14](pages-navigation-extraction.md#o14-attachments-panel); this board is its reader form, without an inspector. Header “3 files · 1.3 MB” and **Edit in Organise** (§8.11); rows with a paperclip, Newsreader name, “86 KB · Spreadsheet”, a neutral badge for associated files (“Source of page 3”, “Data for page 11”), and **Download** (web; “Save to disk…” on desktop), named with the file (“Download repairs-by-region.xlsx”). Footnote: “Attachments stay inside this PDF, on this device. lamina saves them exactly as they are and never opens or runs them.” | `file.save-attachment` |

**States.** Outline loading for big files: the first 50 rows with a skeleton below, children loaded on expand. No outline: empty state “This file has no outline” / “Use Pages to move through it.” Skipped entries (repairs) are omitted and counted in R04. Thumbnails render at low priority; skeletons until drawn. The thumbnails board was reached by clicking A-1 from page 3, so Back reads “Back to page 3”. *Program attachment* (O14): an `.exe`, `.js`, `.bat` or `.app` file gets a warning badge “Program”, and Download first opens an `alertdialog` “This file is a program. Programs from documents can harm your computer. lamina never opens it; save it only if you trust the sender.” with Cancel (default) and Save anyway. *Too large for this device* (web): “This file is larger than lamina can hold in memory here (limit 512 MB). Use the desktop app to save it.”

**Behaviour.** Selecting an outline row or thumbnail is a history jump (R07) and updates `#page=` through `buildFragment`. Tab choice, rail width and open state are remembered per browser. Saving an attachment writes its original bytes via the platform save dialog. Nothing in the reader edits the outline; that happens in Organise.

**Accessibility.** Tree keys: ↑ ↓ move, → expands or enters, ← collapses or goes to the parent, Home/End, Enter activates, type-ahead on titles; one row is in the tab order (drawn with the focus ring). Thumbnails use roving focus with ↑ ↓. Tabs are `role="tablist"` with arrow-key switching.

**Platform differences.** *Desktop:* same rail; widths are remembered per window. *Phone:* each tab is a sheet (R18, Reader B). *Tablet:* the rail is a docked panel, collapsed by default in portrait.

**Artboards.** `R06-rail-outline.dc.html`, `R06-rail-thumbnails.dc.html`, `R06-rail-attachments.dc.html`.

**Todos.** [C6a](../../todos/ready/C6a-thumbnails-outline-page-labels.md) (Pages, Outline, page labels); [E3f](../../todos/ready/E3f-attachments-and-associated-files.md) (Attachments).

## R07 Links and history

**Purpose.** Show where a link goes before you follow it, return to where you were after any jump, and never leave the document for the web without asking.

**Entry points.** Hovering or focusing a link on a page; Back/Forward in the top bar; `Alt ←` / `Alt →` (Windows/Linux) and `⌘[` / `⌘]` (macOS); the browser’s own Back button.

**Layout.** The **history buttons** (Back, Forward; 26 px icon buttons in a `role="group"` named “History”) sit before the page input on every reader board. Link preview: a short-lived popover under the link (z 30). External links: a dialog (§7.18, 440 px).

**Content and controls.**

| Control | Does | Command id / shortcut |
| --- | --- | --- |
| Back | Returns to the previous position (page, scroll, zoom). Its name says where: “Back to page 2”; disabled when there is nothing to go back to | `nav.back` (`Alt ←`, `⌘[`) |
| Forward | Redoes a Back; disabled after a new jump | `nav.forward` (`Alt →`, `⌘]`) |
| Internal link | On hover or keyboard focus, after 300 ms, a **link preview**: a crop of the target page’s top, the target’s outline title (“4. Surveyor capacity”) and “Link to page 11 · Enter to go”. Click or Enter follows it. Named actions (Next page, First page…) show “Goes to the next page” and no crop | `nav.follow-link` |
| External link | Opens the confirmation dialog: “Open this website?”, the full address with the host in 600 (punycode for international hosts), “It opens in a new tab. The site isn’t told which document you came from.”, **Copy link** (ghost), **Cancel** (default focus), **Open website** (primary); close button “Close” (the dialog supplies the context) | `nav.open-external-link`, `nav.copy-link` |

**States.** *Preview* (drawn, with Back enabled after jumping from the contents page; the link points to `#page=11` because the file is local, and the preview sits under the link, anchored to it, so it follows the line when the text rewraps). *External confirmation* (drawn; the page text shows a shorter address than the real target, which is why the dialog shows the full one). *Unsupported:* `Launch`, `JavaScript`, `SubmitForm` and other actions are never run; hovering shows “This link runs an action lamina doesn’t run” and the action is listed in R04’s Not run. *Other file* (`GoToR`): “Links to another file, budget-2026.pdf, which lamina can’t open from here.” Only `http(s):` and `mailto:` can be opened.

**Behaviour.** Jumps (links, outline, thumbnails, go to page, find results) push a history entry; scrolling alone doesn’t. Entries are written to the URL fragment with `buildFragment`, so the browser’s Back and Forward work too and nothing is sent to the server. External links open with `noopener,noreferrer`.

**Accessibility.** Links are real anchors in page order with names from `/Contents` or the destination (“Link to page 11, 4. Surveyor capacity”). The preview is a `role="tooltip"` referenced by `aria-describedby`; it stays while hovered, Escape dismisses it (WCAG 1.4.13). The focus ring is drawn on the link.

**Platform differences.** *Web:* `Alt ←` and `⌘[` are also the browser’s Back; because history lives in the fragment, both do the same thing, so lamina doesn’t intercept them. *Desktop:* lamina owns these keys and the mouse back/forward buttons; external links open in the default browser after the same dialog. *Phone:* long-press a link shows the preview in a sheet with **Go to page 11**; Back and Forward live in More; the OS back gesture goes back in lamina’s history first.

**Artboards.** `R07-link-preview.dc.html`, `R07-external-link.dc.html`.

**Todos.** [C6b](../../todos/ready/C6b-links-and-history.md).

## R08 Document properties

**Purpose.** Everything lamina knows about the file in one place, read from lamina’s own parser: description, version, pages and size, tagging, fonts, security and signatures.

**Entry points.** More › Document properties (`file.properties`); the palette. No shortcut (`Mod D` is reserved by browsers, §8.8).

**Layout.** Dialog (§7.18 at form width, 560 px) over the dimmed reader shell, with a fixed header (title “Document properties” over the file name, close “Close”) and footer, and a body that scrolls (`max-height: calc(100vh - 232px)`) with four sections with Heading 18 titles: **Description**, **Document**, **Fonts**, **Security**. Values are a two-column definition list (labels 12/600 `text-3`, 148 px).

**Content and controls.**

| Section | Rows (sample values) |
| --- | --- |
| Description | Title “Q3 Board Report” · Author · Subject · Keywords · Created “14 Oct 2026, 09:12” · Modified “28 Oct 2026, 16:04” · Made with “Northwind report template · saved by Northwind print service” (Info and XMP merged; XMP wins when they differ) |
| Document | PDF version 1.7 · Pages “24 · labelled 1–21, A-1–A-3” · Page size “US Letter, 8.5 × 11 in · all pages” (or “Mixed: 22 Letter, 2 Tabloid”) · File size “1.2 MB (1,262,480 bytes)” · Tagged “Yes · headings and reading order for screen readers” · Fast web view · Contains “3 attachments · no forms · no JavaScript” |
| Fonts | Table: Name · Type · In the file (“Embedded, subset”, “Not embedded · shown with a similar font”) |
| Security | Encryption · Permissions · Signatures |

Each value has a copy button on hover or focus (drawn focused on Title, “Copy title”); the footer has **Copy all** (ghost, plain text) and **Close**. All are dialog-local (no command ids).

**States.** *Encrypted or restricted file:* Encryption “Password protected · AES 256-bit (revision 6)”; Permissions lists what the author restricted (“Printing: not allowed · Copying text: allowed”) with the note “lamina asks before doing something the author restricted.” (see R09’s restricted print). *Signed file:* Signatures “2 signatures present, not verified” (validation comes with the editor’s signing work). *Long values* wrap; the dialog never scrolls horizontally.

**Behaviour.** Read-only; nothing changes in the file. Dates use the reader’s locale (`Intl.DateTimeFormat`).

**Accessibility.** `role="dialog"`, labelled by its title; sections are `section` with headings; the fonts table has `th scope="col"`. Copy buttons are named by the value they copy; a toast “Copied” confirms.

**Platform differences.** *Desktop:* same dialog (File › Properties… in the menu bar). *Phone:* a sheet at the large detent; the fonts table becomes a list.

**Artboards.** `R08-properties.dc.html`.

**Todos.** [C6c](../../todos/ready/C6c-print-download-properties.md).

## R09 Print and Download a copy

**Purpose.** Print pages that look the same in every browser, and save the file exactly as it was opened.

**Entry points.** More › Print… and `Ctrl P` / `⌘P` when a document is open (`file.print`; lamina’s print replaces the browser’s); More › Download a copy (`file.save-copy`, no shortcut). The More menu is drawn open on `R09-download`.

**Layout.** Print: a dialog (§7.18 form width, 560 px) over the dimmed reader shell, then a progress dialog. Download: a menu item, then the browser’s save, then a toast.

**Content and controls (print).**

| Control | Does |
| --- | --- |
| Pages (radio group) | All 24 pages · Current page (3) · Pages [range field, placeholder “1–5, 11, A-1”] with help “Use page numbers or labels, for example 1–5, 11, A-1.” Choosing the field selects its radio |
| Scale (select) | Fit to paper (default) · Actual size |
| Quality (select) | Standard (150 dpi) · High (300 dpi); help “High takes longer to prepare.” |
| Print comments and highlights (checkbox) | On by default; follows each annotation’s print flag |
| Page appearance notice | Shown when the page appearance isn’t Original (drawn: Sepia): “**You’re reading in Sepia.** Pages print in their original colours, as the author made them.” with the checkbox “Print in Sepia instead” |
| Footer | “Your browser’s print window opens next, to choose the printer.” · Cancel · **Print…** (primary); close “Close”. Pages, Scale, Quality, the checkboxes, Cancel and Close are dialog-local |

The radios and checkboxes are native inputs, visually hidden inside their labels, with drawn 16 px boxes and circles (§7.19, decision 8); the selects are native `select`s styled to §7.19 (32 px, with the chevron).

**States.**

- *Preparing* (drawn, `R09-print-progress`): “Preparing pages for print”, “lamina draws each page at print quality on this device. Your browser’s print window opens when all 24 pages are ready.”, a determinate bar “Page 9 of 24 · 38%”, **Cancel** (focused; `print.cancel`, as S05). Cancel stops rendering and frees memory.
- *Printing restricted by the author* (drawn, `R09-print-restricted`, both steps: the print dialog dimmed underneath, the confirmation on top): when the file’s permissions don’t allow printing (the `/P` print bits, ISO 32000-2 §7.6.4.2), the print dialog shows a `warning` notice “The author restricted printing of this file.” and its button reads **Print anyway…**; it opens an `alertdialog` “Print a restricted file?” explaining that the restriction is a request, not a lock, and asking to print only with permission: **Cancel** (default focus) and **Print anyway**. When only high-quality printing is restricted, Quality is fixed at Standard with the note “The author allows printing at standard quality only.” **Owner decision needed:** [C4c](../../todos/ready/C4c-remote-files-encryption-privacy.md) says owner-password permissions are “not enforced beyond display in v0.1”, while [C6c](../../todos/ready/C6c-print-download-properties.md)’s risk note recommends respecting the print bits with a warning and confirmation. This design follows C6c (warn and confirm, never block); if the owner keeps C4c’s rule, drop the confirmation and keep only the notice.
- *Range error:* “There’s no page 30. This file has 24 pages (1–21, A-1–A-3).” under the field; Print stays enabled and focuses the field.
- *Too large for memory:* High quality on a very long range falls back to Standard with a notice “Printing at standard quality to stay within this device’s memory.”
- *Download done:* toast (§7.20) “Downloaded Q3-board-report.pdf”. An encrypted file downloads still encrypted; a remote file is streamed from the cache and its host without changes; the name comes from the URL or `Content-Disposition`, sanitised.

**Behaviour.** Printing renders pages in the worker into a hidden iframe (CSP-safe, [C6c](../../todos/ready/C6c-print-download-properties.md)) and calls the browser’s print; it changes nothing in the file. Download a copy (`file.save-copy`) writes the document as it currently is; the v0.1 reader cannot change it, so that is the original bytes, byte for byte, even if the file was repaired in memory (R04). The menu item’s tooltip says “The file exactly as it was opened”. No shortcut: `Ctrl S` / `⌘S` is Save, in edit sessions only.

The web More menu (drawn) holds Download a copy (`file.save-copy`) · Print… (`file.print`, `Ctrl P`) · Document properties… (`file.properties`) │ Network activity (`privacy.network-activity`) · Install lamina… (`app.install`, only where the browser offers it) · Get the desktop app (link to S11) │ Keyboard shortcuts (`help.shortcuts`, `?`) · Privacy and help │ Open another file… (`file.open`, `Ctrl O`). It is the web counterpart of Reader B’s phone More sheet (R18), which adds View settings because the phone top bar has no View button.

**Accessibility.** The dialog uses a `fieldset`/`legend` for Pages and real `select`s; Enter prints; Escape cancels. The progress dialog announces every 25%. The restricted-print confirmation is an `alertdialog` with focus on Cancel. The More menu is `role="menu"` opened from a button with `aria-haspopup="menu"` and `aria-expanded`; ↑ ↓ move, Enter runs, Escape closes and returns focus; the active item uses the palette’s active-row style.

**Platform differences.** *Desktop:* print hands off to the native print dialog with a preview (S05, [D4c](../../todos/ready/D4c-native-printing.md)), with the same restriction rule once the owner decides it (Desktop shell’s S05 doesn’t draw the step yet); the menu item is File › Save a copy… (`file.save-copy`, no shortcut, design language decision 18) with a native save panel. In the read-only reader it writes the bytes as opened; in an edit session it writes the document with its changes. *Phone:* Print uses the OS share/print sheet after the same preparing step; Download becomes Share › Save to Files. *Windows/Linux vs macOS:* chips `Ctrl P` / `⌘P`.

**Artboards.** `R09-print.dc.html`, `R09-print-progress.dc.html`, `R09-print-restricted.dc.html`, `R09-download.dc.html`.

**Todos.** [C6c](../../todos/ready/C6c-print-download-properties.md).

## New components

Merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29; see its decisions table.

## Proposed design-language additions

Merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29; see its decisions table.
