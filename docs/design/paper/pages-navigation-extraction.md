# Paper: pages, navigation and extraction

Screens O09–O17 of the [screen inventory](screens.md#o-edit-session-organise-navigation-and-extraction-e1e3): page geometry, page labels, bookmarks, links, the initial view and article threads, attachments, the object inspector, export and extraction, and organising pages on a phone. They implement [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and [§E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) in the [Paper design language](design-language.md).

- **Canvas:** [lamina Paper — Pages, navigation and extraction](https://claude.ai/artifact/XxRzeJ46jd6mBFhp7HevYf) (private until shared). One row per screen, ordered by ID, with a sticky note per row.
- **Shells:** desktop screens are drawn in the macOS window frame from the `Desktop` foundation board; the web export state uses the web shell from `Main`; phone screens use the `Mobile-read`/`Mobile-select` shell.
- **Sample file:** `Q3-board-report.pdf`, Northwind Housing Trust, 24 pages, page 3 "2. Repairs backlog", page labels `1`–`21` then `A-1`–`A-3` for Appendix A (pages 22–24), and 3 attached files (1.3 MB), the same facts as [Reader A](reader-a.md). The certified state adds a certification by Northwind Housing Trust (form filling and signing allowed).
- **Scope note:** Organise mode's page grid, insert, merge and split (O06–O08) belong to the Edit session and organise area ([edit-session-organise.md](edit-session-organise.md)). This area adds geometry, labels and navigation tools to that mode and draws the grid only as a backdrop.

Rules that hold for every screen here:

- **Top bar.** Every board uses the editor top bar of [§7.1](design-language.md#71-top-bar): file name and save state, a divider, Undo and Redo, the mode switcher, page input `n / 24`, zoom −/%/+, Find, View (view settings), Commands, Save (only with changes) and More. Undo names the last step ("Undo add bookmark") and is disabled, like Redo, when there is nothing to undo. In Read the Reader toggle follows View. In Organise the page input and zoom give way to the thumbnail size control (− 100% +); while the Crop tool shows a single page (O09) the same control zooms that page and its buttons are named Zoom out and Zoom in.
- **Organise toolbar ([§8.1](design-language.md#81-modes-and-their-secondary-toolbars), shared with O06).** Insert ▾ · Rotate left · Rotate right · Extract · Delete │ Move to page… │ Page setup ▾ (Crop, Resize…, Page boxes…, Page labels…, Make landscape, Make portrait) · Bookmark ▾ (Add bookmark, Create a contents page…, Named destinations…) · More page actions (⋯) · context note. Insert, Rotate, Extract, Delete, Move to page, Crop (on Page setup) and Bookmark use the icons of [§5](design-language.md#5-iconography). While the Crop tool is on, Page setup shows the selected style (`accent` border on `accent-tint`). It wraps onto a second row when narrow.
- **Edit toolbar ([§8.1](design-language.md#81-modes-and-their-secondary-toolbars), from A01).** Select · Edit text · Add text · Image · Shape ▾ · Link │ Page marks ▾ · Replace… │ Redact · Protect · Tools ▾ (Form design, Actions, Accessibility, Print production) · context note. Tools ▾ uses the Tools grid icon, not the wrench, which means "Repaired on open" in the status bar.
- **Rail tabs ([§8.10](design-language.md#810-rail-tab-set-proposed)).** Read boards show Pages · Outline · Attachments at 220 px (the sample has attachments); Outline is the file's bookmarks and there is no separate Bookmarks tab. Organise has no Pages tab, because the grid is the pages: its rail is Outline · History · Attachments at 220 px, and bookmark editing (O11) happens in its Outline tab. Threads is added only when a file has article threads (O13); with four or more tabs the rail is 280 px, three tabs show, and a 28 px More tabs button (⋯, [§7.5](design-language.md#75-segmented-tabs-rail-tabs)) outside the tablist holds the rest. The tab of the current task takes the third slot, and the selected tab always stays in the row. Boards that need the room for a dialog or the page close the rail (Sidebar `aria-pressed="false"`).
- **Explicit save.** Every edit below is an op in the op log (ADR 0006): it adds one unsaved change and one undo step, and nothing reaches the file until Save. Read-only views (O13 threads, O15, O16) never mark the file changed.
- **Read mode is read-only.** Panels in Read show facts and offer "Edit in Organise" (bookmarks, attachments) or "Edit in Edit" (links); nothing in Read changes the file.
- **Signed files (ADR 0007).** With approval signatures only, edits save as an incremental update (`AppendOnly`) and the signatures stay valid: no banner, and the status bar shows "Signed file. Saving adds your changes after the signature; the signature stays valid." On a `Conflict` (a certification that doesn't allow the change, a FieldMDP/`/Lock`ed field, or an op on a page holding a signed signature field) the op is checked with `would_violate` before it applies, nothing changes, and the constraint banner appears under the toolbar naming each signature, with Save a copy, Apply anyway… and Details. Apply anyway… opens an alert dialog that lists each affected signature by signer; confirming applies the op with `invalidate_signatures`. Geometry, labels, bookmarks, named destinations, the initial view and attachments are class `PageStructure` or `Document` (a conflict under every certification level); links are class `Annotation` (allowed under DocMDP P3). Files certified "no changes" (P1) show the banner on entering any editing mode. Drawn in `O09-page-geometry-signed.dc.html`, following the constraint banner and signature rows of [§7.13](design-language.md#713-save-note-and-save-banner-signed-file).

## O09 Page geometry

**Purpose.** Crop, resize, set page boxes and orientation for one or many pages, and say plainly that cropping hides content without removing it.

**Entry points.** Organise toolbar › Page setup ▾ › Crop, Resize…, Page boxes…, Make landscape, Make portrait; the page context menu has the same items; palette "Crop pages" (`pages.crop`), "Resize pages" (`pages.resize`), "Set page boxes" (`pages.edit-boxes`), "Make landscape" (`pages.make-landscape`), "Make portrait" (`pages.make-portrait`). No shortcuts.

**Layout.** Desktop shell, Organise mode, the Organise toolbar with Page setup shown as active and the note "Cropping page 3 · drag the edges or type the margins". Rail (220 px): Outline · History · Attachments, Outline open. Canvas: the selected page at true size with the crop overlay (the Crop tool swaps the page grid for the single page until it is turned off). Inspector "Page geometry" with tabs Crop · Size · Page boxes. Resize opens a dialog over the dimmed page grid, with the same rail.

**Content and controls.**

- Crop overlay on the page: the area being cut is dimmed with `rgba(31,29,26,0.38)`; the kept area has a 2 px `accent` frame and eight 8 px white handles with a 1.5 px `accent` border and a 24 px hit area ([§7.32](design-language.md#732-page-overlays-proposed)). Dragging a handle or an edge changes the matching margin.
- Inspector › Crop: "Remove from each edge" with Top, Bottom, Left, Right number inputs (mm or in by locale), a live "Visible area 188 × 226 mm (page 216 × 279 mm)" line; "Apply to": This page / All 24 pages / Pages [range]. Warning notice (`warning-tint`): "**Cropped content stays in the file.** Cropping only hides the edges: the running header and page number can still be copied, searched and extracted. To remove them, use Redact." Actions: Crop page (primary), Reset (ghost).
- Inspector › Size: the same fields as the resize dialog for the selected pages; orientation as two radios.
- Inspector › Page boxes (advanced): Media, Crop, Bleed, Trim and Art boxes as four numbers each (left, bottom, right, top in pt), each of the last three with a "Same as crop box" checkbox.
- Resize dialog "Resize pages": subtitle "24 pages · now US Letter, 216 × 279 mm"; New size select (A4, US Letter, US Legal, A5, Custom: Custom shows width and height inputs); Orientation radios; Content radios "Scale to fit the new size" (default, `Scale`) and "Keep the content size" (`Extend`/`Anchor`); a preview with the current page dashed (84 × 109) and A4 solid (82 × 116) at the same scale; a 3 × 3 Anchor radio grid, disabled until "Keep the content size" is chosen; the sentence "Bookmarks, links and the opening view are moved with the content, so they still land on the same place. You can undo this in one step."; Cancel, Resize 24 pages. Close button "Close Resize pages".

**States.**

- Default: overlay hidden until Crop is on; the inspector opens on the Crop tab.
- Margins larger than the page: the field turns `danger` with "Leaves no visible area"; Crop page disabled.
- Mixed sizes in the selection: the visible-area line reads "Sizes differ: margins are removed from each page"; the resize dialog's subtitle reads "24 pages · 3 sizes".
- Certified file (third artboard): with Apply to set to All 24 pages, Crop 24 pages is a conflict, so nothing is cropped; the banner reads "**Cropping would invalidate the certification.** Northwind Housing Trust certified this file for form filling and signing only, so nothing has been cropped." with Save a copy · Apply anyway… · Details. Apply anyway… opens the alert dialog "Crop and invalidate 1 signature?" listing "Northwind Housing Trust · Certification · form filling and signing allowed · 24 Oct 2026, 09:30 · Valid", with Cancel (focused, drawn with the ring) and "Crop and invalidate signature" (danger, filled), and a `warning-tint` note pointing to Save a copy… to keep the certification. The status bar's middle slot shows the certified-file note "Certified file. Northwind Housing Trust allows form filling and signing only."; the toolbar note reads "Cropping 24 pages · drag the edges or type the margins".
- Done: toast "Page 3 cropped · Undo"; the top bar shows "1 unsaved change".

**Behaviour.** Crop writes `/CropBox` (`CropPages`); the op report's `CroppedContentRemains` is why the notice is always shown, not only after applying. Resize runs `ResizePages` with the chosen mode; annotations, links, bookmarks, named destinations and the open action are transformed with the page (E2f). Each apply is one undo step. Rotate uses `/Rotate`; "Make landscape" asks nothing extra when the content can be rotated, and opens the resize dialog when the page must change shape.

**Accessibility.** The numeric fields are the non-drag alternative to the handles (WCAG 2.5.7); the handles are `aria-hidden`. Focus order: tabs → margins → Apply to → Crop page. Arrow keys on a focused margin field change it by 1 mm (Shift: 10 mm). The warning notice is `role="status"`. The anchor grid is a `radiogroup` of labelled buttons (Top left … Bottom right) and is `aria-disabled` while unused. The page grid behind the resize dialog is a `listbox` with `aria-multiselectable` whose thumbnails are `option`s, as in O06. The alert dialog uses `role="alertdialog"` and starts on Cancel.

**Platform differences.** Web and desktop identical. Phone: crop and resize are in O17's More menu as full-height sheets with the same fields (no handles). Tablet: handles keep their 24 px hit areas and grow to 44 px for touch.

**Artboards.** `O09-page-geometry.dc.html` (crop with warning), `O09-page-geometry-resize.dc.html` (resize dialog), `O09-page-geometry-signed.dc.html` (certified file: banner and Apply anyway confirm).

**Todos.** [E2f](../../todos/ready/E2f-page-geometry.md), [E2s](../../todos/ready/E2s-resize-and-orientation.md), [E2g](../../todos/ready/E2g-page-labels.md).

## O10 Page labels editor

**Purpose.** Define the page names readers show (roman front matter, arabic body, a prefixed appendix) without touching the printed page.

**Entry points.** Organise toolbar › Page setup ▾ › Page labels…; page context menu › Page labels…; palette "Edit page labels" (`pages.edit-labels`).

**Layout.** Dialog (max 560) over the dimmed Organise shell (full top bar with Undo/Redo, the Organise toolbar with the note "24 pages", rail Outline · History · Attachments, page grid backdrop marked `aria-hidden`). The sample file is labelled 1–21 and A-1–A-3; the board shows the dialog after the user has made pages 1–2 roman (i, ii), so the body now starts at 1 on page 3, and is typing the appendix prefix.

**Content and controls.**

- Intro: "The names pages show in lamina and other PDF readers. They don’t change what is printed on the page."
- Ranges table (`<table>` with column headers From page · Style · Prefix · Start at · remove): one row per range. The first row always starts at page 1 (its From field is disabled and it can't be removed). Style select: 1, 2, 3 · i, ii, iii · I, II, III · a, b, c · A, B, C · No number (prefix only).
- "Add range" (ghost with plus) adds a row after the last one.
- Preview: a compact grid of "page → label" pairs for the first pages, the pages around each range start and the last pages; labels in the range being edited are shown in `accent` 600. A caption names the elided run ("Pages 6–19 continue 4–17").
- Actions: Cancel, Apply labels (`SetPageLabels`). Close button "Close Page labels".

**States.** Two rows starting on the same page: both From fields turn `danger`, "Two ranges can’t start on page 22", Apply disabled. A file with no labels opens with one row "1 · 1, 2, 3 · (no prefix) · 1". Duplicate labels are allowed (the spec allows them) and noted in `muted` ("Pages 1 and 3 are both labelled 1"). After apply: thumbnails, the page input and the status bar show labels; with the drawn ranges page 5 reads "Page 3 (5 of 24)".

**Behaviour.** One op and one undo step for the whole table. Later page moves keep labels through `PageLabels::remap` (E2j); the dialog always reflects the current tree. Signed files: as the area rules.

**Accessibility.** Each input has an `aria-label` naming its row and column ("Range 3, prefix"). Remove buttons are labelled "Remove range 3". The preview list is an `<ol aria-label="Labels by page">`. Focus starts on the first editable field; after "Add range", focus moves to the new row's From field.

**Platform differences.** Identical on web and desktop. Phone: a full-height sheet with one card per range (fields stacked, 44 px), preview below.

**Artboards.** `O10-page-labels.dc.html`.

**Todos.** [E2q](../../todos/done/E2q-page-labels-op.md), [E2g](../../todos/ready/E2g-page-labels.md).

## O11 Bookmarks editor

**Purpose.** Create, rename, nest, reorder, style and retarget the file's bookmarks (the PDF outline), manage named destinations, and generate a contents page.

**Entry points.** Rail › Outline tab, editable in Organise (in Read and other modes it is read-only with an "Edit in Organise" link at its foot); Organise toolbar › Bookmark ▾ › Add bookmark, Create a contents page…, Named destinations…; the selection mini toolbar's More › "Add bookmark here"; palette "Add bookmark" (`organise.add-bookmark`), "Create a contents page" (`organise.create-contents-page`), "Named destinations" (`organise.named-destinations`). The rail header's More menu repeats these and adds Expand all, Collapse all and Sort by page.

**Layout.** Desktop shell, Organise mode, the Organise toolbar with the note "8 bookmarks"; the top bar shows "1 unsaved change", Save and "Undo add bookmark". Rail (220 px, resizable wider) with Outline · History · Attachments, Outline open. Canvas shows the destination; inspector "Bookmark". The contents-page generator and named destinations are dialogs over the same shell.

**Content and controls.**

- Rail header: "Bookmarks in this file", Add (icon, "Add bookmark"), More (icon, `aria-haspopup="menu"`).
- Tree (`role="tree"`, rows `treeitem` with `aria-level`, `aria-expanded`): Newsreader rows like the outline row (§7.7), page number right. Selected row: `accent-tint` with the stroked drag grip of §5. Inline rename: the row turns into a text input (Newsreader 14) with the focus ring; Enter commits, Escape cancels.
- Footer hint: "Drag to reorder or nest. With the keyboard: ⇧⌘↑ ⇧⌘↓ move, ⇧⌘← ⇧⌘→ change level." (Windows/Linux: Ctrl Shift + arrows, matching E2h's page moves.) They work only while the tree has focus.
- Destination marker on the page: a dashed 2 px `accent` line at the target's top with a 16 px `accent` tag carrying the bookmark title, drawn in the gap between paragraphs, which opens to 28 px while the marker shows, so it never covers text (page overlay, `aria-hidden`).
- Inspector: "Goes to" (page and position in words: "Page 3 · second paragraph"); "Use current view" (`organise.set-bookmark-destination`) with a one-line hint; "Zoom when opened" select (Keep the reader’s zoom · Fit page · Fit width · 100%; maps to `/XYZ` with null zoom, `/Fit`, `/FitH`, `/XYZ … 1`); Style toggles Bold and Italic (`/F` flags); Colour swatches Default, Grey, Red, Blue, Green (`/C`; Default removes the key) with "Some readers ignore bookmark colours."; "Show nested bookmarks when opened" checkbox (`/Count` sign; disabled when the bookmark has no children); actions Go to (secondary) and Delete bookmark (danger ghost, `organise.delete-bookmark`).
- Contents page dialog: Insert (Before page 1 · After page 1 · At the end), Levels (Top level only · Two levels), Page title (default "Contents"), "Add a “Contents” bookmark" checkbox, a preview of the generated page with dot leaders and the page labels readers will see after the insert (inserted before page 1, the contents page takes label 1, every later page number in the body goes up by one, and the appendix keeps A-1 to A-3), the consequence sentence "Adds 1 page before page 1, so every later page number goes up by one; the appendix keeps A-1 to A-3. Bookmarks, links and page labels follow their pages.", Cancel, Create contents page (`GenerateToc`). Close button "Close Create a contents page". The shell behind shows "1 unsaved change" from the bookmark added before.
- Named destinations dialog (third artboard, max 560): intro "Places in this file that links and bookmarks can point to by name. Other files may link to these names too."; a table Name (mono) · Goes to (link) · Used by ("2 links · 1 bookmark", "Not used in this file") · Rename and Delete icon buttons; inline rename (drawn on `repairs-backlog`, with the focus ring) with the hint "Enter to rename, Esc to cancel. Links in this file follow the new name; links from other files to the old name will stop working."; "Add for the current view"; Done. Each rename, add or delete is one op (`SetNamedDestination`, `DeleteNamedDestination`). The board's rows: `chair-summary`, `repairs-backlog`, `repairs-by-region` (not used), `surveyor-capacity`, `data-notes`. Close button "Close Named destinations".

**States.** Empty: empty state "No bookmarks yet" / "Add one for the page you’re on, or create them from a contents page." with Add bookmark. A broken outline repaired on open: a notice at the top of the tree "Some bookmarks were damaged; lamina rebuilt the list" (warning tint). Dragging: a 2 px `accent` insertion line indented to the target level; dropping onto a row's middle third nests. A bookmark whose target page was deleted shows "Page removed" in `danger` in place of its number and in the inspector, with "Use current view" to fix it. Deleting a named destination that is used asks first ("2 links and 1 bookmark use repairs-backlog. They will stop working." · Cancel · Delete name).

**Behaviour.** Each add, rename, move, nest, restyle, retarget or delete is one undo step (`AddOutlineItem`, `UpdateOutlineItem`, `MoveOutlineItem`, `DeleteOutlineItem`). Clicking a row selects it and navigates; double-click or F2 renames. "Use current view" stores the current page, scroll position and (if not "Keep") zoom. The contents page is one composite op and one undo step.

**Accessibility.** Tree keyboard model from the APG tree pattern: ↑/↓ move, →/← expand/collapse, Enter goes to the destination, F2 renames, Delete removes (with toast Undo). Move and level shortcuts are the non-drag alternative. Position changes are announced politely ("Moved under 2. Repairs backlog, position 2 of 2"). Colour swatches have names and `aria-pressed`. The destination marker is decorative; its information is in the inspector.

**Platform differences.** Identical on web and desktop (desktop menu bar adds Bookmarks › …). Phone: the Outline sheet gets an Edit button; edit mode reorders via "Move…" sheets and a rename field, no drag.

**Artboards.** `O11-bookmarks.dc.html` (editor with inline rename and inspector), `O11-bookmarks-toc.dc.html` (create a contents page), `O11-named-destinations.dc.html` (named destinations with an inline rename).

**As built (web, E3b).** Organise mode doesn't exist yet ([E2h](../../todos/ready/E2h-page-organiser-ui.md)), so the editor lives in the rail's Outline tab: Read shows the tree as before with an **Edit bookmarks** button at its foot, which turns the tab into the editor until **Done editing**.

Differs from the boards:

- The selected bookmark's details sit under the tree in the rail, not in an inspector, and have no "Zoom when opened" select: Use current view and Add bookmark store the place at the top of the view (`/XYZ` with the reader's zoom kept).
- Titles show their bold, italic and colour only while editing, which keeps the reader's startup script within its budget. The colour is a dot before the title, not the text's colour: a file's colours are chosen for white paper and fail contrast on the dark theme.
- "Used by" counts this file's bookmarks only, not its links, so every delete asks first: links and other files may use the name too. The rename field is named "New name for …", apart from the Rename button.
- The rebuilt-outline notice shows after the first edit that rebuilds it, not on open, since reading a file changes nothing. A failed change, or Add bookmark with no page in view, shows a notice above the tree; deleting a bookmark has no toast, and Undo or the History tab takes it back.
- The drag grip is a 44 px button at each row's end, shown on hover, focus or selection; dropping onto a row nests it, with React Aria's drop indicator in place of the indented accent line.

Not built yet:

- The rail header's More menu (Expand all, Collapse all, Sort by page, Create a contents page…); the contents page is [E3l](../../todos/ready/E3l-contents-page.md)'s.
- The destination marker on the page, and "Page removed" for a bookmark whose page was deleted (page deletion came with [E2b](../../todos/done/E2b-page-structure-ops.md), which sends a bookmark on to a surviving page).
- The palette commands (`organise.add-bookmark`, `organise.named-destinations`, `organise.set-bookmark-destination`, `organise.delete-bookmark`) and the selection toolbar's Add bookmark here, which join Organise's commands.
- The polite announcement of a move ("Moved under …"), and the phone's Edit button with its Move… sheets.

**Todos.** [E3b](../../todos/done/E3b-bookmarks-and-destinations.md).

## O12 Link editor

**Purpose.** Make a link from selected text or a drawn area and point it at a page, a named destination, a web address or an email address.

**Entry points.** Edit toolbar › Link (draw a box, `tool.link`); selection mini toolbar › More › "Make link" (`edit.make-link`); selecting an existing link in Edit mode; palette "Add link" (`edit.add-link`, places a default box at the caret or page centre), "Make link from selection" (`edit.make-link`). No shortcuts.

**Layout.** Desktop shell, Edit mode, with "Undo add link" in the top bar. The Edit toolbar with Link pressed and the note "Draw a box, or select text and press Make link · 2 links on this page". Rail closed in the drawing (Sidebar not pressed; any tab can be open). Inspector "Link".

**Content and controls.**

- On the page, while the Link tool is on: every existing link shows a 1 px dashed `muted` outline; the selected link has the 2 px `accent` outline and an `accent` 8% tint computed from the accent, so it follows an accent change (text links follow the glyph quads; drawn links show four 8 px corner handles).
- Inspector: the linked text as a quote (text links only) or "drawn area over the chart" in the context line. "Goes to" radios: A page in this file · A named destination · A web address · An email address.
  - Page: number input with the page's outline title beside it, and "Pick a place on the page…" (click a spot on any page to set an `/XYZ` position).
  - Named destination: a searchable select of the file's names plus "New name…" (opens O11's named destinations dialog with a new row).
  - Web address: text input; only `https:` and `http:` are accepted (and `mailto:` under Email).
  - Email: address and optional subject.
- "Zoom when followed" select (page and named destinations).
- Description textarea (`/Contents`), help "Read aloud by screen readers."
- Actions: Follow link (secondary, `nav.follow-link`), Delete link (danger ghost, `edit.delete-link`).

**States.** Rejected address (second artboard, `O12-link-editor-error`): `javascript:` and other schemes show the field in `danger` with "This address would run a script, so it can’t be used. Use an address that starts with https:// or http://."; the link keeps its last valid target and Follow link is disabled. Empty target on a new link: Follow link disabled, and leaving the inspector with no target removes the new link (toast "Link removed: it had no target · Undo"). A note under web targets: "Readers of this file are asked before a web link opens." (the C6 external-link confirmation). Script link found in an opened file (E3c "flagged on read"; described, not drawn): a link whose action is `javascript:` or another script scheme shows a `danger` dashed outline on the page in Edit, and the inspector's Goes to reads "Runs a script: javascript:openDashboard()" in `mono` with the notice "This link would run a script. lamina never runs it." (`danger-tint`); Follow link is disabled, and the only actions are choosing a new target or Delete link. In Read and the other modes, clicking it does nothing beyond the same sentence in the link preview, so nothing runs anywhere.

**Behaviour.** Drawing or making a link creates it (`AddLink`) and selects it; every inspector change is `UpdateLink`, committed on blur or Enter, one undo step each. Delete removes it. Signed files follow the area rules (allowed under P3; a conflict under P1/P2).

**Accessibility.** "Make link" works entirely from a keyboard text selection; drawn areas can be created with "Add link" and moved or resized with arrow keys (Shift+arrows resize). The error text is linked with `aria-describedby` and `aria-invalid`. The Description field is called out because it is what assistive tech reads for the link.

**Platform differences.** Identical on web and desktop. Phone: links are edited from the selection action bar's More › Make link and a sheet with the same fields; no drawing.

**Artboards.** `O12-link-editor.dc.html` (text link to a page), `O12-link-editor-error.dc.html` (drawn area, rejected `javascript:` address).

**As built (web, E3c).** The reader has no Edit mode yet, so the Link tool's toggle sits in the top bar, after Undo and Redo, and the editor opens in a panel by the link rather than in an inspector.

Differs from the boards:

- A new link is written once it has somewhere to go, so a box closed without a target leaves nothing behind and needs no "Link removed" toast.
- A page target goes to the whole page (`/Fit`): there is no "Zoom when followed" select and no "Pick a place on the page…".
- Make link is a button of the selection toolbar, not an entry of its More menu.
- The tool's note ("Draw a box, or select text…") is Make link's reason in the palette when no text is selected, not a line in a toolbar.

Not built yet:

- The palette's `edit.add-link` (a default box at the caret) and `tool.link`, and moving or resizing a drawn link with the arrow keys and its corner handles.
- "New name…" in the named destination select.
- The phone's Make link sheet.
- A script link's `danger` outline outside the Link tool; in Read mode the link preview already says such a link never runs.

**Todos.** [E3c](../../todos/done/E3c-links.md).

## O13 Initial view and article threads

**Purpose.** Choose how the file opens in any reader, and list and follow the file's article threads.

**Entry points.** Initial view: in Organise, More page actions (⋯) › Initial view…, and More › Document › Initial view…; palette "Set initial view" (`organise.initial-view`). In Read, Document properties (R08) shows the current initial view read-only with "Edit in Organise". Threads: rail › Threads tab (shown only when the file has threads); palette "Next thread part" / "Previous thread part" (`nav.next-thread-part`, `nav.previous-thread-part`).

**Layout.** Initial view: dialog over the dimmed Organise shell (Undo/Redo, the Organise toolbar with the note "24 pages", rail Outline · History · Attachments, page grid backdrop marked `aria-hidden`). Threads: desktop shell in Read mode, rail 280 px with Pages · Outline · Threads and the More tabs button (⋯) holding Attachments, Threads in the third slot as the current task; page overlay on the current part.

**Content and controls.**

- Initial view dialog: intro "How this file opens for anyone, in lamina and in other PDF readers."; Open at page (number + outline title); Zoom (Fit width · The reader’s default · Fit page · 100%…); Page layout radios: One page at a time · Continuous scroll · Two pages side by side · Side by side, scrolling · Side by side, cover page alone · Side by side scrolling, cover alone (`/SinglePage`, `/OneColumn`, `/TwoPageLeft`, `/TwoColumnLeft`, `/TwoPageRight`, `/TwoColumnRight`); "Show when opened" select: Bookmarks panel · Just the page · Page thumbnails · Attachments panel · Layers panel · Full screen (`/UseOutlines`, `/UseNone`, `/UseThumbs`, `/UseAttachments`, `/UseOC`, `/FullScreen`); "Reader window" checkboxes (show the document title, fit the window to the first page, hide the reader's toolbar and menus → `/ViewerPreferences`) with "These are requests: some readers, and lamina’s own settings, may override them."; "Use the current view" (ghost), Cancel, Apply (`SetInitialView`). Close button "Close Initial view".
- Threads panel: one expandable entry per thread (title from the thread's info, or "Untitled thread", and "3 parts · pages 3, 5, 6"); its parts as links "Part 1 · page 3"; Previous part / Next part buttons; the note "Threads join the parts of an article across pages. lamina follows them; editing threads comes later."
- On the page: the current part (bead) has a 2 px `accent` outline and a tag "Part 1 of 3 · continues on page 5"; the status bar's page slot reads "Page 3 · Repairs backlog in depth, part 1 of 3".

**States.** Initial view with nothing set: every control shows the reader default and Apply is disabled until something changes. Threads: tab hidden when there are none; a bead pointing to a missing page is listed as "Part 4 · page missing" in `muted` and skipped by Next part.

**Behaviour.** Apply is one undo step; lamina honours the saved settings the next time the file opens, unless the user turned off "Use the file’s initial view" in View settings (R10). Following a thread scrolls to the bead and keeps the column at the top; it adds history entries so Back returns (C6b).

**Accessibility.** The bead outline is decorative; the panel's `aria-current="location"` and the status bar (`aria-live="polite"`) carry the position. Next/Previous part are real buttons with disabled states. The More tabs button is a 28 px icon button outside the tablist with `aria-label="More tabs"`, `aria-haspopup="menu"` and `aria-expanded`, as in Reader A.

**Platform differences.** Identical on web and desktop; "Resize the window to fit the first page" has no effect in lamina's web app (the help text says so on web). Phone: threads in the Outline sheet as a second segment; initial view in More › Document.

**Artboards.** `O13-initial-view.dc.html`, `O13-article-threads.dc.html`.

**Todos.** [E3d](../../todos/ready/E3d-initial-view-and-threads.md).

## O14 Attachments panel

**Purpose.** See and save out files embedded in the PDF, including associated files tied to a page (PDF/A-3, Factur-X); in Organise, add, describe and delete them.

**Entry points.** Rail › Attachments tab (any mode); More › Attachments; palette "Save attachment…" (`file.save-attachment`); in Organise, "Attach file…" (`organise.attach-file`), drag files onto the panel, and "Delete attachment" (`organise.delete-attachment`).

**Layout.** The board shows Read mode: desktop shell, rail 220 px with Pages · Outline · Attachments, inspector for the selected attachment, everything read-only ([§8.11](design-language.md#811-read-mode-is-read-only-proposed)).

**Content and controls (Read, drawn).**

- Rail header: count and total size ("3 files · 1.3 MB") and the "Edit in Organise" link (drawn with the focus ring).
- List (`role="listbox"`): File icon (§5), name (ellipsis), "86 KB · Spreadsheet", and for associated files a neutral badge naming the relationship and target ("Source of page 3", "Data for page 11"). Selected: `accent-tint` with an `accent` inset ring. Portfolios (`/Collection`) show their schema columns as extra metadata lines.
- Foot note: "Attachments stay inside this PDF, on this device. To add, describe or delete them, switch to Organise."
- Inspector: file name as title, "Attachment · associated with page 3"; Description as text, Belongs to (a link to the page), Relationship as text ("Source" with "The file this page was made from."), facts (Type, Size with exact bytes, Modified, Checksum badge "Matches" or warning "Doesn’t match"); actions "Save to disk…" (desktop) / "Download" (web) and "Edit in Organise".

**Organise (described, same layout).** The rail header shows "Add file…" (ghost with plus) in place of the link, and the compact drop zone of [§7.21](design-language.md#721-empty-state) sits at the foot (radius 6, dashed `control-border`, paperclip, "Drop files here to attach" / "They stay inside this PDF, on this device."). The inspector's Description becomes a textarea, Belongs to a select (The whole document · Page n · the annotation it came from), Relationship a select (Source · Data · Alternative · Supplement · Not specified) with its one-line meaning, and the second action becomes "Delete attachment" (danger ghost).

**States.** Empty: empty state "No attachments" / "Add a file to keep it inside this PDF." with Add file… (Organise) or nothing to do (Read; the tab is hidden then). Executable or script attachment (`.exe`, `.js`, `.bat`, `.app`…): a warning badge "Program" in the list and, on save, an alert dialog "This file is a program. Programs from documents can harm your computer. lamina never opens it; save it only if you trust the sender." with Cancel / Save anyway. Checksum mismatch: warning badge in the inspector. Too large for the device budget (web): "This file is larger than lamina can hold in memory here (limit 512 MB). Use the desktop app to save it." Deleting (Organise): toast "Attachment deleted · Undo". Saving is incremental, so the deleted file's bytes stay in the earlier revision: after save the status bar's middle slot shows the history note of [§6.1](design-language.md#61-desktop-and-web-shell-inverted-l) ("Earlier versions stay inside the file until you choose Save an optimised copy…"), and the panel offers "Save an optimised copy…" (`file.save-optimised`) to remove it from earlier versions.

**Behaviour.** Add, edit and delete are ops (`AttachFile`, `UpdateAttachment`, `DeleteAttachment`), one undo step each, only in Organise. Saving out writes the bytes exactly (SHA-256 equal to the original) and never changes the PDF. Attachments are never opened by lamina.

**Accessibility.** The drop zone (Organise) duplicates the Add file button; it is never the only way in. List rows are options with full names in their accessible name ("repairs-by-region.xlsx, 86 KB, spreadsheet, source of page 3"). Focus ring shown on "Edit in Organise".

**Platform differences.** Desktop: "Save to disk…" opens the save dialog; drag a row out of the panel to Finder/Explorer to save it (proposed). Web: "Download" uses the browser download; no drag-out. Phone: Attachments in the More sheet as a list sheet; tapping a file opens a detail sheet with Share (and Delete in Organise).

**Artboards.** `O14-attachments.dc.html` (Read, read-only).

**Todos.** [E3f](../../todos/ready/E3f-attachments-and-associated-files.md).

## O15 Object inspector

**Purpose.** Read-only inspection for power users and developers: the object tree from the trailer, any object's dictionary, streams decoded or raw, references, revisions, fonts, colour profiles and conformance claims.

**Entry points.** More › Developer › Inspect objects; Document properties (R08) › "Inspect…"; palette "Inspect objects" (`file.inspect-objects`), "Go to object…" (`file.go-to-object`, takes a number: "go to object 14"). It is off the main path on purpose: no toolbar button.

**Layout.** A full-canvas view in Read mode that replaces the pages until "Back to pages". It has no mode toolbar; a **view bar** takes the secondary toolbar's place: Back to pages · tabs Objects · Revisions · Fonts · Colour · Conformance · "Read only" badge · "PDF 1.7 · 412 objects · 2 revisions". Rail 300 px: "Go to object" input and the object tree. Main: the object detail on `surface`.

**Content and controls.**

- Tree (`role="tree"`, `mono`): keys and indices with their reference and a plain description ("12 0 R · Page 3"). Selected row: `accent-tint`.
- Detail: breadcrumb path; title "Object 12 · Page dictionary"; context line (page, revision where it last changed, object stream); Show page and Copy as text buttons; a key / value / type table (references are links); "Referenced by" list with plain descriptions; for streams, a "Content stream · object 14" section with filters and sizes, a Decoded · Raw · Hex toggle group, and a `mono` block with the first lines and "Show more".
- Revisions tab: one row per revision (xref section, objects changed, signed or not); Fonts tab: name, type, embedded/subset, encoding, ToUnicode coverage; Colour tab: output intents and ICC profiles; Conformance tab: claims (PDF/A, PDF/UA, PDF/X) as badges with "Claimed, not verified" wording.

**States.** Stream over the size cap or `Limits`: the block shows "This stream decodes to more than 16 MB. lamina shows the first 1 MB." with "Save raw stream…"; a stream that fails to decode shows the error text and switches to Raw. Broken reference: value in `danger` "14 0 R · missing". Large files: the tree is virtualised and loads children on expand.

**Behaviour.** Nothing here changes the file. "Show page" returns to the canvas with that page and outlines the object (page, annotation or XObject) for 2 s. Copy as text copies the object in PDF syntax.

**Accessibility.** Tree keyboard as the APG tree; the detail table has a caption and column headers; the stream block is a focusable `pre`. One focus ring is drawn on the selected tree item.

**Platform differences.** Identical on web and desktop (E3g). Not offered on phones (the command is hidden below 600 px).

**Artboards.** `O15-object-inspector.dc.html`.

**Todos.** [E3g](../../todos/ready/E3g-inspectors.md).

## O16 Export and extraction

**Purpose.** Make new files from the PDF: pages as images, text (plain, Markdown, JSON with positions), the original pictures, and links and bookmarks (JSON, CSV).

**Entry points.** More › Export…; Organise toolbar › Extract › "As images…" for selected pages; palette "Export…" (`file.export`), "Export pages as images" (`pages.export-images`), "Export text" (`file.export-text`), "Export pictures" (`file.export-pictures`), "Export links and bookmarks" (`file.export-links-and-bookmarks`), each taking a page range ("export text 3-6"). No shortcut: ⌥⌘E belongs to Export highlights.

**Layout.** Dialog (max 560) over the dimmed Read shell (page 3 behind, rail closed); on web, the same dialog turns into a progress dialog (440) while the zip is built (web shell with the full editor top bar including zoom, and the rail Pages · Outline · Attachments at 220 px).

**Content and controls.**

- "What to export": four radio cards in a 2 × 2 grid (selected card: `accent` border on `accent-tint`).
- Pages: All 24 · This page (3) · Pages [range].
- Options per kind: images → Format (PNG · JPEG · TIFF), Resolution (72 / 150 / 300 dpi with pixel sizes), "Include comments and filled-in fields"; text → Format (Plain text · Markdown · JSON with positions), "Keep reading order from tags when available"; pictures → "Keep originals where possible (JPEG, JPEG 2000 unchanged)", minimum size; links and bookmarks → JSON · CSV.
- Save to (desktop): folder path and "Choose folder…", and the resulting file names ("4 files: Q3-board-report-p03.png to Q3-board-report-p06.png").
- Actions: Cancel, a primary that names the result ("Export 4 pages", "Export text", "Export 12 pictures"). Close button "Close Export".

**States.** In progress (web artboard): title "Exporting 24 pages as PNG", determinate bar with "15 of 24 pages · 62%", the sentence "Pages are drawn on this device and packed into Q3-board-report-pages.zip, which your browser downloads when it’s ready. Nothing is uploaded.", Cancel export. Desktop progress shows in the same place, then a toast "4 pages exported · Show in folder". Nothing to export (no pictures, no links): the card is disabled with "None in this file". Error mid-way: the dialog lists the failed pages ("Page 17 couldn’t be drawn") and offers "Export the rest". Untagged file + Markdown: a notice "Approximate structure: this file has no tags, so headings and lists are guessed."

**Behaviour.** Read-only for the PDF. Desktop writes files into the chosen folder (native rendering, in parallel). Web streams into a zip within the memory budget (`Limits`). A single file (one page, or text) downloads directly instead of zipped. Plain text and JSON with positions come from E3i; Markdown output comes from the `laminapdf-extract` serialisers in [L3b](../../todos/done/L3b-extract-tables-and-serialisers.md), so the Markdown option ships when L3b does.

**Accessibility.** Radio cards are real radios inside labels; the progress bar is a `progressbar` with live text; Cancel keeps focus during progress. Focus ring drawn on "Choose folder…" (desktop) and "Cancel export" (web).

**Platform differences.** Desktop: folder output, Show in folder. Web: zip download, no folder field. Phone: Share sheet target for one file; several files are zipped and shared.

**Artboards.** `O16-export.dc.html` (desktop dialog), `O16-export-web.dc.html` (web, zipping in progress).

**Todos.** [E3h](../../todos/done/E3h-images-to-pdf-and-page-export.md), [E3i](../../todos/ready/E3i-extraction.md), [L3b](../../todos/done/L3b-extract-tables-and-serialisers.md) (Markdown).

## O17 Phone: organise pages

**Purpose.** Organise pages on a phone without drag: select in a grid, then act from the bottom action bar; reorder with "Move to page…".

**Entry points.** Pages sheet › "Select" or long-press a thumbnail; More › Organise pages.

**Layout.** Phone shell. Top bar: Done organising (close icon, 44 px), title "Organise pages" over "2 of 24 selected" (`aria-live`), "Select all" text button (`pages.select-all`). A scrolling 3-column page grid (100 × 129 thumbnails, 13 px numbers or labels). Bottom action bar (`role="toolbar"`, "Page actions"): Move to… (`pages.move-to`) · Rotate (`pages.rotate-right`) · Duplicate (`pages.duplicate`) · Delete (`pages.delete`, in `danger`) · More (Insert blank page, Insert from file…, Extract…, Crop…, Resize…, Page labels…). "Move to…" is the phone's deliberate short form of "Move to page…": the full label doesn't fit a fifth of 390 px at 11.5 px, and its accessible name stays "Move to page…".

**Content and controls.**

- Thumbnails show a 24 px selection circle top right: empty with a `control-border` ring, or `accent` with a white check; selected thumbnails also get the 2 px `accent` outline.
- Move sheet (dialog as a sheet at the large detent, 56 px from the top, over a scrim): title "Move 2 pages" with "pages 4 and 5"; Where to radios (To the start · After a page · To the end, 48 px rows, 16 px text); After page: a 44 px number field with − and + buttons and the outline title of that page; a Result strip showing 11 │ 4 5 │ 12 with `accent` insertion bars; "Bookmarks and links still point to the same pages. You can undo this in one step."; full-width Move pages (primary) and Cancel (48 px). Close button "Close Move pages".

**States.** Nothing selected: Move to…, Rotate, Duplicate and Delete are disabled; More keeps Insert blank page and Insert from file…; Select all stays in the top bar. One page selected: the sheet's title is "Move page 4". After Move/Rotate/Duplicate/Delete: toast above the action bar "2 pages moved after page 11 · Undo"; Delete asks first in an alert sheet ("Delete 2 pages?" · Delete 2 pages (danger) · Cancel). Signed files: as the area rules (on a conflict, the banner appears under the top bar and the action doesn't apply).

**Behaviour.** Tap toggles selection; long-press on a thumbnail outside selection mode enters it with that page selected. Each action is one op and one undo step for the batch (ADR 0006). No live reordering while selecting.

**Accessibility.** The grid is a `listbox` with `aria-multiselectable`; each thumbnail is a 44 px+ `option` button with `aria-selected` and a name "Page 4, selected". Move to… is the single-pointer alternative to drag (WCAG 2.5.7). Focus ring drawn inset on the Move to… button (`aria-label="Move to page…"`), and on Move pages in the sheet board. The sheet title uses the Title size (Newsreader 20). Text inputs are 16 px (no iOS zoom).

**Platform differences.** Tablet: the same grid with 5–6 columns and the bottom action bar; drag to reorder is added for pen and mouse, but Move to page… stays.

**Artboards.** `O17-phone-organise.dc.html` (2 selected), `O17-phone-organise-move.dc.html` (Move sheet).

**Todos.** [E2h](../../todos/ready/E2h-page-organiser-ui.md).

## New components

All of these are now in the design language; the boards use its markup.

- **Crop overlay** (O09): dimmed cut area, 2 px `accent` frame and 8 px handles, with numeric margins as the non-drag alternative ([§7.32](design-language.md#732-page-overlays-proposed)).
- **Destination marker** (O11): dashed `accent` line with a title tag, drawn in a widened paragraph gap.
- **Editable outline tree** (O11): rename input, stroked drag grip, outline colours ([§7.7](design-language.md#77-outline-row), [§2.7](design-language.md#27-fixed-content-palettes)).
- **Constraint banner and signature rows** (O09 certified): [§7.13](design-language.md#713-save-note-and-save-banner-signed-file).
- **Ranges table and label preview** (O10), **named destinations table** (O11) and **radio cards** (O16): tables and lists of [§7.31](design-language.md#731-lists-and-tables-proposed).

## Proposed design-language additions

Merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29; see its decisions table.

## Open questions

- **Bookmarks from headings.** The inventory lists "from headings" for O11, but E3b leaves headings-based generation to [E15b](../../todos/ready/E15b-structure-tree-editing.md). The Bookmark menu gains "Bookmarks from headings…" for tagged files once E15b lands.
- **Units.** Geometry fields show mm or inches by locale; points are used only in Page boxes. A setting to force a unit is not designed.
