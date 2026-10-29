# Paper: convert, compare and produce

- **Status:** Draft (designed 2026-10-29 in the Paper direction)
- **Canvas:** [lamina Paper — Convert, compare and produce](https://claude.ai/artifact/GwyjtEGo2D2wMo4VUubKKZ) (artboards `X01`–`X12`)
- **Covers:** screens X01–X12 of the [screen inventory](screens.md#x-convert-compare-batch-accessibility-print-production-advanced-e13e17): OCR, Office/HTML conversion with server consent, revisions, compare, batch, accessibility checking and tagging, PDF/UA and PDF/A output, optimisation, print production, layers and actions, measurement and media.
- **Follows:** the [Paper design language](design-language.md) (tokens, components, patterns) and the [UI/UX plan](../../plans/ui-ux.md) principles §4 and layout §5.
- **Plan:** [full-editor §E13](../../plans/full-editor.md#e13-ocr-and-conversion), [§E14](../../plans/full-editor.md#e14-compare-revisions-and-batch), [§E15](../../plans/full-editor.md#e15-accessibility), [§E16](../../plans/full-editor.md#e16-optimisation-and-print-production), [§E17](../../plans/full-editor.md#e17-advanced-interactions).

These are power features, so the rule from principle 1 matters most here: none of them adds permanent chrome. Each one is reached from a menu, the palette or a contextual notice, and opens either a **dialog** (one-shot jobs: OCR options, convert, save to a standard, optimise, impose), a **tool toolbar** in the secondary-toolbar slot (ongoing work on the open document: Accessibility, Print production, Measure), a **rail panel** (Revisions, Layers), or its own **window** (Compare, Batch).

Three rules hold across the area:

1. **Nothing leaves the device without an explicit, per-job yes.** Every server job (OCR on the server, Office ↔ PDF, PDF → Word/Excel, web page → PDF, server batch) shows the [server consent block](design-language.md#718-dialog-proposed) first: what is sent, how big, to which host, for how long, for what. Its heading, rows and checkbox are worded for the job (see [Consent wording per job](#consent-wording-per-job)). The upload button stays disabled until the person ticks the box naming the file, pages or files. The retention promise is fixed before upload and never weakened afterwards. The network indicator then names the host and counts the approved uploads (principle 7).
2. **Copies, not surprises.** Jobs that rewrite the file (optimise, PDF/A, PDF/UA, imposition, conversions) save a **new file** and say so; jobs that add to the file (OCR text layer, tags, alt text, layers, actions, measurements) become ordinary unsaved changes with undo, saved incrementally, and happen in an editing mode, never in Read ([§8.11](design-language.md#811-read-mode-is-read-only-proposed)). Signed files follow [§7.13](design-language.md#713-save-note-and-save-banner-signed-file): a file with approval signatures shows the status-bar note (“Signed file. Saving adds your changes after the signature; the signature stays valid.”) while it has unsaved changes; only a certified file that forbids the change shows the banner. Where the result would lose or break a signature, the dialog says it before the person commits (§8.4 explicit save).
3. **Machine checks are not the whole truth.** Accessibility and conformance results always separate what lamina checked from what a person must still check.

Platform notes: all X screens are drawn as the **macOS desktop window** except X02 and the server-batch consent of X05, drawn as the **web app** because the inventory lists web first and those jobs are server-only there. The web shell is the same minus the window frame; differences are listed per screen. Windows/Linux differ only in window controls, menu placement and shortcut chips (`Ctrl` words instead of glyphs).

Shell: every document board uses the **Editor** top bar of [§7.1](design-language.md#71-top-bar) (sidebar toggle, file name and save state, Undo · Redo, mode switcher, page n/N, zoom −/%/+, Find, View, Commands, Save when there are changes, More). The Compare window keeps zoom, Find and View but has no page input (two documents); the Batch window has no document view, so it has neither page input nor zoom. Rails follow [§8.10](design-language.md#810-rail-tab-set-proposed) (Pages · Outline, plus Attachments in Edit; no Bookmarks tab); docked inspectors and job panels are 288 px ([§7.11](design-language.md#711-inspector-panel)); badges use [§7.23](design-language.md#723-badge-and-tag-proposed) (20 px, 12/600).

Sample files: `Q3-board-report.pdf` (24 pages; this area adds a scanned *Appendix B. Contractor invoices* on pages 23–24 and four saved revisions), plus three fictional files: `annual-review-2026.pdf` (16-page print piece with a video and a 3D model), `mill-lane-A102.pdf` (2-page floor plan with layers) and the batch folder `~/Board/2026 Q3`.

## X01 Make searchable (OCR)

**Purpose.** Add an invisible text layer to scanned pages so they can be found, selected, copied and read aloud, without touching the page images.

**Entry points.** Status-bar notice "2 scanned pages have no text · Make searchable…" (shown when pages have images and no text, also in Read); More menu › *Make searchable…*; palette `ocr.make-searchable`; native menu Tools › Make searchable…. No shortcut (low frequency). Because the job adds to the file, opening it from Read switches to Edit first (§8.11).

**Layout.** Options: dialog (560 wide) over the dimmed shell in Edit mode (Edit toolbar of §8.1 with no tool pressed and the context note “Make searchable · reading the text of scanned pages”), rail Pages · Outline · Attachments on Pages with the scanned thumbnails labelled "No text". Server chosen: the same dialog's second step holds the consent block. Progress and review: the dialog closes and a **job panel** (288) docks in the inspector slot ("Make searchable · On this computer · English"); the canvas shows the page with recognised words outlined; the rail closes to give the page room.

**Content and controls.**

- *Pages* radio group: "Pages without text · 23–24 (2 pages)" (default), "All pages · 24 (pages with text keep it)", "Pages" + range input.
- *Languages on these pages*: select + "Add a language" ghost; help "each extra one makes recognition slower".
- *Where to read the text*: two radio cards. "On this computer" (default on desktop; names the installed engine and its install date) and "On lamina’s server" (says a copy is uploaded and that the person will confirm what is sent first). Choosing the server card swaps the primary button to "Continue…", which shows the [server consent block](design-language.md#718-dialog-proposed) as the dialog's second step (drawn: `X01-ocr-server-consent`): a summary line of the choices with Change; the block with OCR wording (only the chosen pages are sent, as a copy: “Pages 23–24 of Q3-board-report.pdf, as a 2-page copy · 1.1 MB”; deleted as soon as the text comes back, 24 hours at most); the local alternative “Read the text on this computer instead”; Back · **Upload and read text** (disabled with the visible reason “Tick the box to upload” until the box is ticked).
- "Tag the recognised text for screen readers" checkbox (on when the file is tagged).
- Info note: one unsaved change; saving appends the layer. On the signed sample file it adds the §7.13 note: “Signed file. Saving adds your changes after the signature; the signature stays valid.”
- Buttons: Cancel · **Make searchable** (`ocr.run`).
- Job panel: determinate progress ("Reading page 24 · 1 of 2 pages done"), per-page list (words found, how many to check), **Words to check**: each low-confidence word with its line, what was read ("Re-p1aster"), an editable input pre-filled with the best guess and "Accept"; switch "Outline recognised words"; "Add text to 2 pages" (primary, disabled until all pages are read) and "Stop" (danger ghost).

**States.**

- *Engine not installed* (desktop): the "On this computer" card reads "OCR engine not installed · Download (about [SIZE])"; the download is verified and signed (E13d) and is itself a network request the indicator shows.
- *Web, file too large for in-browser OCR*: the local card is disabled with the reason ("Files over [LIMIT] pages need the desktop app or the server").
- *In progress*: drawn (`X01-ocr-progress`). Pages already read can be reviewed while others run.
- *Done*: panel shows "2 pages read · 3 words checked"; pressing "Add text to 2 pages" applies one `AddOcrLayer` op per page as a single undo step, closes the panel and shows the toast "Text added to 2 pages · Undo". Top bar: "1 unsaved change".
- *Stopped/cancelled*: nothing is added; toast "Stopped. Nothing was added."
- *Error* (engine crash, unreadable page): the page row shows a danger badge "Couldn’t read" with "Try again"; other pages still apply.
- *Signed file (approval signatures, as the sample)*: the info note carries the §7.13 note, and after applying the status bar shows it while the change is unsaved.
- *Certified file that allows no changes*: a warning note in the dialog: "Adding text changes page content. This file is certified to allow no changes, so the signature would no longer be valid. Save a copy instead." with the save banner (§7.13) after applying.

**Behaviour.** Recognition runs in a worker (web) or a sidecar (desktop); the page stays readable and scrollable. The text layer uses render mode 3 so print and appearance don't change. Undo removes the layer. Save is incremental.

**Accessibility.** Dialog focus starts on the first radio; Tab order: pages, languages, where, tag checkbox, buttons. The progress bar has `aria-valuenow`; page completion is announced politely. Words to check are real inputs with labels naming the line and the misreading. Word outlines are decorative (`aria-hidden`); the list is the accessible equivalent.

**Platform differences.** Web: in-browser OCR only for small documents (E13d), otherwise server only; the status-bar text and consent are identical. Server OCR, like every server job, follows X02's sign-in and plan rules. Phone/tablet: not designed in this round; the dialog becomes a large sheet and the job panel a medium sheet.

**Artboards.** `X01-ocr.dc.html` (options), `X01-ocr-progress.dc.html` (in progress with review), `X01-ocr-server-consent.dc.html` (server chosen: consent step, nothing uploaded yet).

**Todos.** [E13c](../../todos/ready/E13c-ocr-text-layer.md), [E13d](../../todos/ready/E13d-local-ocr.md).

## X02 Convert to and from PDF

**Purpose.** Convert this PDF to Word or Excel, or Word/Excel/PowerPoint/web pages to PDF, as a job whose result opens as a new document; on the web this always needs the server, so the job sheet is built around consent.

**Entry points.** More menu › *Convert…*; palette `convert.from-pdf`, `convert.to-pdf`; home screen drop zone when an Office file is dropped (opens the dialog on the "To PDF" tab); desktop native menu File › Convert.

**Layout.** Dialog (560) over the dimmed web editor (Read mode; converting makes a new document and leaves this one unchanged). Jobs then live in a **Server jobs** panel (288, inspector slot; `jobs.open`) and a status-bar item "2 server jobs · 1 ready" that reopens it.

**Content and controls.**

- Tabs: "From this PDF" · "To PDF (Word, Excel, PowerPoint, web page)", each tab `aria-controls` its `tabpanel`. The To-PDF tab has a *Convert* radio (A file · A web page), a file picker, or a *Web address* field (HTML → PDF via the browser service on lamina’s job server), with the help “The PDF opens as a new document; nothing is added to Q3-board-report.pdf.”
- *Web page → PDF* (drawn: `X02-convert-url`): lamina’s job server fetches a third-party page, so the consent block says so. **What is sent:** the address only, no cookies, passwords or sign-ins from the browser. **Where to:** lamina’s job server (jobs.lamina.example), in [REGION], over HTTPS; it then visits the page's host (northwind.example), which sees a visit from lamina’s server, not from the person. **How long:** the PDF is deleted as soon as it is downloaded and after 24 hours at the latest; the address isn't kept after the job. **Used for:** making this one PDF. A 12 `muted` line under the rows: “Pages that need you to sign in will come out as the sign-in page, because your sign-in isn’t sent.” Checkbox “Send this address to lamina’s server to make a PDF”; primary **Send and convert**; local alternative “Open the page in your browser and print it to PDF there.”
- *Convert to* radio: "Word document (.docx)" / "Excel workbook (.xlsx)" with one-line descriptions.
- The [server consent block](design-language.md#718-dialog-proposed) (always shown on web; on desktop only when no local engine applies), in the conversion wording: “This conversion runs on lamina’s server”; What is sent “Q3-board-report.pdf, the whole file · 2.4 MB”; Where to “lamina’s job server (jobs.lamina.example), in [REGION], over HTTPS” (the destination name from [account and cloud](account-cloud-product.md)); How long it stays “Deleted as soon as you download the result, and after 24 hours at the latest. You can delete it sooner.”; What it is used for “Converting this file to Word, once. Not kept, not used for anything else.”; checkbox “Upload Q3-board-report.pdf to convert it to Word”.
- A local alternative line: "Want nothing to leave this device? Export text as Markdown runs in your browser." (links to O16 extraction).
- Footer hint "Tick the box to upload", Cancel, **Upload and convert** (disabled until the consent box is ticked).
- Server jobs panel, per job: title "file → format", status badge (Uploading, Converting, Ready, Failed, Deleted), a step list (Uploaded · size · time; Converted · pages, tables; retention "On the server until 30 Oct 2026, 11:42 or until downloaded"), **Download .docx** (primary, `jobs.download-result`), "Delete from server now" (danger ghost, `jobs.delete-from-server`), and a 12 `muted` line repeating what was agreed: “As you agreed before upload: deleted from the server as soon as you download it, and after 24 hours at the latest.” There is no control that keeps the file longer than the consent said; a later “keep it longer” would need a new consent. A running job shows an indeterminate bar, the current step ("Converting with LibreOffice", "lamina adds title, language and tags") and "Cancel and delete" (`jobs.cancel`).

**States.** Consent (drawn) · uploading (determinate bar with bytes) · converting (indeterminate) · ready (drawn) · downloaded and deleted ("Deleted from the server at 11:52", positive badge) · expired (neutral badge, "Expired and deleted") · failed (danger badge, reason in words, "Try again" re-asks consent) · offline (Upload button disabled with "You’re offline").

**Sign-in and plan.** Local use never needs an account. Server jobs do (they run on lamina’s job server under a plan): when signed out, the dialog shows “Sign in to use server jobs” with **Sign in…** ([H01](account-cloud-product.md#h01-sign-in--create-account)) in place of the consent block, and the local alternative stays visible. When the plan excludes server jobs, the [H11 upgrade prompt](account-cloud-product.md#h11-upgrade-prompt-before-work-starts) appears before the consent block, naming `[PLAN NAME]` and `[PRICE]` (never after work starts, never a lock icon on a feature already in use).

**Behaviour.** Upload starts only after consent; consent is per job, never remembered across documents. The result opens as a new, unsaved document tab; the source is unchanged. Job state survives a reload (job id kept locally) but not sign-out.

**Accessibility.** The consent checkbox label repeats the file name; the dialog `aria-describedby` points at the consent summary. Disabled primary has `aria-disabled` and the visible hint explains why. Job status changes are announced (`role="status"` on the badge).

**Platform differences.** Desktop: Office → PDF runs locally when LibreOffice is installed (the dialog shows "On this computer · LibreOffice 24.x found" and no consent block); PDF → Word/Excel is server-only on both. Desktop draws the same panel in the inspector slot.

**Artboards.** `X02-convert.dc.html` (consent, nothing uploaded yet), `X02-convert-progress.dc.html` (one job ready, one converting), `X02-convert-url.dc.html` (To PDF tab, web page: consent for sending an address).

**Todos.** [E13f](../../todos/ready/E13f-office-and-html-to-pdf.md), [E13g](../../todos/ready/E13g-pdf-to-word-excel.md), [E13e](../../todos/ready/E13e-server-jobs-infrastructure.md).

## X03 Revision browser

**Purpose.** Show each incremental save as a revision and open any earlier one read-only, to see what the file looked like before.

**Entry points.** Status-bar item on files with more than one revision ("4 saves"); View › Revisions; palette `revisions.open`. The signature panel's "See what changed after signing" (G05, `signatures.compare`) opens X04 with the signed-revision preset ([§8.12](design-language.md#812-compare-proposed)), not this panel.

**Layout.** Rail becomes the **Revisions rail panel** (260, back button to the rail tabs). Opening a revision adds a **revision notice strip** ([§7.29](design-language.md#729-strip-proposed): 36 px min, `accent-tint`, Clock icon) under the top bar and puts the canvas in read-only state: modes other than Read are disabled, the top bar context reads "Revision 2 of 4 · read only", the window title adds "— revision 2 (read only)". Inspector shows revision details.

**Content and controls.**

- Timeline, newest first: "Revision 4 · Current", "Revision 3 · Signed" (with "Signature valid" badge), "Revision 2 · Open", "Revision 1 · Original"; each with date, author/producer and a one-line summary of changes.
- Notice strip: "You are viewing revision 2, saved 20 Oct 2026, 17:30. It is read only; the file on disk is unchanged." with "Save this revision as…" (ghost, `revisions.save-revision-as`), "Compare with current" (secondary, opens X04 via `compare.open`), **Back to current** (primary, `revisions.back-to-current`, Escape).
- Inspector: Saved by, Part of the file (byte range), Changed in this save (links that jump to the page), "14 objects changed · Show in object inspector" (O15), switch "Outline changed areas" (dashed accent outlines on the page).

**States.** One revision only: panel empty state "This file has been saved once, so there’s only one version." · damaged `/Prev` chain: notice "Revisions before 2 couldn’t be read; lamina repaired the file on open" · revision with unsupported change: summary "Changed: 3 objects" with no page links.

**Behaviour.** `revisions.open-revision` loads the prefix of the file read-only (`Document::open_revision`); edits are impossible, not just discouraged. "Save this revision as…" writes the bytes up to that revision's end as a new file (exact, not re-serialised). Escape or "Back to current" returns.

**Accessibility.** Timeline is an ordered list of buttons with `aria-current` on the open revision; the notice strip is a labelled region that receives focus when a revision opens. Changed-area outlines are paired with the list in the inspector.

**Platform differences.** Web identical (rail panel, no window title change).

**Artboards.** `X03-revisions.dc.html`.

**Todos.** [E14b](../../todos/ready/E14b-revision-browser.md).

## X04 Compare two versions

**Purpose.** Show what changed between two files or two revisions: text, drawings and images, comments and pages, side by side or overlaid with a pixel difference.

**Entry points.** File › Compare…; palette `compare.open`; X03 "Compare with current"; S04 file-changed strip "Compare" (`file.compare-with-disk`, on-disk preset); G05 “See what changed after signing” (`signatures.compare`, signed-revision preset: older = the signed revision, newer = current, with the signature's scope noted in the alignment note); the cloud conflict (H04, both copies); drag a second PDF onto the Compare window. This is the one compare view of [§8.12](design-language.md#812-compare-proposed); every entry point opens it with a preset rather than drawing its own.

**Layout.** A separate window (desktop) or tab (web) titled "Compare". Top bar: "Compare", two file buttons (Older / Newer, each opens a picker of files and revisions), swap button (Swap icon), a **view switcher** using the mode-switcher pattern (Side by side · Overlay), change navigation (previous, "1 of 9", next), zoom −/%/+, Find, View, "Export report…", More; no page input (two documents). Secondary toolbar: *Show* filters as toggles with counts (Text 5 · Images and drawings 1 · Comments 2 · Pages 1, all pressed in the drawing), "Scroll together" checkbox, alignment note. Rail: **Changes** list grouped by page (side by side) or **Pages** with change counts (overlay). Canvas: two pages, or one overlay.

**Content and controls.** Side by side: removed text struck through on a danger tint (older side), added text underlined on a positive tint (newer side), changed figures outlined; the current change carries the focus outline on both sides. Overlay: unchanged ink in muted grey; "Only in older" orange `#B4540F`, "Only in newer" blue `#2D4A6B` (blue/orange, distinct in lightness, plus legend); toggles "Both versions" / "Pixel difference", "Older version" opacity slider, "Ignore shifts under 1 px" select; numbered change badge on the page.

**States.** Choosing files (empty: two drop targets "Older" and "Newer") · comparing (progress per page) · no differences (empty state "These versions look the same" with "Text, drawings and comments match") · a replaced page (drawn: change 9, “Page replaced”, “Surveyor rota, September” → “Surveyor rota, October”; both files have 24 pages; the overlay rail lists page 12 as “page replaced”) · different page counts ("Page 12 is new" note; unmatched pages listed as Page added/removed) · web over `Limits` ("Comparing the first [N] pages") · report exported (toast).

**Behaviour.** Read-only on both files. Next/previous change: `compare.next-change` / `compare.previous-change`, shortcuts `⌥⌘↓` / `⌥⌘↑` on macOS (palette precedent) and none on Windows/Linux (avoid `Ctrl Alt`). Export report: PDF summary or JSON (`compare.export-report`), a new file.

**Accessibility.** Changes list is the full textual equivalent of the visual diff (each item says what and where, including old → new values); colour is never alone (strike/underline, legend words). Slider has a label and value text.

**Platform differences.** Web: opens in a new tab with the same layout; files must both be opened in the page (no folder access).

**Artboards.** `X04-compare.dc.html` (side by side), `X04-compare-overlay.dc.html` (overlay, pixel difference).

**Todos.** [E14c](../../todos/ready/E14c-compare.md).

## X05 Batch runner

**Purpose.** Apply a saved recipe of steps to many files and get a per-file result, without changing the originals.

**Entry points.** File › Batch…; palette `batch.open`; "Record steps from an open document" from a document's More menu; opening a `.lamina-recipe` bundle.

**Layout.** Its own window "Batch — <recipe>". Top bar: recipe name, "Recipe · saved 27 Oct 2026", Open recipe… (`batch.open-recipe`), Save recipe (`batch.save-recipe`), run control (Run on N files `batch.run` / Stop after current files `batch.stop`), More. No page input or zoom: the window has no document view. Left (300): **Steps** list. Main: **Files** with add buttons, progress and a results table. Right: step settings inspector.

**Content and controls.**

- Steps: numbered rows (title + what it applies to, from the recipe's selectors: "all pages", "pages labelled ‘draft notes’", "whole document"), "Add a step…", "Record steps from an open document".
- Files table (scrolls horizontally at narrow widths): File · Pages · Status badge · Result. Statuses: Waiting, Step n of 5, Done (positive), Done with warnings (warning; "Step 3: no page is labelled ‘draft notes’"), Skipped (warning; "Signed: saving would break the signature · Run anyway…"), Not processed (danger; "Needs a password · Enter password…").
- Output: copies next to each file with a suffix (step 5), never overwrite.
- Inspector: the selected step's parameters (locked while running), selector choices, bundled font/blob note, note that a non-matching selector is a warning.
- "Export run report (JSON)".

**States.** Empty queue (empty state "Add files or a folder to run ‘Board pack tidy-up’ on them") · ready (primary "Run on 12 files") · running (drawn) · finished ("12 files · 9 done · 2 warnings · 1 not processed", toast) · stopped · recipe from a newer lamina (error with the version).

**Behaviour.** Each file runs in its own transaction; a failure never touches other files or the original. Signed files are skipped by default. Concurrency is shown in the status bar ("2 files at a time").

**Accessibility.** Table with `scope="col"` headers; progress bar with values; status words plus icons. Reordering steps uses Move up/down in each step's More menu (drag optional).

**Platform differences.** Web: no folders; "Add files…" only, outputs downloaded as a zip; optional **server batch** (E14e) shows the server consent block before any upload (drawn: `X05-batch-server-consent`): dialog “Run ‘Board pack tidy-up’ on 12 files”; What is sent “12 files, the whole of each · 31.6 MB in all, and the recipe with its bundled font”, with a “Show the 12 files” disclosure listing each file and its size; How long “Each file and its result are deleted as soon as you download the zip, and after 24 hours at the latest”; Used for “Running ‘Board pack tidy-up’ on these 12 files, once”; one checkbox naming the count and size (“Upload these 12 files (31.6 MB) to run ‘Board pack tidy-up’”); local alternative “Run on the desktop app”; **Upload and run** disabled until ticked. The run then happens in the Server jobs panel (X02). The server batch depends on [full-editor §9](../../plans/full-editor.md#9-open-questions) question 3 (is batch also a hosted service?); if that is answered “desktop and CLI only”, E14e closes as won't-do and the web shows only the desktop suggestion. Desktop runs locally only.

**Artboards.** `X05-batch.dc.html` (desktop, running), `X05-batch-server-consent.dc.html` (web, server batch consent before upload).

**Todos.** [E14d](../../todos/ready/E14d-batch-runner.md), [E14e](../../todos/ready/E14e-server-batch.md).

## X06 Accessibility checker

**Purpose.** Run the machine-checkable PDF/UA (Matterhorn) checks and font/Unicode checks, take the person to each issue and fix it where a fix is safe.

**Entry points.** Edit mode › Tools ▾ › Accessibility; palette `a11y.check`; Document properties (R08) "Tagged: yes · Check accessibility"; X08 failure "Fix in the checker".

**Layout.** Edit mode with the **Accessibility tool toolbar**: tool name, toggles Check · Tags · Reading order, "Document language…", "Save as PDF/UA…", issue count, Done. Rail (280): check results grouped by kind. Canvas: the page with the issue outlined and labelled. Inspector: the selected issue with its fix.

**Content and controls.**

- Results header: "Check results", "Check again", summary "PDF/UA-1 machine checks · 5 issues left · 2 fixed".
- Groups: Images, Text and fonts, Tables, … each issue as a link row with a warning icon, a plain title ("Figure has no alt text", "Characters can’t be read as text", "Header cells don’t say what they head") and its location.
- "Fixed in this session" group (positive check, "Undo with ⌘Z").
- Footer: "Passed: N machine checks" and "Need a person: …" with a link to the manual checklist.
- Inspector: title, "Page 3 · Matterhorn 13-004", one-sentence explanation of the effect on a screen-reader user, the fix fields (alt text textarea pre-filled from the chart's labels with "Check it before saving"), actions "Set alt text" (primary), "Mark as decorative", "Next issue".

**States.** Not run (empty state with "Check this document") · checking (progress in the rail header) · issues (drawn) · all machine checks pass (positive notice "No machine-detectable problems" plus the "Need a person" list — never "Accessible") · untagged file (single issue "This file has no tags" with "Start tagging" → X07).

**Behaviour.** One-click fixes (set language, display title, role-map repair) are ordinary undoable ops; fixes that need judgement (alt text, artifacts) always go through the inspector. Checks re-run on the affected page after each fix. Save is incremental. On a file with approval signatures (the sample) the status bar shows the §7.13 note “Signed file. Saving adds your changes after the signature; the signature stays valid.” while changes are unsaved; only a certified (DocMDP) file whose permissions forbid the change shows the §7.13 banner.

**Accessibility.** Issues list is a `nav` of links; selecting one moves focus to the inspector's first field. Outline + text label on the page, never colour only.

**Platform differences.** Web identical.

**Artboards.** `X06-accessibility.dc.html`.

**Todos.** [E15d](../../todos/ready/E15d-accessibility-checks.md).

## X07 Tags and reading order

**Purpose.** See and edit the structure tree, the order a screen reader follows, alt text and language, and tag untagged content.

**Entry points.** Accessibility tool › Tags / Reading order; X06 "Start tagging"; right-click on page content › Tag as…; palette `tags.show`, `tags.tag-selection-as`.

**Layout.** Same tool toolbar with Tags and Reading order pressed, plus "Tag selection as" select and "Tag" button (disabled until content is selected). Rail: **Tags** tree (`role="tree"`). Canvas: reading-order overlay — each tagged block outlined (dashed accent) with a numbered badge in the margin; artifacts outlined in `control-border` and labelled "Artifact". Inspector: the selected element.

**Content and controls.** Tree rows: type (H1, P, Figure, Caption, Section, Document · language) and a text excerpt; numbered badges match the page. Inspector: Tag select, Alt text, Language ("Same as document (English, UK)"), Reading order "Move earlier" / "Move later" (disabled at the ends) with the keyboard and drag hint, "Remove tag, keep content" (danger ghost). Tag-as list: Paragraph, Heading 1–6, List, Table, Figure, Artifact (decorative); tables open a small header-scope step.

**States.** Untagged file: rail empty state "This file has no tags" with "Tag automatically" (heuristic, marks the result Approximate for review) and "Tag by hand". Selection with mixed tags: inspector shows "Several tags". Signed file: the §7.13 note (drawn), or the banner on a certified file, as in X06.

**Behaviour.** `tags.move-earlier` / `tags.move-later` (`⌥↑` / `⌥↓` on macOS and `Alt Up` / `Alt Down` elsewhere, active only while the tree or page has focus); dragging a tree row is optional. Each change is one undoable `Document`/`Content` op; ParentTree is maintained invisibly.

**Accessibility.** APG tree pattern (arrow keys, `aria-level`, `aria-setsize`, `aria-posinset`, `aria-expanded`, `aria-selected`); the overlay badges are `aria-hidden` (the tree is the accessible equivalent). Move buttons are the non-drag alternative (WCAG 2.5.7).

**Platform differences.** Web identical.

**Artboards.** `X07-tags.dc.html`.

**Todos.** [E15b](../../todos/ready/E15b-structure-tree-editing.md), [E15c](../../todos/ready/E15c-tagging-tools.md).

## X08 Save as PDF/UA or PDF/A

**Purpose.** Save a copy that conforms to PDF/UA-1, PDF/A-2u/2b, or both, refusing (with a report and fixes) when checks fail.

**Entry points.** File › Save a copy to a standard…; Accessibility toolbar "Save as PDF/UA…"; palette `file.save-as-pdfua`, `file.save-as-pdfa`.

**Layout.** Dialog (560) over the dimmed shell, drawn in Read mode for both states (it saves a copy, so the open file is unchanged; opened from the Accessibility toolbar it shows over that toolbar instead).

**Content and controls.** Standard radio group (PDF/UA-1 · PDF/A-2u · PDF/A-2b · PDF/UA-1 and PDF/A-2u together) · **Check result** section: badge ("All machine checks pass" positive, or "Not saved" danger), what lamina will add (identifier in metadata, tagged marker, display title), warning note "Machines can’t check everything" with the manual checklist link · Save as filename with "A new file next to the original" · Cancel / **Save PDF/UA copy**. Failure: heading "3 things stop this file being PDF/A-2b", each problem in plain words with its location and an inline fix ("Embed font", "Remove from copy", "Add sRGB"), "Export the full report", blocked reason in danger text, primary "Fix all 3 and check again".

**States.** Checking (indeterminate bar in the result section; Save disabled) · pass (drawn) · fail (drawn) · saved (toast "Saved Q3-board-report-UA.pdf · Open" ) · signed source (note: the copy is a full rewrite and won't carry the signature; the original keeps it).

**Behaviour.** Fixes apply to the copy only. Conversion runs locally; no server. The open document is unchanged.

**Accessibility.** Result is a `role="status"`; problems are a list whose fix buttons name the action; the blocked reason is text, not only a disabled button.

**Platform differences.** Web: "Save as" becomes "Download"; file name field stays.

**Artboards.** `X08-save-conformance.dc.html`, `X08-save-conformance-failed.dc.html`.

**Todos.** [E15e](../../todos/ready/E15e-pdfua-output.md), [E16e](../../todos/ready/E16e-preflight-and-conversion.md).

## X09 Optimise

**Purpose.** Save a smaller or faster-to-open copy, with an estimate of the result before saving.

**Entry points.** File › Save an optimised copy…; palette `file.save-optimised`; Document properties size line "Reduce size…".

**Layout.** Dialog (560).

**Content and controls.** Before/after size bars ("Now 2.4 MB", "After ≈ 1.1 MB", one `role="img"` with a text label) · Preset toggles (Smallest · Balanced · Print quality, an argument of `file.save-optimised`, not separate ids; editing any option turns the preset to none) · options list, each with its estimated saving: Shrink images (ppi input, JPEG quality select, "11 images above 150 ppi. Masks and transparency are kept."), Keep only the characters each font uses, Remove unused and duplicate objects, Pack objects into compressed streams, Fast web view · signed-file warning note · Save as filename · footer "Sizes are estimates from a trial run", Cancel / **Save optimised copy**.

**States.** Estimating (values show "…" and the bars a skeleton) · ready (drawn) · nothing to gain ("This file is already compact" note, Save still allowed) · saved (toast with the real before/after) · web over `Limits` (desktop suggestion).

**Behaviour.** Always a full rewrite (`RequiresRewrite`), always a new file; the signed warning appears before saving, never after.

**Accessibility.** Each option's estimate is in the same row as its checkbox label; inputs labelled.

**Platform differences.** Web downloads the copy; desktop can embed system fonts, web only supplied ones (font option hint changes).

**Artboards.** `X09-optimise.dc.html`.

**Todos.** [E16b](../../todos/ready/E16b-structural-optimiser.md), [E16c](../../todos/ready/E16c-images-and-fonts.md).

## X10 Print production

**Purpose.** Prepare files for commercial print: preflight against a profile, output intent, separations and overprint preview, page boxes, colour conversion, and imposition.

**Entry points.** Edit mode › Tools ▾ › Print production; palette `print.preflight`, `print.impose`; File › Impose…

**Layout.** Edit mode with the **Print production tool toolbar**: Preflight, Separations, Page boxes toggles; "Overprint preview" switch; "Convert colours…", "Impose…"; Done. Rail (280): Preflight. Canvas: the page with trim marks and the problem marked. Inspector: Separations (inks with show/hide checkboxes and coverage at the pointer, total ink vs limit), Output intent, and the fix for the selected error. Imposition: a wide dialog (820) with options left and a live sheet preview right. The current preflight result carries the keyboard focus ring. Photos on the mock pages are flat `#4E5A63` blocks (no gradients).

**Content and controls.** Preflight profile select (PDF/X-4 · PDF/X-1a · Office printer; rules are a data file, "Edit profile…"), Run, summary counts, results grouped Errors / Warnings with badges (word + colour) and locations. Imposition: Layout radio (2 per sheet, 4 per sheet, Booklet, Custom grid…), Sheet select, Pages range, Creep with help, Printer’s marks checkboxes, preview "sheet 1 of 4" with front/back and previous/next sheet, "16 pages · 4 sheets", Cancel / **Create booklet**.

**States.** Preflight not run · running · pass ("All 29 rules passed" positive) · issues (drawn). Separations off: inspector shows the selected issue only. Imposition with a page count not divisible by 4: note "2 blank pages will be added at the end".

**Behaviour.** Separations and overprint preview are view-only. "Set output intent…" (`print.set-output-intent`) is an incremental change; colour conversion ("Convert colours…", `print.convert-colours`) and flattening are rewrites saved as a copy (signed warning as X09). Imposition always creates a new document.

**Accessibility.** Ink rows are labelled checkboxes with the ink name (swatch is decorative); preview sheets are `role="img"` with a description of which pages sit where; prev/next sheet buttons are labelled.

**Platform differences.** Web identical; large print files get the desktop suggestion when over `Limits`.

**Artboards.** `X10-preflight.dc.html` (`annual-review-2026.pdf` page 5 in Edit mode: Preflight and Separations pressed, overprint preview on; PDF/X-4 results "2 errors · 3 warnings · 24 rules passed"; the 180 ppi photo selected and marked on the page with an Error label and the total ink at the pointer; inspector with the fix (Replace image… / Accept for this file), five inks including the spot ink "Northwind Teal", total ink 318% over the 300% limit, output intent Coated FOGRA39; status bar shows the trim and bleed), `X10-imposition.dc.html` (wide dialog over the dimmed shell: Booklet chosen, SRA3 landscape (450 × 320 mm, the default: two A4 pages with 3 mm bleed and crop marks don't fit on A3, and the help under the select says so), pages 1–16, binding left, creep 0.2 mm, crop marks; preview of sheet 1 front with pages 16 and 1, Front/Back toggle, previous/next sheet; Save as `annual-review-2026-booklet.pdf`; note that links and form fields aren't carried into the booklet).

**Todos.** [E16d](../../todos/ready/E16d-colour.md), [E16e](../../todos/ready/E16e-preflight-and-conversion.md), [E16f](../../todos/ready/E16f-imposition.md).

## X11 Layers and actions

**Purpose.** Show, hide and edit optional-content layers; inspect and author actions (go to, open web address, show/hide, reset, JavaScript as text) on buttons, links, fields, pages and file open.

**Entry points.** Layers: status-bar item "4 layers · 1 hidden in your view" (only on files with layers); View › Layers; palette `layers.show`. Actions: Edit mode › Tools ▾ › Actions; selecting a link, button or field in Edit mode (inspector Actions section); palette `actions.open`.

**Layout.** Layers: **Layers rail panel** (back to tabs) with eye toggles and a lock glyph; inspector (288) for the selected layer in Edit mode, under the §8.1 Edit toolbar (`Select · Edit text · Add text · Image · Shape ▾ · Link │ Page marks ▾ · Replace… │ Redact · Protect · Tools ▾`, the same markup as add-edit-content A01, Select pressed). Actions: the **Actions tool toolbar** replaces the Edit toolbar (§8.1 “tools of Edit”: title “Actions” · Add an action… · When the file opens… │ Show only JavaScript (toggle) · context note · Done); rail lists every action in the document grouped by place (When the file opens, Buttons, Links, With JavaScript); the selected button outlined on the page; inspector (288) with the action list.

**Content and controls.** Layers rail: explanation "The eye shows or hides a layer for you only. It doesn’t change the file.", rows (eye button `aria-pressed`, name, "· hidden", locked icon with label), "Show all", "Reset to file’s view". Layer inspector: Name, *When someone opens the file*: Visible switch, Prints select, Lock checkbox; Order (Move up/Move down); New layer; Delete layer… (confirm dialog). Actions inspector: When select (Clicked, Pointer enters, Pointer leaves, Gets focus), ordered steps with More menu each, the JavaScript step with badge "Not run by lamina", a textarea in `mono` 12.5/1.6 on a white `field` fill, and "Saved as text. lamina never runs it; other readers may, if their settings allow."; "Add an action…" with the list of supported kinds; "Remove all actions".

**States.** File without layers: no status item and no panel. Locked layer: eye disabled with "Locked by the author" tooltip text. File with JavaScript: status-bar note "This file contains JavaScript. lamina never runs it." (the standard wording of [§8.5](design-language.md#85-trust-surfaces)). Recognised scripts (format, keystroke and calculate scripts lamina knows, as in Forms P02) are listed with a positive “Handled natively” badge and say what lamina does instead (“recognised as a sum of 3 fields; lamina adds them up without running the script”); only unrecognised ones carry the JavaScript badge. The rail header counts both (“2 with JavaScript, 1 handled natively”).

**Behaviour.** Eye toggles are view state (not undoable, not saved); inspector edits are `UpdateOcg`/`SetOcgDefaults`/`SetActions` ops, undoable and incremental. JavaScript is not executed: this is the working assumption of the design language, not a decision. ADR 0021 (the PDF JavaScript policy, [E8a](../../todos/ready/E8a-adr-0021-javascript-policy.md), [full-editor §9](../../plans/full-editor.md#9-open-questions) question 1) is still pending; if it allows a sandbox, the wording and the badges change.

**Accessibility.** Eye buttons are named "Hide <layer>"/"Show <layer>" with `aria-pressed`; step lists are ordered lists; reorder uses buttons.

**Platform differences.** Web identical (layers panel is a reader feature on both shells).

**Artboards.** `X11-layers.dc.html` (`mill-lane-A102.pdf` in Edit mode with the Layers rail panel: Dimensions locked, Electrical hidden, Furniture selected with its eye button focused; inspector for Furniture with Name, Visible switch, Prints select, Lock checkbox, Move up/Move down, Delete layer…; status-bar item "4 layers · 1 hidden in your view"), `X11-actions.dc.html` (`annual-review-2026.pdf` page 2 with the Actions tool: rail grouped by place, the "Read the full accounts" button selected on the page; inspector with When = Clicked and two ordered steps, Open web address and Run JavaScript with the "Not run by lamina" badge and the JavaScript textarea focused; the Total rent field's calculate script listed as “Handled natively”; page links drawn in page ink with an underline; status-bar note “This file contains JavaScript. lamina never runs it.”).

**Todos.** [E17c](../../todos/ready/E17c-layers.md), [E17b](../../todos/ready/E17b-action-editor.md).

## X12 Measurement and media

**Purpose.** Measure distance, perimeter and area at the drawing's scale and save the measurements as annotations; show embedded video/audio with a poster and play control, and 3D models as their saved picture with a notice.

**Entry points.** Measure is a tool of Comment ([§8.1](design-language.md#81-modes-and-their-secondary-toolbars)); Comment has no Tools menu, so it opens from the palette (`measure.distance`, `measure.perimeter`, `measure.area`), the Comment toolbar's `…` menu, and the context note "This page has a scale (1:100) · Measure" on pages with a viewport `/Measure`. Media: on the page; selecting it opens the inspector.

**Layout.** Measure: Comment mode with the **Measure tool toolbar** in the secondary-toolbar slot; the page centred, with measurements drawn over it; inspector (288) for the selected measurement; no rail. Media: Read mode; the page with posters in place of the media; inspector (288) for the selected media item (read-only facts and “Save video as…”, which saves out and doesn't change the file).

**Content and controls.** Measure toolbar: Distance · Perimeter · Area (`measure.distance`, `measure.perimeter`, `measure.area`), Scale "1:100 · from the drawing", Calibrate…, Units select (m, cm, mm, ft and in), "Enter points…", hint "Click each corner, double-click to finish · Esc to cancel", Done. Canvas: measurements with value labels; the selected one has square handles. Inspector: Area, Perimeter, Sides, Scale (definition list), Label, Precision (0.1 · 0.01 · 1), "Edit points as numbers…", Copy value, Delete measurement. Status bar: pointer coordinates in drawing units (and latitude/longitude on geospatial PDFs). Media: video poster with a centred 64 px play button and a duration chip; 3D poster with an inline info note "3D model · shown as its saved picture. lamina can’t turn 3D models yet." and "Save model as…"; inspector: File, Size, Length, Format, Captions, Playback ("Only when you press play"), "Save video as…".

**States.** No scale in the file: toolbar shows "Scale: not set · Calibrate…" and values are in page units (pt) until calibrated by drawing a known length. Calibrate: small dialog "This line is [ ] m long". Video playing: native controls inside the frame, Escape stops focus trap. Unsupported codec on web: poster stays with "Can’t play this format here · Save video as…". 3D: never interactive in this round.

**Behaviour.** Measurements are annotations (Comment-mode rules for signed files); they are undoable and incremental. Media never autoplays and never fetches remote renditions without the consent rule (remote media: "This video is on example.org · Play from there?" names the host first).

**Accessibility.** "Enter points…" and "Edit points as numbers…" are the non-drag alternative for drawing measurements (WCAG 2.5.7); value labels on the page are mirrored in the inspector; the play button names the video and its length.

**Platform differences.** Web: media limited to browser codecs; measurement identical. Desktop plays more formats.

**Artboards.** `X12-measure.dc.html` (`mill-lane-A102.pdf` in Comment mode with the Measure toolbar, Area pressed, scale 1:100 from the drawing; the living-room area 22.0 m² (5.50 m × 4.00 m, perimeter 19.0 m, measured to the wall centrelines so the shape sits on the drawing's dimension lines) selected with square handles and a 4.20 m distance across the kitchen; inspector with area, perimeter, sides, Label focused, Precision; pointer coordinates in the status bar), `X12-media.dc.html` (`annual-review-2026.pdf` page 6 in Read mode: video poster with the focused play button and 2:14 chip, 3D model as its saved picture with the info note; inspector with file, size, length, format, captions, playback policy and Save video as…). The play button's focus ring is white (2 px, offset 2) because the `focus` teal has too little contrast on the dark poster; this is the sanctioned variant for focus on dark media (see the proposed additions).

**Todos.** [E17e](../../todos/ready/E17e-measurement.md), [E17d](../../todos/ready/E17d-media.md), [E17f](../../todos/ready/E17f-3d-detection.md).

## Consent wording per job

The [server consent block](design-language.md#718-dialog-proposed) keeps its structure (Share icon, heading, why, four rows, one checkbox, disabled primary with a visible reason, local alternative) and changes its words with the job. “Where to” is always “lamina’s job server (jobs.lamina.example), in [REGION], over HTTPS” (the destination name used by [account and cloud](account-cloud-product.md)); a job that makes the server contact another host names that host too. “How long it stays” always ends at download and at 24 hours at most, and nothing after upload can extend it.

| Job | Heading | What is sent | How long it stays | What it is used for | Checkbox | Primary |
| --- | --- | --- | --- | --- | --- | --- |
| Convert a file (X02) | This conversion runs on lamina’s server | The file, whole, with its size | Deleted as soon as you download the result, and after 24 hours at the latest | Converting this file to *format*, once | Upload *file* to convert it to *format* | Upload and convert |
| Web page → PDF (X02) | Making a PDF of a web page runs on lamina’s server | The address only; no cookies, passwords or sign-ins | The PDF: at download, 24 hours at most; the address isn't kept | Making this one PDF | Send this address to lamina’s server to make a PDF | Send and convert |
| OCR on the server (X01) | Reading the text runs on lamina’s server | Only the chosen pages, as a copy, with their size | Deleted as soon as the text comes back, 24 hours at most | Reading the text on these *n* pages, once | Upload pages *range* of *file* to read their text | Upload and read text |
| Server batch (X05) | This batch runs on lamina’s server | *n* files with the total size, the recipe and its bundled fonts; a disclosure lists every file | Each file and result: at download of the zip, 24 hours at most | Running ‘*recipe*’ on these *n* files, once | Upload these *n* files (*size*) to run ‘*recipe*’ | Upload and run |

All four end the “used for” row with “Not kept, not used for anything else.” Consent is per job and never remembered.

## New components

Merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29; see its decisions table.

## Proposed design-language additions

Earlier proposals were merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29. New since the merge:

- **Server consent block wording** (§7.18): replace “four fixed rows … a checkbox whose label repeats the file name” with “four rows worded for the job (see [Consent wording per job](#consent-wording-per-job)); a checkbox whose label names what is sent (file, pages, address or file count)”. Add: the retention promise is set before upload; panels after upload repeat it and offer only “Delete from server now”, never a longer stay.
- **Command ids** for the appendix (`namespace.verb-object`, §8.8):

  | Id | Title | Area and screen | Shortcut |
  | --- | --- | --- | --- |
  | `batch.open-recipe` | Open recipe… | Convert and produce X05 | — |
  | `batch.run` | Run batch | Convert and produce X05 | — |
  | `batch.save-recipe` | Save recipe | Convert and produce X05 | — |
  | `batch.stop` | Stop after current files | Convert and produce X05 | — |
  | `jobs.cancel` | Cancel and delete server job | Convert and produce X02 | — |
  | `jobs.delete-from-server` | Delete from server now | Convert and produce X02 | — |
  | `jobs.download-result` | Download result | Convert and produce X02 | — |
  | `jobs.open` | Server jobs | Convert and produce X02 | — |
  | `print.convert-colours` | Convert colours… | Convert and produce X10 | — |
  | `print.set-output-intent` | Set output intent… | Convert and produce X10 | — |
  | `revisions.back-to-current` | Back to current | Convert and produce X03 | Escape (revision open) |
  | `revisions.save-revision-as` | Save this revision as… | Convert and produce X03 | — |

  The optimise presets (Smallest, Balanced, Print quality) are an argument of `file.save-optimised`, not ids.
- **Mockup-only colours** (§2.5) used on these boards' document content: scanned paper `#F5F2EC` and `#F3F0EA`, scan ink and smudges `#B8B2A8` and `#A9A296` (X01); photo blocks `#4E5A63` (X10, X12; flat, no gradients); the fictional spot ink “Northwind Teal” `#1F7A80` (X10 separations and the annual review's buttons in X11). Page links on mock pages use page ink with an underline.
- **Focus ring on dark media** (§7.0, §7.40): on a dark poster the ring is 2 px white with offset 2 (X12 media), because `focus` teal has too little contrast there.
