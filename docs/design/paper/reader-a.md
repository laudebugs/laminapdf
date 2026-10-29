# Paper: Reader A

Screens R01–R09 of the [screen inventory](screens.md): opening a file (home, URL, recent files), loading, the password prompt, repaired and damaged files, find, the rail tabs, links and history, document properties, and print and download. These are the web reader v0.1 screens; the desktop app shares the same viewer and differs only where noted. Everything follows the [Paper design language](design-language.md); new parts are listed under [New components](#new-components).

- **Canvas:** [lamina Paper — Reader A](https://claude.ai/artifact/MWpEwMqZSWUBYSMuCi1Fz2) (private until the owner shares it). R08 and R09 are written but were not yet published when this was drafted (the canvas hit its daily publish limit); their files are listed under each screen.
- **Plans:** [web reader §4 functional](../../plans/web-app.md#functional) and [privacy and security](../../plans/web-app.md#privacy-and-security); layout rules from the [UI/UX plan](../../plans/ui-ux.md) §4–§5.
- **Sample file:** `Q3-board-report.pdf`, Northwind Housing Trust, 24 pages, page labels `1`–`21` then `A-1`–`A-3` for Appendix A (pages 22–24), 2 attached files. Other files (`Tenant-survey-2026.pdf`, `Northwind-asset-survey-2026.pdf`, `Minutes-september.pdf`) and hosts (`files.example.org`, `www.northwind-housing.example`) are fictional and use reserved example domains.

Conventions on every web board: the shortcut chips use the Windows/Linux form (`Ctrl K`); macOS shows `⌘K`. The status bar always carries the network indicator. Command ids use the registry’s `area.verb-object` form ([U1b](../../todos/ready/U1b-command-registry-and-palette.md)); a command without a listed shortcut has none and shows no chip.

## R01 Home / open

**Purpose.** The first screen of the web reader when no file is open: open a PDF from this device, by URL or from the recent list, and see in one line what happens to the file.

**Entry points.** Loading the app without a `#url=` fragment; closing the last document; More › Open another file… (`file.open`); the phone’s Back to files.

**Layout.** Top bar without document controls: the wordmark “lamina” (Newsreader 18/500) left; Commands and More right. No rail, mode switcher or secondary toolbar. The body is `ground`, one centred column (max 720): Display heading, a drop zone, the Recent list and the privacy line. Status bar: network indicator, “No file open”.

**Content and controls.**

| Control | Does | Command id |
| --- | --- | --- |
| Drop zone (“Drop a PDF anywhere in this window”) | The whole window accepts a dropped file; the zone only explains it. Hint: paste with `Ctrl V` | `file.open` (drop, paste) |
| Choose a file (primary) | `showOpenFilePicker` where available, else the file input | `file.open` |
| Open from URL (toggle, `aria-expanded`) | Reveals the inline form: “PDF address” input (type `url`) and Open. Help: “lamina downloads the file straight from that site. Our server never sees the address.” Opening sets `#url=<encoded>` with `buildFragment` ([C4c](../../todos/ready/C4c-remote-files-encryption-privacy.md)) | `file.open-url` |
| Recent rows | Newsreader file name over “This device / host · pages · last page”, date right. A local file whose handle can’t be reused says “Choose the file again to reopen” | `recent.open` |
| Clear list (ghost) | Empties the recent list in this browser | `recent.clear` |
| Privacy line | Shield + “lamina reads PDFs inside this browser tab and has no upload server. The status bar counts every network request, so you can check.” + link “How lamina handles your files” | — |

**States.**

- *Idle* (drawn, with the URL form open and focused).
- *Drag over* (described): the whole window gets an `accent-tint` overlay with a 2 px dashed `accent` inset border and “Drop to open Q3-board-report.pdf”; a non-PDF shows “That isn’t a PDF. lamina opens PDF files only.” in the zone and is ignored.
- *No recent files*: the Recent section is hidden (no empty state needed on a first visit).
- *Bad URL*: the field error under the input (§7.19): “Use an address that starts with https://” or C4c’s “This link’s document address isn’t encoded; ask the sender for a link made with the share button, or paste the address in Open URL”.
- *Site blocks reading* (drawn, `R01-open-url-blocked`): when the fetch fails with a CORS or network error, the canvas shows an error page instead of the document: “This site doesn’t let other sites read the file”, the address with the host in 600, and **Open the link in a new tab** (`rel="noreferrer noopener"`), **Choose the downloaded file**, **Try again**. The status bar shows the one refused request (“1 network request · to files.example.org only (refused by the site)”). No proxy is offered.

**Behaviour.** Opening replaces the home screen with the reader (R02). Remote files fetch from the worker with ranges; the address stays in the fragment only. Recent files are stored locally (OPFS/IndexedDB, [C7b](../../todos/ready/C7b-offline-pwa.md)); nothing about them leaves the browser.

**Accessibility.** Focus order: Commands, More, Choose a file, Open from URL, the URL field, Open, Clear list, recent rows, privacy link. The drop zone is a labelled `section`, not a focus stop; every drop has a button alternative. Recent rows are buttons whose name is the file name plus its meta line. The error page is `role="alert"` and moves focus to its heading.

**Platform differences.** *Web:* drawn. *Desktop:* replaced by the start window (S01) with OS recent files and thumbnails. *Phone* (drawn): title “lamina” in the top bar; full-width 48 px **Choose a file** and **Open from URL** (the URL form opens as a sheet); recent rows 64 px; the privacy line sits at the bottom and leads with “0 network requests.”; no drop zone, no bottom bar until a file is open. *Tablet:* the web layout with 44 px targets.

**Artboards.** `R01-home.dc.html`, `R01-home-phone.dc.html`, `R01-open-url-blocked.dc.html`.

**Todos.** [C4b](../../todos/ready/C4b-web-app-shell.md), [C4c](../../todos/ready/C4c-remote-files-encryption-privacy.md); recent files need [C7b](../../todos/ready/C7b-offline-pwa.md).

## R02 Loading states

**Purpose.** Make opening feel immediate and stable: every page has its true size at once, then fills in (plan [§5.6](../../plans/ui-ux.md#56-perceived-performance-r6-r7)).

**Entry points.** Any open (R01, a `#url=` link, a recent file); scrolling or zooming into pages not yet drawn.

**Layout.** The normal reader shell (R06). The file name’s save-state slot says “Loading pages…” until the visible pages are sharp. The drawn board uses 60% zoom to show three pages at once.

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

**Behaviour.** The network indicator counts range requests live (“9 network requests · to files.example.org only”); for a local file it stays at 0. Cancelling a download aborts the fetch; nothing is cached.

**Accessibility.** `aria-busy` on pages and the rail list until drawn; the progress bar is `role="progressbar"` with `aria-valuenow`; the status text is not announced repeatedly (the bar announces at 25% steps). Reduced motion: no fades between low-res and sharp.

**Platform differences.** *Desktop:* local files only, so only placeholders and tiles. *Phone:* same pipeline with lower tile caps; the page pill shows the page number while placeholders fill in.

**Artboards.** `R02-loading.dc.html`, `R02-loading-download.dc.html`.

**Todos.** [C3b](../../todos/ready/C3b-viewer-virtualiser-and-zoom.md).

## R03 Password prompt

**Purpose.** Open an encrypted file: ask for its password once, explain that it isn’t kept, and say plainly when it is wrong.

**Entry points.** Opening a file whose security handler needs a user password (`password-required`); re-prompt after a worker restart. Files that only have an owner password open without a prompt (their restrictions show in R08).

**Layout.** Dialog (§7.18, 440 px) over the dimmed shell. Behind it the top bar shows the file name with “Locked”, the page count (“– / 24”) and blank page placeholders.

**Content and controls.**

| Control | Does | Command id |
| --- | --- | --- |
| Lock icon + title “Enter the password” | `aria-labelledby` of the dialog | — |
| Body | “Q3-board-report.pdf is encrypted. Enter its password to show the pages.” | — |
| Password field | `type="password"`, `autocomplete="current-password"`; focused on open | — |
| Show / Hide (text toggle inside the field) | Reveals the password; `aria-pressed`, name “Show password” | — |
| Help | “lamina uses it once to open the file on this device and doesn’t save it.” | — |
| Cancel | Closes the file, back to R01 | — |
| Open (primary, Enter) | Sends the password to the worker | `doc.unlock` |

**States.** *Required* (drawn). *Checking:* Open shows “Opening…” and is disabled for up to a second. *Wrong password* (drawn): `danger` field border, `aria-invalid`, and under the field a `role="alert"` message with the warning icon: “That password didn’t open the file. Passwords are case-sensitive; check Caps Lock and try again.” The field keeps focus with its text selected. No attempt limit. *Unsupported handler* (`unsupported-security-handler`): no field; the dialog says “This file uses a kind of encryption lamina can’t open yet.” with Close.

**Behaviour.** The password goes once to the worker (lamina and PDFium) and is not held by the main thread; a worker crash re-prompts. The file is not changed and nothing is uploaded; the network indicator stays at 0 for a local file.

**Accessibility.** Focus is trapped in the dialog; Escape is Cancel; focus returns to the opener (or the home screen). The error is linked by `aria-describedby` and announced once.

**Platform differences.** *Desktop:* same dialog in the window; the OS keychain is not used. *Phone* (drawn): a sheet at the large detent without a grabber (a required question), 44 px field with 16 px text, full-width **Open** then **Cancel**. *Tablet:* the desktop dialog with 44 px controls.

**Artboards.** `R03-password.dc.html`, `R03-password-wrong.dc.html`, `R03-password-phone.dc.html`.

**Todos.** [C4c](../../todos/ready/C4c-remote-files-encryption-privacy.md).

## R04 Repaired / damaged file

**Purpose.** Tell the reader, without alarm, that lamina had to repair a damaged file to show it, and what it changed; and when nothing can be recovered, say so with a way forward.

**Entry points.** The status-bar notice “Repaired on open · Details” (§7.8), shown whenever opening raised repair warnings (ADR 0003 codes); the palette command “Show repair details” (`repair.show-details`).

**Layout.** Repaired: the details open in a **docked panel** on the right (the inspector slot, 288 px, `surface`), never a modal; the canvas narrows. The notice stays pressed (`aria-expanded="true"`, `accent-tint`) while the panel is open. Unrecoverable: the canvas holds an error page; no rail or modes.

**Content and controls (details panel).**

| Part | Content |
| --- | --- |
| Header | “Repaired on open” over the file name; Close (“Close repair details”) |
| Notice (`warning-tint`, wrench) | “**This file is damaged.** lamina repaired it in memory so you can read it. The file on your device hasn’t changed, and other readers may show it differently.” |
| What lamina did | One row per warning kind, a plain sentence over the code in `muted` 12: “Rebuilt the table that locates each object in the file” `xref-repaired`; “Measured 3 page streams whose stated length was wrong” `stream-length-recovered · pages 3, 4, 9`; “Left out 1 outline entry that pointed to no page” `outline-item-skipped` |
| Not run | Unsupported features found in the file, e.g. “This file contains JavaScript. lamina never runs scripts in PDFs.” |
| Actions | **Copy details** (secondary; copies codes, counts and the lamina version as text) · “About repairs” (link) |

Codes come from `fixtures/codes.json`; each code has one plain sentence in the UI string table. More than five kinds collapse to the first five and “Show all 9”.

**Unrecoverable** (drawn): error page in the canvas, `danger` warning icon, “This file can’t be opened”, “lamina couldn’t find the document’s list of pages, even after trying to repair it. It may be incomplete: ask the sender for a new copy, or download it again.”, the code and bytes read in `muted` (“missing-root · 1.8 MB read”), **Try another file** (primary, back to R01) and **Copy error details**. Other fatal codes use the same page with their own sentence (`not-pdf`: “This file isn’t a PDF.”; `limit-exceeded`: “This file is larger or more complex than lamina can open safely.”).

**Behaviour.** Repairs live in memory only; saving later (editor) writes a clean file. The panel closes with Escape or its close button and returns focus to the notice.

**Accessibility.** The panel is `aside` labelled by its title; rows are a list. The error page is `role="alert"`, focus moves to its heading; the primary action is next in tab order.

**Platform differences.** *Desktop:* same. *Phone:* the notice lives in More (“Repaired on open”) and opens the details as a sheet at the medium detent.

**Artboards.** `R04-repaired.dc.html`, `R04-unrecoverable.dc.html`.

**Todos.** [C4b](../../todos/ready/C4b-web-app-shell.md).

## R05 Find in document

**Purpose.** Find text across the whole document without blocking scrolling, with a live count, next/previous, match case and whole words, and a list of every result.

**Entry points.** The top-bar Find button (pressed while open); `Ctrl F` / `⌘F` when focus is in the reader (the browser keeps its own find elsewhere); the palette “Find in document” (`find.open`); phone: the bottom bar’s Search.

**Layout.** A **find bar** in the secondary-toolbar slot under the top bar (`role="search"`). In Read mode the slot is empty; in other modes the find bar replaces the mode toolbar while it is open, so there is still only one bar. The rail switches to a **Results** tab that exists only while find is open (with four tabs the rail opens at 280 px; see [New components](#new-components)). Matches are painted on the pages.

**Content and controls.**

| Control | Does | Command id / shortcut |
| --- | --- | --- |
| Find field (300 px, search icon, count inside) | Searches as you type (debounced); count “3 of 9”, “40+” while counting, “No matches” | `find.open` (`Ctrl F` / `⌘F`) |
| Previous match / Next match | Move and scroll the current match into view | `find.previous` (`Shift Enter`, `Shift F3`), `find.next` (`Enter`, `F3`) |
| Match case · Whole words | Toggle buttons (§7.24); re-run the search | `find.match-case`, `find.whole-words` |
| Context note | “Searching… page 14 of 24”, then “Searched all 24 pages” | — |
| Close find | Clears highlights, removes the Results tab, returns focus to where it was | `find.close` (`Esc`) |
| Results tab | Matches grouped under “Page 3 · Repairs backlog” (page label + outline section), each a link with a snippet and the match in 600; the current one has `accent-tint` and an `accent` inset ring, `aria-current="true"` | — |

Match colours on the page (new, proposed): other matches `#F6E3BC`, current match `#EDBF6A` plus the 2 px `accent` outline; ink on both ≥ 9:1. They differ from the yellow highlight by the outline and by being transient.

**States.** *Typing* (count “…”); *counting* (“40+”, results stream in by page); *done* (drawn: “3 of 9”, “Searched all 24 pages”); *no matches* (count “No matches”, Results tab shows “No matches for “backlogs”.” with a ghost “Turn off Whole words” when a toggle is on); *scanned pages without text*: the context note adds “4 pages have no text to search”.

**Behaviour.** The worker searches from the current page forward with wrap-around at background priority; a new query cancels the old one. Each Next/Previous or result click is a history jump (R07). Nothing in the file changes.

**Accessibility.** The count is a polite live region, throttled to one announcement per second. Focus stays in the field while Enter moves through matches. Results are links in page order. The current match is not shown by colour alone (outline, count, `aria-current` in the list).

**Platform differences.** *Desktop:* same; `⌘F`/`Ctrl F` always belong to lamina. *Phone* (drawn): the top bar becomes the search field (16 px text, count inside, **Done**); the bottom bar becomes a find toolbar (Previous · Results · Next); Results is a sheet at the medium detent with **Match case** / **Whole words** as 44 px toggles and 44 px result rows. *Tablet:* desktop layout, Results as a sheet when the rail is collapsed.

**Artboards.** `R05-find.dc.html`, `R05-find-phone.dc.html`.

**Todos.** [C5b](../../todos/ready/C5b-find-in-document.md).

## R06 Rail tabs

**Purpose.** Move through a long document by its outline, its pages and its labels, with bookmarks and attached files one tab away.

**Entry points.** The Sidebar button in the top bar (`rail.toggle`); clicking a tab; palette “Show outline” (`rail.outline`), “Show pages” (`rail.pages`), “Show bookmarks” (`rail.bookmarks`), “Show attachments” (`rail.attachments`); the outline also opens on load when the file says so (`/PageMode /UseOutlines`).

**Layout.** Left rail with segmented tabs (§7.5): **Pages · Outline · Bookmarks**, plus **Attachments** when the file has embedded files. With four tabs the rail opens at 280 px. The drawn boards use the sample file, which has attachments.

**Content and controls.**

| Tab | Content | Command id |
| --- | --- | --- |
| Outline (drawn) | A WAI-ARIA **tree** (not the §7.7 link list): rows Newsreader 15, level 2 at 14 `text-3`; a 16 px chevron (down = expanded, right = collapsed) before rows with children; page label right. Current section `accent` 600 on `accent-tint`, `aria-current="location"`, tracked as you scroll (deepest entry at or above the top of the view). Click or Enter goes to the destination with its fit mode | `outline.go`, `outline.expand-all`, `outline.collapse-all` (palette) |
| Pages (drawn on the appendix) | Thumbnails (§7.6) labelled with the **page label** (“A-1”), name “Page A-1, 22 of 24”. The top-bar page input widens to fit the label and shows the physical number beside it: `A-1` `(22 / 24)`; typing a label or a number goes there (a label wins; when both match, a hint under the field shows both). The status bar reads “Page A-1 (22 of 24) · Data notes” | `nav.go-to-page` |
| Bookmarks | Pages the reader marked (kept locally). Empty state (§7.21): “No bookmarks yet” / “Choose Add bookmark from a page’s More menu.” | `bookmark.add`, `bookmark.remove` |
| Attachments (drawn) | “2 files attached to this PDF”; rows with a paperclip, Newsreader name, type and size, the file’s description, and **Save a copy** (name includes the file). Footnote: “lamina saves attachments exactly as they are and never opens or runs them. Check a file before you open it on your computer.” | `attachment.save` |

**States.** Outline loading for big files: the first 50 rows with a skeleton below, children loaded on expand. No outline: empty state “This file has no outline” / “Use Pages to move through it.” Skipped entries (repairs) are omitted and counted in R04. Thumbnails render at low priority; skeletons until drawn.

**Behaviour.** Selecting an outline row or thumbnail is a history jump (R07) and updates `#page=` through `buildFragment`. Tab choice, rail width and open state are remembered per browser. Saving an attachment writes the original bytes via the platform save dialog.

**Accessibility.** Tree keys: ↑ ↓ move, → expands or enters, ← collapses or goes to the parent, Home/End, Enter activates, type-ahead on titles; one row is in the tab order (drawn with the focus ring). Thumbnails use roving focus with ↑ ↓. Tabs are `role="tablist"` with arrow-key switching.

**Platform differences.** *Desktop:* same rail; widths are remembered per window. *Phone:* each tab is a sheet (R18, Reader B). *Tablet:* the rail is a docked panel, collapsed by default in portrait.

**Artboards.** `R06-rail-outline.dc.html`, `R06-rail-thumbnails.dc.html`, `R06-rail-attachments.dc.html`.

**Todos.** [C6a](../../todos/ready/C6a-thumbnails-outline-page-labels.md).

## R07 Links and history

**Purpose.** Show where a link goes before you follow it, return to where you were after any jump, and never leave the document for the web without asking.

**Entry points.** Hovering or focusing a link on a page; Back/Forward in the top bar; `Alt ←` / `Alt →` (Windows/Linux) and `⌘[` / `⌘]` (macOS); the browser’s own Back button.

**Layout.** Two **history buttons** (Back, Forward; 26 px icon buttons in a `role="group"` named “History”) sit before the page input. Link preview: a short-lived popover under the link (z 30). External links: a dialog (§7.18, 480 px).

**Content and controls.**

| Control | Does | Command id / shortcut |
| --- | --- | --- |
| Back | Returns to the previous position (page, scroll, zoom). Its name says where: “Back to page 2”; disabled when there is nothing to go back to | `nav.back` (`Alt ←`, `⌘[`) |
| Forward | Redoes a Back; disabled after a new jump | `nav.forward` (`Alt →`, `⌘]`) |
| Internal link | On hover or keyboard focus, after 300 ms, a **link preview**: a crop of the target page’s top, the target’s outline title (“4. Surveyor capacity”) and “Link to page 11 · Enter to go”. Click or Enter follows it. Named actions (Next page, First page…) show “Goes to the next page” and no crop | `link.follow` |
| External link | Opens the confirmation dialog: “Open this website?”, the full address with the host in 600 (punycode for international hosts), “It opens in a new tab. The site isn’t told which document you came from.”, **Copy link** (ghost), **Cancel** (default focus), **Open website** (primary) | `link.open-external`, `link.copy` |

**States.** *Preview* (drawn, with Back enabled after jumping from the contents page). *External confirmation* (drawn; the page text shows a shorter address than the real target, which is why the dialog shows the full one). *Unsupported:* `Launch`, `JavaScript`, `SubmitForm` and other actions are never run; hovering shows “This link runs an action lamina doesn’t run” and the action is listed in R04’s Not run. *Other file* (`GoToR`): “Links to another file, budget-2026.pdf, which lamina can’t open from here.” Only `http(s):` and `mailto:` can be opened.

**Behaviour.** Jumps (links, outline, thumbnails, go to page, find results) push a history entry; scrolling alone doesn’t. Entries are written to the URL fragment with `buildFragment`, so the browser’s Back and Forward work too and nothing is sent to the server. External links open with `noopener,noreferrer`.

**Accessibility.** Links are real anchors in page order with names from `/Contents` or the destination (“Link to page 11, 4. Surveyor capacity”). The preview is a `role="tooltip"` referenced by `aria-describedby`; it stays while hovered, Escape dismisses it (WCAG 1.4.13). The focus ring is drawn on the link.

**Platform differences.** *Web:* `Alt ←` and `⌘[` are also the browser’s Back; because history lives in the fragment, both do the same thing. *Desktop:* lamina owns these keys and also the mouse back/forward buttons; external links open in the default browser after the same dialog. *Phone:* long-press a link shows the preview in a sheet with **Go to page 11**; Back and Forward live in More; the OS back gesture goes back in lamina’s history first.

**Artboards.** `R07-link-preview.dc.html`, `R07-external-link.dc.html`.

**Todos.** [C6b](../../todos/ready/C6b-links-and-history.md).

## R08 Document properties

**Purpose.** Everything lamina knows about the file in one place, read from lamina’s own parser: description, version, pages and size, tagging, fonts, security and signatures.

**Entry points.** More › Document properties (`doc.properties`); the palette. No shortcut (`Mod D` is reserved by browsers, §8.8).

**Layout.** Dialog (§7.18 at form width, 560 px) with a fixed header (title “Document properties” over the file name, close) and footer, and a scrolling body of four sections with Heading 18 titles: **Description**, **Document**, **Fonts**, **Security**. Values are a two-column definition list (labels 12/600 `text-3`, 148 px).

**Content and controls.**

| Section | Rows (sample values) |
| --- | --- |
| Description | Title “Q3 Board Report” · Author · Subject · Keywords · Created “14 Oct 2026, 09:12” · Modified “28 Oct 2026, 16:04” · Made with “Northwind report template · saved by Northwind print service” (Info and XMP merged; XMP wins when they differ) |
| Document | PDF version 1.7 · Pages “24 · labelled 1–21, A-1–A-3” · Page size “US Letter, 8.5 × 11 in · all pages” (or “Mixed: 22 Letter, 2 Tabloid”) · File size “1.2 MB (1,262,480 bytes)” · Tagged “Yes · headings and reading order for screen readers” · Fast web view · Contains “2 attachments · no forms · no JavaScript” |
| Fonts | Table: Name · Type · In the file (“Embedded, subset”, “Not embedded · shown with a similar font”) |
| Security | Encryption · Permissions · Signatures |

Each value has a copy button on hover or focus (drawn focused on Title, “Copy title”); the footer has **Copy all** (ghost, plain text) and **Close**.

**States.** *Encrypted file:* Encryption “Password protected · AES 256-bit (revision 6)”, Permissions lists what the author restricted (“Printing: allowed · Copying text: not allowed”) with the note “lamina v0.1 shows these restrictions but doesn’t enforce them beyond this list.” *Signed file:* Signatures “2 signatures present, not verified” (validation comes with the editor’s signing work). *Long values* wrap; the dialog never scrolls horizontally.

**Behaviour.** Read-only; nothing changes in the file. Dates use the reader’s locale (`Intl.DateTimeFormat`).

**Accessibility.** `role="dialog"`, labelled by its title; sections are `section` with headings; the fonts table has `th scope="col"`. Copy buttons are named by the value they copy; a toast “Copied” confirms.

**Platform differences.** *Desktop:* same dialog (File › Properties… in the menu bar). *Phone:* a sheet at the large detent; the fonts table becomes a list.

**Artboards.** `R08-properties.dc.html` (written, not yet published to the canvas).

**Todos.** [C6c](../../todos/ready/C6c-print-download-properties.md).

## R09 Print and Download a copy

**Purpose.** Print pages that look the same in every browser, and save the file exactly as it was opened.

**Entry points.** More › Print… and `Ctrl P` / `⌘P` when a document is open (`doc.print`; lamina’s print replaces the browser’s); More › Download a copy (`doc.download`, no shortcut). The More menu is drawn open on `R09-download`.

**Layout.** Print: a dialog (520 px) over the dimmed shell, then a progress dialog. Download: a menu item, then the browser’s save, then a toast.

**Content and controls (print).**

| Control | Does |
| --- | --- |
| Pages (radio group) | All 24 pages · Current page (3) · Pages [range field, placeholder “1–5, 11, A-1”] with help “Use page numbers or labels, for example 1–5, 11, A-1.” Choosing the field selects its radio |
| Scale (select) | Fit to paper (default) · Actual size |
| Quality (select) | Standard (150 dpi) · High (300 dpi); help “High takes longer to prepare.” |
| Print comments and highlights (checkbox) | On by default; follows each annotation’s print flag |
| Page appearance notice | Shown when the page appearance isn’t Original (drawn: Sepia): “**You’re reading in Sepia.** Pages print in their original colours, as the author made them.” with the checkbox “Print in Sepia instead” |
| Footer | “Your browser’s print window opens next, to choose the printer.” · Cancel · **Print…** (primary) |

**States.**

- *Preparing* (drawn, `R09-print-progress`): “Preparing pages for print”, “lamina draws each page at print quality on this device. Your browser’s print window opens when all 24 pages are ready.”, a determinate bar “Page 9 of 24 · 38%”, **Cancel** (focused). Cancel stops rendering and frees memory.
- *Range error:* “There’s no page 30. This file has 24 pages (1–21, A-1–A-3).” under the field; Print stays enabled and focuses the field.
- *Too large for memory:* High quality on a very long range falls back to Standard with a notice “Printing at standard quality to stay within this device’s memory.”
- *Download done:* toast (§7.20) “Downloaded Q3-board-report.pdf”. An encrypted file downloads still encrypted; a remote file is streamed from the cache and its host without changes; the name comes from the URL or `Content-Disposition`, sanitised.

**Behaviour.** Printing renders pages in the worker into a hidden iframe (CSP-safe, [C6c](../../todos/ready/C6c-print-download-properties.md)) and calls the browser’s print; it changes nothing in the file. Download returns the original bytes, byte for byte, even if the file was repaired in memory (R04) — the menu item’s tooltip says “The file exactly as it was opened”.

**Accessibility.** The dialog uses a `fieldset`/`legend` for Pages and real `select`s; Enter prints; Escape cancels. The progress dialog announces every 25%. The More menu is `role="menu"` opened from a button with `aria-haspopup="menu"` and `aria-expanded`; ↑ ↓ move, Enter runs, Escape closes and returns focus; the active item uses the palette’s active-row style.

**Platform differences.** *Desktop:* print hands off to the native print dialog with a preview (S05, [D4c](../../todos/ready/D4c-native-printing.md)); the menu item is File › Save a copy… with a native save panel. *Phone:* Print uses the OS share/print sheet after the same preparing step; Download becomes Share › Save to Files. *Windows/Linux vs macOS:* chips `Ctrl P` / `⌘P`.

**Artboards.** `R09-print.dc.html`, `R09-print-progress.dc.html`, `R09-download.dc.html` (written, not yet published to the canvas).

**Todos.** [C6c](../../todos/ready/C6c-print-download-properties.md).

## New components

All proposed; built from the design language’s tokens.

- **Icons** (24 grid, stroke 1.5, add to §5): File `<path d="M6.5 3.5h7l4 4v13h-11z"></path><path d="M13.5 3.5v4h4"></path>` · Link `<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"></path><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"></path>` · External link `<path d="M13.5 4.5h6v6"></path><path d="M19.5 4.5L11 13"></path><path d="M17 13.5v6H4.5V7h6"></path>` · Lock `<rect x="5.5" y="10.5" width="13" height="9" rx="1.5"></rect><path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5"></path>` · Paperclip `<path d="M16.5 7.5l-7 7a2 2 0 0 0 2.8 2.8l7.2-7.2a4 4 0 0 0-5.7-5.7l-7.2 7.2a6 6 0 0 0 8.5 8.5l5.4-5.4"></path>` · Download `<path d="M12 4.5V15M8 11l4 4 4-4"></path><path d="M5 19.5h14"></path>` · Printer `<path d="M7 9.5V4.5h10v5"></path><rect x="4.5" y="9.5" width="15" height="7" rx="1"></rect><path d="M7 14.5h10v5H7z"></path>` · Info `<circle cx="12" cy="12" r="8"></circle><path d="M12 11v5M12 8v.5"></path>` · Forward `<path d="M9 5l7 7-7 7"></path>` · Chevron up `<path d="M7 14l5-5 5 5"></path>` · Chevron right `<path d="M10 7l5 5-5 5"></path>` · List `<path d="M5 7h14M5 12h14M5 17h9"></path>`.
- **Drop zone.** `surface`, 1 px dashed `control-border`, radius 6, padding `40px 24px`, centred 32 px File icon, title Newsreader 20/500, hint 14 `muted`, buttons gap 8. Drag-over: full-window `accent-tint` overlay with a 2 px dashed `accent` inset and one line of text.
- **File list row** (recent files). Full-width button, min-height 56 (64 on touch), padding 8, gap 14: 20 px icon, Newsreader 15/500 name over a 12 `muted` meta line, date right; rows separated by `hairline`.
- **Error page.** The empty state (§7.21) with a 32 px `danger` warning icon, the code in 12 `muted` below the sentence, `role="alert"`; actions: one primary and one ghost.
- **Details panel.** The inspector’s shell (§7.11) used for a document-level explanation that isn’t a selection (repairs; also suits R13’s network panel). Opened from a status-bar item, which shows a pressed state (`accent-tint` fill, `aria-expanded="true"`) while it is open.
- **Find bar.** Secondary-toolbar slot, `role="search"`: a 28 px field (300 px, search icon, count inside in `muted` tabular), Previous/Next 28 px icon buttons, a divider, toggle buttons (§7.24 without icons), a context note and Close. **Match fills:** `find-match` `#F6E3BC`, `find-current` `#EDBF6A` + 2 px `accent` outline (proposed tokens).
- **Results list.** Page group label 12/600 `text-3` (“Page 3 · Repairs backlog”), result links 13/1.45 with the match in 600, padding `6px 8px`, radius 4; current result `accent-tint` + inset `accent` ring.
- **Rail width with four tabs.** The rail opens at 280 px when it has four tabs (Results or Attachments added); below that width the last tabs move into a **More tabs** button (`aria-haspopup="menu"`) at the end of the tab row.
- **Outline tree.** §7.7’s rows as a WAI-ARIA tree (`ul role="tree"`, `li role="treeitem"` with `aria-level`, `aria-expanded`); the chevron is decoration inside the row, toggled by click or ← →, so there is no nested button.
- **Page input with label.** Widens to fit the label (44 px for “A-1”); the physical position follows in `muted`: `(22 / 24)`.
- **History buttons.** Back and Forward 26 px icon buttons in a `role="group"` before the page input; names say the destination (“Back to page 2”); disabled uses the `disabled` token.
- **Link preview.** Popover (z 30) 248 px, `surface`, `chip-border`, radius 6, floating shadow, padding 12, gap 10: a 92 px crop of the target page’s top (thumbnail shadow) over the target title (Newsreader 15/500) and a 12 `muted` line. `role="tooltip"`, hoverable, Escape dismisses.
- **Address box.** For URLs shown before acting on them: `field` white, `hairline` edge, radius 4, padding `10px 12px`, wraps anywhere, host in 600 `text`, the rest `text-3`.
- **Password field.** The text input (§7.19) with an inline Show/Hide text toggle (13/600 `accent`, `aria-pressed`) inside the right edge.
- **Properties list.** Definition list in a two-column grid (148 px labels 12/600 `text-3`, values 14 `text`, extra facts in `muted`), gap `8px 16px`; per-value copy icon buttons 24 px; sections headed with Heading 18.
- **Menu.** `role="menu"` popover (z 30) under its button: `surface`, `chip-border`, radius 6, floating shadow, padding 4, width 260; items 32 high, padding `0 10px`, gap 10, 16 px icon (items without an icon indent to 36), label, shortcut chip right; separators 1 px `hairline` with 6 px side margins; the active item uses the palette’s active-row style (`accent-tint`, inset `accent` ring). The opening button shows the `pressed` fill.
- **Phone find header and find toolbar.** The top bar becomes a 40 px search field (16 px text, count inside) with **Done**; the bottom bar becomes a three-column toolbar (Previous · Results · Next) with the action-bar sizes (§7.15).
