# Paper: pages, navigation and extraction

Screens O09–O17 of the [screen inventory](screens.md#o-edit-session-organise-navigation-and-extraction-e1e3): page geometry, page labels, bookmarks, links, the initial view and article threads, attachments, the object inspector, export and extraction, and organising pages on a phone. They implement [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and [§E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) in the [Paper design language](design-language.md).

- **Canvas:** [lamina Paper — Pages, navigation and extraction](https://claude.ai/artifact/XxRzeJ46jd6mBFhp7HevYf) (private until shared). One row per screen, ordered by ID, with a sticky note per row.
- **Shells:** desktop screens are drawn in the macOS window frame from the `Desktop` foundation board; the web export state uses the web shell from `Main`; phone screens use the `Mobile-read`/`Mobile-select` shell.
- **Scope note:** Organise mode's page grid, insert, merge and split (O06–O08) belong to part A. This area adds geometry, labels and navigation tools to that mode and draws the grid only as a backdrop.

Rules that hold for every screen here:

- **Explicit save.** Every edit below is an op in the op log (ADR 0006): it adds one "unsaved change", one undo step, and nothing reaches the file until Save. Read-only views (O13 threads, O15, O16) never mark the file changed.
- **Signed files.** Geometry, labels, bookmarks, named destinations, initial view and attachments are class `PageStructure` or `Document`, so under a certification signature they are blocked before they apply, with the E1e constraint banner naming the signature (design language §7.13); with approval signatures only they save as an incremental update and the status-bar note says so. Links are class `Annotation` and are allowed under DocMDP P3. The control that would apply a blocked op stays enabled; pressing it shows the banner and does nothing else.
- **Rail tabs.** In this area the rail shows Pages and Outline, plus Attachments when the file has attachments (or in any editing mode) and Threads when the file has article threads. The foundation boards label the third tab Bookmarks; see [Open questions](#open-questions).

## O09 Page geometry

**Purpose.** Crop, resize, set page boxes and orientation for one or many pages, and say plainly that cropping hides content without removing it.

**Entry points.** Organise mode › secondary toolbar Crop (toggle) and Resize…; page context menu › Crop…, Resize…, Page boxes…; palette "Crop pages", "Resize pages", "Set page boxes", "Make landscape" / "Make portrait". No shortcuts.

**Layout.** Desktop shell, Organise mode. Secondary toolbar: Insert · Rotate left · Rotate right · Crop · Resize… · Extract · Delete │ Move to page… · context note. Rail: Pages. Canvas: the selected page at true size with the crop overlay. Inspector "Page geometry" with tabs Crop · Size · Page boxes. Resize opens a dialog over the dimmed page grid.

**Content and controls.**

- Crop overlay on the page: the area being cut is dimmed with the scrim colour; the kept area has a 2 px `accent` frame and eight 10 px white handles (`accent` border). Dragging a handle or an edge changes the matching margin.
- Inspector › Crop: "Remove from each edge" with Top, Bottom, Left, Right number inputs (mm; the unit follows the locale: mm or in), a live "Visible area 188 × 226 mm (page 216 × 279 mm)" line; "Apply to": This page / All 24 pages / Pages [range]. Warning notice (`warning-tint`): "**Cropped content stays in the file.** Cropping only hides the edges: the running header and page number can still be copied, searched and extracted. To remove them, use Redact." Actions: Crop page (primary, `organise.crop`), Reset (ghost).
- Inspector › Size: the same fields as the resize dialog for the selected pages; orientation as two radios.
- Inspector › Page boxes (advanced): Media, Crop, Bleed, Trim and Art boxes as four numbers each (left, bottom, right, top in pt), each with a "Same as crop box" checkbox for the last three; `organise.page-boxes`.
- Resize dialog "Resize pages" (`organise.resize`): subtitle "24 pages · now US Letter, 216 × 279 mm"; New size select (A4, US Letter, US Legal, A5, Custom: Custom shows width and height inputs); Orientation radios; Content radios "Scale to fit the new size" (default, `Scale`) and "Keep the content size" (`Extend`/`Anchor`); a preview with the current page dashed and the new size solid; a 3 × 3 Anchor radio grid, disabled until "Keep the content size" is chosen; the sentence "Bookmarks, links and the opening view are moved with the content, so they still land on the same place. You can undo this in one step."; Cancel, Resize 24 pages.

**States.**

- Default: overlay hidden until Crop is on; the inspector opens on the Crop tab.
- Margins larger than the page: the field turns `danger` with "Leaves no visible area"; Crop page disabled.
- Mixed sizes in the selection: the visible-area line reads "Sizes differ: margins are removed from each page"; the resize dialog's subtitle reads "24 pages · 3 sizes".
- A page that carries a signed signature field: Crop page and Resize show the constraint banner ("Saving would invalidate the signature on page 12") and do not apply.
- Done: toast "Page 3 cropped · Undo"; the top bar shows "1 unsaved change".

**Behaviour.** Crop writes `/CropBox` (`CropPages`); the op report's `CroppedContentRemains` is why the notice is always shown, not only after applying. Resize runs `ResizePages` with the chosen mode; annotations, links, bookmarks, named destinations and the open action are transformed with the page (E2f). Each apply is one undo step. Rotate left/right uses `/Rotate`; "Make landscape" asks nothing extra when the content can be rotated, and offers the resize dialog when the page must change shape.

**Accessibility.** The numeric fields are the non-drag alternative to the handles (WCAG 2.5.7); the handles are `aria-hidden`. Focus order: tabs → margins → Apply to → Crop page. Arrow keys on a focused margin field change it by 1 mm (Shift: 10 mm). The warning notice is `role="status"`. The anchor grid is a `radiogroup` of labelled buttons (Top left … Bottom right) and is `aria-disabled` while unused.

**Platform differences.** Web and desktop identical. Phone: crop and resize are in the page's More sheet in O17 as full-screen sheets with the same fields (no handles). Tablet: handles grow to 24 px hit areas.

**Artboards.** `O09-page-geometry.dc.html` (crop with warning), `O09-page-geometry-resize.dc.html` (resize dialog).

**Todos.** [E2f](../../todos/ready/E2f-page-geometry.md), [E2h](../../todos/ready/E2h-page-organiser-ui.md).

## O10 Page labels editor

**Purpose.** Define the page names readers show (roman front matter, arabic body, a prefixed appendix) without touching the printed page.

**Entry points.** Organise mode › secondary toolbar "Page labels…"; page context menu › Page labels…; More › Document › Page labels…; palette "Edit page labels" (`organise.page-labels`).

**Layout.** Dialog (max 560) over the dimmed Organise grid.

**Content and controls.**

- Intro: "The names pages show in lamina and other PDF readers. They don’t change what is printed on the page."
- Ranges table (`<table>` with column headers From page · Style · Prefix · Start at · remove): one row per range. The first row always starts at page 1 (its From field is disabled and it can't be removed). Style select: 1, 2, 3 · i, ii, iii · I, II, III · a, b, c · A, B, C · No number (prefix only).
- "Add range" (ghost with plus) adds a row starting after the last range's first page.
- Preview: a compact grid of "page → label" pairs for the first pages, the pages around each range start and the last pages; labels in a changed range are shown in `accent` 600. A caption names the elided run ("Pages 4–20 continue 2–18").
- Actions: Cancel, Apply labels (`SetPageLabels`).

**States.** Two rows starting on the same page: both From fields turn `danger`, "Two ranges can’t start on page 22", Apply disabled. A file with no labels opens with one row "1 · 1, 2, 3 · (no prefix) · 1". Duplicate labels (for example two "1"s) are allowed and noted in `muted` ("Pages 1 and 3 are both labelled 1"), since the spec allows them. After apply: thumbnails, the page input and the status bar show labels ("Page iii (5 of 24)").

**Behaviour.** One op and one undo step for the whole table. Later page moves keep labels through `PageLabels::remap` (E2b); the dialog always reflects the current tree. Signed: as the area rules.

**Accessibility.** Each input has an `aria-label` naming its row and column ("Range 3, prefix"). Remove buttons are labelled "Remove range 3". The preview list is an `<ol aria-label="Labels by page">`. Focus starts on the first editable field; after "Add range", focus moves to the new row's From field.

**Platform differences.** Identical on web and desktop. Phone: a full-height sheet with one card per range (fields stacked, 44 px), preview below.

**Artboards.** `O10-page-labels.dc.html`.

**Todos.** [E2g](../../todos/ready/E2g-page-labels.md).

## O11 Bookmarks editor

**Purpose.** Create, rename, nest, reorder and retarget the file's bookmarks (the PDF outline), and generate a contents page from them.

**Entry points.** Rail › Outline tab (editable in Organise and Edit modes; read-only in Read with an "Edit bookmarks" ghost button that switches to Organise); secondary toolbar "Add bookmark"; the selection mini toolbar's More › "Add bookmark here"; palette "Add bookmark" (`bookmark.add`), "Create a contents page" (`bookmark.generate-contents`). Rail header More menu: Expand all, Collapse all, Create a contents page…, Sort by page, Bookmarks from headings… (tagged files only; see open questions).

**Layout.** Desktop shell, Organise mode; rail widened to 272 px (resizable) with the Outline tab; canvas shows the destination; inspector "Bookmark". The contents-page generator is a dialog.

**Content and controls.**

- Rail header: "Bookmarks in this file", Add (icon, "Add bookmark"), More (icon, `aria-haspopup="menu"`).
- Tree (`role="tree"`, rows `treeitem` with `aria-level`, `aria-expanded`): Newsreader rows like the outline row (§7.7), page number right. Selected row: `accent-tint` with a drag grip. Inline rename: the row turns into a text input (Newsreader 14) with the focus ring; Enter commits, Escape cancels.
- Footer hint: "Drag to reorder or nest. With the keyboard: ⇧⌘↑ ⇧⌘↓ move, ⇧⌘← ⇧⌘→ change level." (Windows/Linux: Ctrl Shift + arrows, matching E2h's page moves.)
- Destination marker on the page: a dashed 2 px `accent` line at the target's top with a small `accent` tag carrying the bookmark title (page overlay, `aria-hidden`).
- Inspector: "Goes to" (page and position in words: "Page 3 · second paragraph"); "Use current view" (`bookmark.set-destination`) with a one-line hint; "Zoom when opened" select (Keep the reader’s zoom · Fit page · Fit width · 100%; maps to `/XYZ` with null zoom, `/Fit`, `/FitH`, `/XYZ … 1`); Style toggles Bold and Italic (`/F` flags); "Show nested bookmarks when opened" checkbox (`/Count` sign; disabled when the bookmark has no children); actions Go to (secondary) and Delete bookmark (danger ghost, `bookmark.delete`).
- Contents page dialog: Insert (Before page 1 · After page 1 · At the end), Levels (Top level only · Two levels), Page title (default "Contents"), "Add a “Contents” bookmark" checkbox, a preview of the generated page with dot leaders and the page labels it will show, the consequence sentence ("Adds 1 page, so every later page moves up by one. Bookmarks, links and page labels follow their pages."), Cancel, Create contents page (`GenerateToc`).

**States.** Empty: empty state "No bookmarks yet" / "Add one for the page you’re on, or create them from a contents page." with Add bookmark. A broken outline repaired on open: a notice at the top of the tree "Some bookmarks were damaged; lamina rebuilt the list" (warning tint). Dragging: a 2 px `accent` insertion line with an indent that shows the target level; dropping onto a row's middle third nests. A bookmark whose target page was deleted shows "Page removed" in `danger` text in place of its number and in the inspector, with "Use current view" to fix it.

**Behaviour.** Each add, rename, move, nest, retarget or delete is one undo step (`AddOutlineItem`, `UpdateOutlineItem`, `MoveOutlineItem`, `DeleteOutlineItem`). Clicking a row in Organise/Edit selects it and navigates; double-click or F2 renames. "Use current view" stores the current page, scroll position and (if not "Keep") zoom. The contents page is one composite op and one undo step.

**Accessibility.** Tree keyboard model from the APG tree pattern: ↑/↓ move, →/← expand/collapse, Enter goes to the destination, F2 renames, Delete removes (with toast Undo). Move and level shortcuts are the non-drag alternative. Position changes are announced politely ("Moved under 2. Repairs backlog, position 2 of 2"). The destination marker is decorative; its information is in the inspector.

**Platform differences.** Identical on web and desktop (desktop menu bar adds Bookmarks › …). Phone: the Outline sheet gets an Edit button; edit mode shows reorder via "Move…" sheets and a rename field, no drag.

**Artboards.** `O11-bookmarks.dc.html` (editor with inline rename and inspector), `O11-bookmarks-toc.dc.html` (create a contents page).

**Todos.** [E3b](../../todos/ready/E3b-bookmarks-and-destinations.md).

## O12 Link editor

**Purpose.** Make a link from selected text or a drawn area and point it at a page, a named destination, a web address or an email address.

**Entry points.** Edit mode › Link tool (draw a box); selection mini toolbar › More › "Make link" (`link.make-from-selection`); selecting an existing link in Edit mode; palette "Add link", "Make link from selection". No shortcuts.

**Layout.** Desktop shell, Edit mode. Secondary toolbar: Text · Image · Link │ Redact · Protect · context note "Draw a box, or select text and press Make link · 2 links on this page". Rail closed in the drawing (any tab can be open). Inspector "Link".

**Content and controls.**

- On the page, while the Link tool is on: every existing link shows a 1 px dashed `muted` outline; the selected link has the 2 px `accent` outline (text links follow the glyph quads; drawn links show four corner handles).
- Inspector: the linked text as a quote (text links only) or "drawn area over the chart" in the context line. "Goes to" radios: A page in this file · A named destination · A web address · An email address.
  - Page: number input with the page's outline title beside it, and "Pick a place on the page…" (click a spot on any page to set an `/XYZ` position).
  - Named destination: a searchable select of the file's names plus "New name…".
  - Web address: text input; only `https:`, `http:` (and `mailto:` under Email) are accepted.
  - Email: address and optional subject.
- "Zoom when followed" select (page and named destinations).
- Description textarea (`/Contents`), help "Read aloud by screen readers."
- Actions: Follow link (secondary, `link.follow`), Delete link (danger ghost, `link.delete`).

**States.** Rejected address (artboard 2): `javascript:` and other schemes show the field in `danger` with "This address would run a script, so it can’t be used. Use an address that starts with https:// or http://."; the link keeps its last valid target and Follow link is disabled. Empty target on a new link: Follow link disabled, and leaving the inspector with no target removes the new link (toast "Link removed: it had no target · Undo"). A note under web targets: "Readers of this file are asked before a web link opens." (the C6 external-link confirmation).

**Behaviour.** Drawing or making a link creates it (`AddLink`) and selects it; every inspector change is `UpdateLink`, committed on blur or Enter, one undo step each. Delete removes it. Links on signed files follow the area rules (allowed under P3).

**Accessibility.** "Make link" works entirely from a keyboard text selection; drawn areas can be created and resized with arrow keys after "Add link" places a default box at the caret or page centre (Shift+arrows resize). The error text is linked with `aria-describedby` and `aria-invalid`. The Description field is called out because it is what assistive tech reads for the link.

**Platform differences.** Identical. Phone: links are edited from the selection action bar's More › Make link and a sheet with the same fields; no drawing.

**Artboards.** `O12-link-editor.dc.html` (text link to a page), `O12-link-editor-web.dc.html` (drawn area, rejected web address).

**Todos.** [E3c](../../todos/ready/E3c-links.md).

## O13 Initial view and article threads

**Purpose.** Choose how the file opens in any reader, and list and follow the file's article threads.

**Entry points.** Initial view: More › Document › Initial view… and Document properties (R08) › "Initial view…"; palette "Set initial view" (`document.initial-view`). Threads: rail › Threads tab (shown only when the file has threads); palette "Next thread part" / "Previous thread part" (`thread.next-part`, `thread.previous-part`).

**Layout.** Initial view: dialog over the dimmed Read shell. Threads: desktop shell in Read mode, rail 248 px with the Threads tab, page overlay on the current part.

**Content and controls.**

- Initial view dialog: intro "How this file opens for anyone, in lamina and in other PDF readers."; Open at page (number + outline title); Zoom (Fit width · The reader’s default · Fit page · 100%…); Page layout radios (One page at a time · Continuous scroll · Two pages side by side · Side by side, scrolling → `/PageLayout`); "Show when opened" select (Bookmarks panel · Just the page · Page thumbnails · Attachments panel · Full screen → `/PageMode`); "Reader window" checkboxes (show the document title, fit the window to the first page, hide the reader's toolbar and menus → `/ViewerPreferences`) with "These are requests: some readers, and lamina’s own settings, may override them."; "Use the current view" (ghost, fills page, zoom and layout from the current view), Cancel, Apply (`SetInitialView`).
- Threads panel: one expandable entry per thread (title from the thread's info, or "Untitled thread", and "3 parts · pages 3, 5, 6"); its parts as links "Part 1 · page 3"; Previous part / Next part buttons; the note "Threads join the parts of an article across pages. lamina follows them; editing threads comes later."
- On the page: the current part (bead) has a 2 px `accent` outline and a tag "Part 1 of 3 · continues on page 5"; the status bar reads "Page 3 · Repairs backlog in depth, part 1 of 3".

**States.** Initial view with nothing set: every control shows the reader default and Apply is disabled until something changes. Threads: tab hidden when there are none; a bead pointing to a missing page is listed as "Part 4 · page missing" in `muted` and skipped by Next part.

**Behaviour.** Apply is one undo step; lamina honours the saved settings the next time the file opens, unless the user turned off "Use the file’s initial view" in View settings (R10). Following a thread scrolls to the bead and keeps the column at the top; it adds history entries so Back returns (C6b).

**Accessibility.** The bead outline is decorative; the panel's `aria-current="location"` and the status bar (`aria-live="polite"`) carry the position. Next/Previous part are real buttons with disabled states.

**Platform differences.** Identical on web and desktop; "Resize the window to fit the first page" has no effect in lamina's web app (stated in the help text on web). Phone: threads in the Outline sheet as a second segment; initial view in More › Document.

**Artboards.** `O13-initial-view.dc.html`, `O13-article-threads.dc.html`.

**Todos.** [E3d](../../todos/ready/E3d-initial-view-and-threads.md).

## O14 Attachments panel

**Purpose.** See, add, describe, save out and delete files embedded in the PDF, including associated files tied to a page (PDF/A-3, Factur-X).

**Entry points.** Rail › Attachments tab; More › Attachments; status-bar item "3 attachments" when the file has any (proposed); drag files onto the panel; palette "Attach file…" (`attachment.add`), "Save attachment…" (`attachment.save`).

**Layout.** Desktop shell (any mode); rail 272 px with the Attachments tab; inspector for the selected attachment.

**Content and controls.**

- Rail header: count and total size ("3 files · 1.3 MB"), "Add file…" (ghost with plus; opens the OS picker on desktop, the file input on web).
- List (`role="listbox"`): file icon, name (ellipsis), "86 KB · Spreadsheet", and for associated files a neutral badge naming the relationship and target ("Source of page 3", "Data for page 11"). Selected: `accent-tint` with an `accent` inset ring. Portfolios (`/Collection`) show their schema columns as extra metadata lines, read-only.
- Drop zone at the bottom: dashed `control-border`, paperclip icon, "Drop files here to attach" / "They stay inside this PDF, on this device."
- Inspector: file name as title, "Attachment · associated with page 3"; Description textarea; "Belongs to" select (The whole document · Page n · an annotation when it came from one); Relationship select (Source · Data · Alternative · Supplement · Not specified) with a one-line meaning; facts (Type, Size with exact bytes, Modified, Checksum badge "Matches" or warning "Doesn’t match"); actions "Save to disk…" (desktop) / "Download" (web) and Delete attachment.

**States.** Empty: empty state "No attachments" / "Add a file to keep it inside this PDF." with Add file…. Executable or script attachment (`.exe`, `.js`, `.bat`, `.app`…): a warning badge "Program" in the list and, on save, an alert dialog "This file is a program. Programs from documents can harm your computer. lamina never opens it; save it only if you trust the sender." with Cancel / Save anyway. Checksum mismatch: warning badge in the inspector. Too large for the device budget (web): "This file is larger than lamina can hold in memory here (limit 512 MB). Use the desktop app to save it." Deleting: toast "Attachment deleted · Undo", and after save a notice in the panel: "Earlier versions of this file still contain it. Save a full copy to remove it for good." with "Save a full copy…".

**Behaviour.** Add, edit and delete are ops (`AttachFile`, `UpdateAttachment`, `DeleteAttachment`), one undo step each. Saving out writes the bytes exactly (SHA-256 equal to the original). Attachments are never opened by lamina.

**Accessibility.** The drop zone duplicates the Add file button; it is not the only way in. List rows are options with full names in their accessible name ("repairs-by-region.xlsx, 86 KB, spreadsheet, source of page 3"). Focus ring shown on "Add file…".

**Platform differences.** Desktop: "Save to disk…" opens the save dialog; drag a row out of the panel to Finder/Explorer to save it (proposed). Web: "Download" uses the browser download; no drag-out. Phone: Attachments in the More sheet as a list sheet; tapping a file opens a detail sheet with Share and Delete.

**Artboards.** `O14-attachments.dc.html`.

**Todos.** [E3f](../../todos/ready/E3f-attachments-and-associated-files.md).

## O15 Object inspector

**Purpose.** Read-only inspection for power users and developers: the object tree from the trailer, any object's dictionary, streams decoded or raw, references, revisions, fonts, colour profiles and conformance claims.

**Entry points.** More › Developer › Inspect objects; Document properties (R08) › "Inspect…"; palette "Inspect objects" (`inspect.objects`), "Go to object…" (`inspect.go-to-object`, takes a number: "go to object 14"). It is off the main path on purpose: no toolbar button.

**Layout.** A view that replaces the canvas until "Back to pages". The secondary toolbar becomes the view's bar: Back to pages · tabs Objects · Revisions · Fonts · Colour · Conformance · "Read only" badge · "PDF 1.7 · 412 objects · 2 revisions". Rail 300 px: "Go to object" input and the object tree. Main: the object detail on `surface`.

**Content and controls.**

- Tree (`role="tree"`, monospace 12.5): keys and indices with their reference and a plain description ("12 0 R · Page 3"). Selected row: `accent-tint`.
- Detail: breadcrumb path; title "Object 12 · Page dictionary"; context line (page, revision where it last changed, object stream); Show page and Copy as text buttons; a key / value / type table (references are links); "Referenced by" list with plain descriptions; for streams, a "Content stream · object 14" section with filters and sizes, a Decoded · Raw · Hex toggle group, and a monospace block that shows the first lines with "Show more".
- Revisions tab: one row per revision (xref section, objects changed, signed or not); Fonts tab: name, type, embedded/subset, encoding, ToUnicode coverage; Colour tab: output intents and ICC profiles; Conformance tab: claims (PDF/A, PDF/UA, PDF/X) as badges with "Claimed, not verified" wording.

**States.** Stream over the size cap or `Limits`: the block shows "This stream decodes to more than 16 MB. lamina shows the first 1 MB." with "Save raw stream…"; a stream that fails to decode shows the error text and switches to Raw. Broken reference: value in `danger` "14 0 R · missing". Large files: the tree is virtualised and loads children on expand.

**Behaviour.** Nothing here changes the file. "Show page" returns to the canvas with that page and outlines the object (page, annotation or XObject) for 2 s. Copy as text copies the object in PDF syntax.

**Accessibility.** Tree keyboard as the APG tree; the detail table has a caption and column headers; the stream block is a focusable `pre` with `aria-label`. One focus ring is drawn on the selected tree item.

**Platform differences.** Identical on web and desktop (E3g). Not offered on phones (the command is hidden below 600 px).

**Artboards.** `O15-object-inspector.dc.html`.

**Todos.** [E3g](../../todos/ready/E3g-inspectors.md).

## O16 Export and extraction

**Purpose.** Make new files from the PDF: pages as images, text (plain, Markdown, JSON with positions), the original pictures, and links and bookmarks (JSON, CSV).

**Entry points.** More › Export…; Organise mode › Extract › "As images…" for selected pages; palette "Export…" (`export.open`), "Export pages as images" (`export.pages-as-images`), "Export text" (`export.text`), "Export pictures" (`export.images`), "Export links and bookmarks" (`export.links-and-bookmarks`), each taking a page range ("export text 3-6"). No shortcut: ⌥⌘E belongs to Export highlights.

**Layout.** Dialog (max 560) over the dimmed shell; on web, the same dialog turns into a progress dialog while the zip is built.

**Content and controls.**

- "What to export": four radio cards in a 2 × 2 grid (selected card: `accent` border on `accent-tint`).
- Pages: All 24 · This page (3) · Pages [range].
- Options per kind: images → Format (PNG · JPEG · TIFF), Resolution (72 / 150 / 300 dpi with pixel sizes), "Include comments and filled-in fields"; text → Format (Plain text · Markdown · JSON with positions), "Keep reading order from tags when available"; pictures → "Keep originals where possible (JPEG, JPEG 2000 unchanged)", minimum size; links and bookmarks → JSON · CSV.
- Save to (desktop): folder path and "Choose folder…", and the resulting file names ("4 files: Q3-board-report-p03.png to Q3-board-report-p06.png").
- Actions: Cancel, primary that names the result ("Export 4 pages", "Export text", "Export 12 pictures").

**States.** In progress (web artboard): title "Exporting 24 pages as PNG", determinate bar with "15 of 24 pages · 62%", the sentence "Pages are drawn on this device and packed into Q3-board-report-pages.zip, which your browser downloads when it’s ready. Nothing is uploaded.", Cancel export. Desktop progress shows in the same place, then a toast "4 pages exported · Show in folder". Nothing to export (no pictures, no links): the card is disabled with "None in this file". Error mid-way: the dialog lists the failed pages ("Page 17 couldn’t be drawn") and offers "Export the rest". Untagged file + Markdown: a notice "Approximate structure: this file has no tags, so headings and lists are guessed."

**Behaviour.** Read-only for the PDF. Desktop writes files into the chosen folder (native rendering, in parallel). Web streams into a zip within the memory budget (`Limits`). A single file (one page, or text) downloads directly instead of zipped.

**Accessibility.** Radio cards are real radios inside labels; the progress bar is a `progressbar` with live text; Cancel keeps focus during progress. Focus ring drawn on "Choose folder…" (desktop) and "Cancel export" (web).

**Platform differences.** Desktop: folder output, Show in folder. Web: zip download, no folder field. Phone: Share sheet target for one file; several files are zipped and shared.

**Artboards.** `O16-export.dc.html` (desktop dialog), `O16-export-web.dc.html` (web, zipping in progress).

**Todos.** [E3h](../../todos/ready/E3h-images-to-pdf-and-page-export.md), [E3i](../../todos/ready/E3i-extraction.md).

## O17 Phone: organise pages

**Purpose.** Organise pages on a phone without drag: select in a grid, then act from the bottom action bar; reorder with "Move to…".

**Entry points.** Pages sheet › "Select" or long-press a thumbnail; More › Organise pages.

**Layout.** Phone shell. Top bar: Done organising (close icon, 44 px), title "Organise pages" over "2 of 24 selected" (`aria-live`), "Select all" text button. A 3-column page grid (100 × 129 thumbnails, 13 px numbers or labels). Bottom action bar (`role="toolbar"`, "Page actions"): Move to… · Rotate · Duplicate · Delete (in `danger`) · More (Insert blank page, Insert from file…, Extract…, Crop…, Resize…, Page labels…).

**Content and controls.**

- Thumbnails show a 24 px selection circle top right: empty with a `control-border` ring, or `accent` with a white check; selected thumbnails also get the 2 px `accent` outline.
- Move sheet (dialog as a sheet at the large detent over a scrim): title "Move 2 pages" with "pages 4 and 5"; Where to radios (To the start · After a page · To the end, 48 px rows, 16 px text); After page: a 44 px number field with − and + buttons and the outline title of that page; a Result strip showing 11 │ 4 5 │ 12 with `accent` insertion bars; "Bookmarks and links still point to the same pages. You can undo this in one step."; full-width Move pages (primary) and Cancel (48 px).

**States.** Nothing selected: the action bar shows only Select all and More (Insert), other buttons disabled. One page selected: Move to… reads "Move to…" and the sheet's title "Move page 4". After Move/Rotate/Duplicate/Delete: toast above the action bar "2 pages moved after page 11 · Undo"; Delete asks first in an alert sheet ("Delete 2 pages?" · Delete 2 pages (danger) · Cancel). Signed file: the action bar buttons stay, and pressing one shows the constraint banner under the top bar.

**Behaviour.** Tap toggles selection; long-press on a thumbnail outside selection mode enters it with that page selected. Each action is one op and one undo step for the batch (ADR 0006). No live reordering while selecting.

**Accessibility.** Grid is a multi-select `role="listbox"` (`aria-multiselectable`; a flowing grid has no fixed rows, as in O06); each thumbnail is a 44 px+ `option` with `aria-selected` and a name "Page 4, selected". Move to… is the single-pointer alternative to drag (WCAG 2.5.7). Focus ring drawn inset on the Move to… button. Text inputs are 16 px (no iOS zoom).

**Platform differences.** Tablet: the same grid with 5–6 columns and the bottom action bar; drag to reorder is added for pen and mouse, but Move to… stays.

**Artboards.** `O17-phone-organise.dc.html` (2 selected), `O17-phone-organise-move.dc.html` (Move sheet).

**Todos.** [E2h](../../todos/ready/E2h-page-organiser-ui.md).

## New components

Designed for this area in the Paper spirit; add them to the design language when the boards are reviewed.

| Component | Spec |
| --- | --- |
| Crop overlay | Kept area: 2 px `accent` frame; cut area: `rgba(31,29,26,0.38)` over the page; eight 10 × 10 white handles with a 1 px `accent` border, 6 px outside the frame. Page overlay (z 1), `aria-hidden`; numeric fields are the accessible control |
| Destination / bead marker | 2 px `accent` line (dashed for a bookmark destination) or outline (thread bead), with an 18 px `accent` tag, white 11/600 Source Sans 3, radius 3. Decorative; the panel or inspector carries the information |
| Editable tree row | Outline row (§7.7) plus: drag grip (six 1.4 px dots, `accent` when selected), inline rename input (Newsreader 14, 26 high, `control-border`, focus ring), `role="tree"`/`treeitem` with `aria-level` and `aria-expanded`, chevrons 16 px |
| Anchor grid | 3 × 3 `radiogroup` of 28 × 28 buttons, gap 4, radius 4, `control-border`; checked = `accent-tint` fill with an 8 px `accent` dot; disabled = `hairline` border and `disabled` dot |
| Radio card | A label containing a radio, title (14) and one line (12 `muted`); padding `10px 12px`, radius 4, `control-border`, white; selected = `accent` border, `accent-tint` fill, title `accent` 600 |
| Ranges table | `<table>` with 12/600 `text-3` headers, rows separated by `hairline`, 32 px controls, a trailing 28 px icon button to remove; each control `aria-label`led with row and column |
| Drop zone | 1 px dashed `control-border`, radius 4, padding `18px 12px`, 22 px `muted` icon, 13 px line + 12 px `muted` line; always paired with a button that does the same |
| Object tree and code block | Monospace system stack `ui-monospace, 'SF Mono', Menlo, Consolas, monospace` at 12.5/1.6 (new token `mono`, no web font); code block white, `hairline` border, radius 4, padding `12px 14px`, horizontal scroll |
| Phone selection circle | 24 px circle on thumbnails in selection mode: off = white with 1.5 px `control-border`; on = `accent` with a white 2.5-stroke check |
| Number stepper (touch) | 44 × 44 − and + buttons (radius 6, `control-border`) around a 72 × 44 input (16 px, tabular) |

New icons (24 grid, stroke 1.5, same rules as §5):

| Name | Markup |
| --- | --- |
| Insert page | `<rect x="5.5" y="3.5" width="13" height="17" rx="1"></rect><path d="M12 9v6M9 12h6"></path>` |
| Rotate left | `<path d="M8 4.5L4.5 8 8 11.5"></path><path d="M4.5 8H14a5.5 5.5 0 0 1 0 11H9"></path>` |
| Rotate right | `<path d="M16 4.5L19.5 8 16 11.5"></path><path d="M19.5 8H10a5.5 5.5 0 0 0 0 11h5"></path>` |
| Crop | `<path d="M7 3.5V17h13.5"></path><path d="M3.5 7H17v13.5"></path>` |
| Resize | `<path d="M4.5 9.5v-5h5M19.5 14.5v5h-5"></path><path d="M4.5 4.5l6 6M19.5 19.5l-6-6"></path>` |
| Extract | `<path d="M13.5 3.5h-7a1 1 0 0 0-1 1v15a1 1 0 0 0 1 1h11a1 1 0 0 0 1-1v-11z"></path><path d="M12 10.5v6M9.5 14L12 16.5 14.5 14"></path>` |
| Delete | `<path d="M4.5 6.5h15M9.5 6.5v-2h5v2M6.5 6.5l1 13h9l1-13"></path>` |
| Bookmark | `<path d="M7 4.5h10v15l-5-4-5 4z"></path>` |
| Plus | `<path d="M12 5v14M5 12h14"></path>` |
| Chevron right | `<path d="M10 7l5 5-5 5"></path>` |
| Link | `<path d="M10 14l4-4"></path><path d="M8.5 11.5l-2 2a3 3 0 0 0 4 4l2-2"></path><path d="M15.5 12.5l2-2a3 3 0 0 0-4-4l-2 2"></path>` |
| Text | `<path d="M5 6.5v-2h14v2M12 4.5v15M9 19.5h6"></path>` |
| Image | `<rect x="3.5" y="5.5" width="17" height="13" rx="1"></rect><circle cx="9" cy="10" r="1.5"></circle><path d="M20.5 15.5l-5-5-8 8"></path>` |
| Redact | `<rect x="4" y="9" width="16" height="6"></rect><path d="M4 5.5h9M4 18.5h12"></path>` |
| Protect | `<rect x="5.5" y="10.5" width="13" height="9.5" rx="1"></rect><path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5"></path>` |
| Attachment (paperclip) | `<path d="M16.5 7.5l-7 7a2 2 0 0 0 3 3l7.5-7.5a3.5 3.5 0 0 0-5-5L7.5 12.5a5 5 0 0 0 7 7l5-5"></path>` |
| File | `<path d="M13.5 3.5h-7a1 1 0 0 0-1 1v15a1 1 0 0 0 1 1h11a1 1 0 0 0 1-1v-11z"></path><path d="M13.5 3.5v5h5"></path>` |
| Folder | `<path d="M3.5 6.5h6l2 2h9v10h-17z"></path>` |
| Move to | `<path d="M4.5 12h11M12 8l4 4-4 4"></path><path d="M19.5 5v14"></path>` |
| Drag grip (filled dots, the one exception to "no fills") | six `<circle r="1.4">` at x 9/15, y 6/12/18, `fill="currentColor"` |

## Open questions

- **Bookmarks vs Outline tab.** In PDF terms the bookmarks are the outline, but the foundation boards and R06 show both an Outline and a Bookmarks tab. This area edits the file's outline in the Outline tab and uses "bookmark" in commands. R06 should decide whether the Bookmarks tab means personal reading bookmarks (not stored in the file) or is dropped, and whether Attachments and Threads join the tab row or move into an overflow.
- **Bookmarks from headings.** The inventory lists "from headings" for O11, but E3b leaves headings-based generation to [E15b](../../todos/ready/E15b-structure-tree-editing.md). The menu item is shown only for tagged files once E15b lands.
- **Units.** Geometry fields show mm or inches by locale; points are used only in Page boxes. A setting to force a unit is not designed.
