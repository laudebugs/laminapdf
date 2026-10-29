# Paper: convert, compare and produce

- **Status:** Draft (designed 2026-10-29 in the Paper direction)
- **Canvas:** [lamina Paper — Convert, compare and produce](https://claude.ai/artifact/GwyjtEGo2D2wMo4VUubKKZ) (artboards `X01`–`X12`)
- **Covers:** screens X01–X12 of the [screen inventory](screens.md#x-convert-compare-batch-accessibility-print-production-advanced-e13e17): OCR, Office/HTML conversion with server consent, revisions, compare, batch, accessibility checking and tagging, PDF/UA and PDF/A output, optimisation, print production, layers and actions, measurement and media.
- **Follows:** the [Paper design language](design-language.md) (tokens, components, patterns) and the [UI/UX plan](../../plans/ui-ux.md) principles §4 and layout §5.
- **Plan:** [full-editor §E13](../../plans/full-editor.md#e13-ocr-and-conversion), [§E14](../../plans/full-editor.md#e14-compare-revisions-and-batch), [§E15](../../plans/full-editor.md#e15-accessibility), [§E16](../../plans/full-editor.md#e16-optimisation-and-print-production), [§E17](../../plans/full-editor.md#e17-advanced-interactions).

These are power features, so the rule from principle 1 matters most here: none of them adds permanent chrome. Each one is reached from a menu, the palette or a contextual notice, and opens either a **dialog** (one-shot jobs: OCR options, convert, save to a standard, optimise, impose), a **tool toolbar** in the secondary-toolbar slot (ongoing work on the open document: Accessibility, Print production, Measure), a **rail panel** (Revisions, Layers), or its own **window** (Compare, Batch).

Three rules hold across the area:

1. **Nothing leaves the device without an explicit, per-job yes.** Every server job (OCR on the server, Office ↔ PDF, PDF → Word/Excel, server batch) shows the [server consent block](#server-consent-block) first: what file, how big, to which host, for how long, for what. The upload button stays disabled until the person ticks the box naming the file. The network indicator then names the host and counts the approved uploads (principle 7).
2. **Copies, not surprises.** Jobs that rewrite the file (optimise, PDF/A, PDF/UA, imposition, conversions) save a **new file** and say so; jobs that add to the file (OCR text layer, tags, alt text, layers, actions, measurements) become ordinary unsaved changes with undo, saved incrementally. Where the result would lose or break a signature, the dialog says it before the person commits (§8.4 explicit save).
3. **Machine checks are not the whole truth.** Accessibility and conformance results always separate what lamina checked from what a person must still check.

Platform notes: all X screens are drawn as the **macOS desktop window** except X02, drawn as the **web app** because the inventory lists web first and conversion is server-only there. The web shell is the same minus the window frame; differences are listed per screen. Windows/Linux differ only in window controls, menu placement and shortcut chips (`Ctrl` words instead of glyphs).

Sample files: `Q3-board-report.pdf` (24 pages; this area adds a scanned *Appendix B. Contractor invoices* on pages 23–24 and four saved revisions), plus three fictional files: `annual-review-2026.pdf` (16-page print piece with a video and a 3D model), `mill-lane-A102.pdf` (2-page floor plan with layers) and the batch folder `~/Board/2026 Q3`.

## X01 Make searchable (OCR)

**Purpose.** Add an invisible text layer to scanned pages so they can be found, selected, copied and read aloud, without touching the page images.

**Entry points.** Status-bar notice "2 scanned pages have no text · Make searchable…" (shown when pages have images and no text); More menu › *Make searchable…*; palette `ocr.make-searchable`; native menu Tools › Make searchable…. No shortcut (low frequency).

**Layout.** Options: dialog (560 wide) over the dimmed shell, Read mode, rail on Pages with the scanned thumbnails labelled "No text". Progress and review: the dialog closes and a **job panel** docks in the inspector slot ("Make searchable · On this computer · English"); the canvas shows the page with recognised words outlined; the rail closes to give the page room.

**Content and controls.**

- *Pages* radio group: "Pages without text · 23–24 (2 pages)" (default), "All pages · 24 (pages with text keep it)", "Pages" + range input.
- *Languages on these pages*: select + "Add a language" ghost; help "each extra one makes recognition slower".
- *Where to read the text*: two radio cards. "On this computer" (default on desktop; names the installed engine and its install date) and "On lamina’s server" (says a copy is uploaded and that the person will confirm what is sent first). Choosing the server card swaps the primary button to "Continue…", which opens the [server consent block](#server-consent-block) before any upload.
- "Tag the recognised text for screen readers" checkbox (on when the file is tagged).
- Info note: one unsaved change; saving appends the layer.
- Buttons: Cancel · **Make searchable** (`ocr.run`).
- Job panel: determinate progress ("Reading page 24 · 1 of 2 pages done"), per-page list (words found, how many to check), **Words to check**: each low-confidence word with its line, what was read ("Re-p1aster"), an editable input pre-filled with the best guess and "Accept"; switch "Outline recognised words"; "Add text to 2 pages" (primary, disabled until all pages are read) and "Stop" (danger ghost).

**States.**

- *Engine not installed* (desktop): the "On this computer" card reads "OCR engine not installed · Download (about [SIZE])"; the download is verified and signed (E13d) and is itself a network request the indicator shows.
- *Web, file too large for in-browser OCR*: the local card is disabled with the reason ("Files over [LIMIT] pages need the desktop app or the server").
- *In progress*: drawn (`X01-ocr-progress`). Pages already read can be reviewed while others run.
- *Done*: panel shows "2 pages read · 3 words checked"; pressing "Add text to 2 pages" applies one `AddOcrLayer` op per page as a single undo step, closes the panel and shows the toast "Text added to 2 pages · Undo". Top bar: "1 unsaved change".
- *Stopped/cancelled*: nothing is added; toast "Stopped. Nothing was added."
- *Error* (engine crash, unreadable page): the page row shows a danger badge "Couldn’t read" with "Try again"; other pages still apply.
- *Signed file*: a warning note in the dialog: "Adding text changes page content. This file is certified to allow no changes, so the signature would no longer be valid. Save a copy instead." with the save banner (§7.13) after applying.

**Behaviour.** Recognition runs in a worker (web) or a sidecar (desktop); the page stays readable and scrollable. The text layer uses render mode 3 so print and appearance don't change. Undo removes the layer. Save is incremental.

**Accessibility.** Dialog focus starts on the first radio; Tab order: pages, languages, where, tag checkbox, buttons. The progress bar has `aria-valuenow`; page completion is announced politely. Words to check are real inputs with labels naming the line and the misreading. Word outlines are decorative (`aria-hidden`); the list is the accessible equivalent.

**Platform differences.** Web: in-browser OCR only for small documents (E13d), otherwise server only; the status-bar text and consent are identical. Phone/tablet: not designed in this round; the dialog becomes a large sheet and the job panel a medium sheet.

**Artboards.** `X01-ocr.dc.html` (options), `X01-ocr-progress.dc.html` (in progress with review).

**Todos.** [E13c](../../todos/ready/E13c-ocr-text-layer.md), [E13d](../../todos/ready/E13d-local-ocr.md).

## X02 Convert to and from PDF

**Purpose.** Convert this PDF to Word or Excel, or Word/Excel/PowerPoint/web pages to PDF, as a job whose result opens as a new document; on the web this always needs the server, so the job sheet is built around consent.

**Entry points.** More menu › *Convert…*; palette `convert.from-pdf`, `convert.to-pdf`; home screen drop zone when an Office file is dropped (opens the dialog on the "To PDF" tab); desktop native menu File › Convert.

**Layout.** Dialog (560) over the dimmed web reader. Jobs then live in a **Server jobs** panel (320, inspector slot) and a status-bar item "2 server jobs · 1 ready" that reopens it.

**Content and controls.**

- Tabs: "From this PDF" · "To PDF (Word, Excel, PowerPoint, web page)". The To-PDF tab has a file picker and a URL field (HTML → PDF via the browser service).
- *Convert to* radio: "Word document (.docx)" / "Excel workbook (.xlsx)" with one-line descriptions.
- The [server consent block](#server-consent-block) (always shown on web; on desktop only when no local engine applies).
- A local alternative line: "Want nothing to leave this device? Export text as Markdown runs in your browser." (links to O16 extraction).
- Footer hint "Tick the box to upload", Cancel, **Upload and convert** (disabled until the consent box is ticked).
- Server jobs panel, per job: title "file → format", status badge (Uploading, Converting, Ready, Failed, Deleted), a step list (Uploaded · size · time; Converted · pages, tables; retention "On the server until 30 Oct 2026, 11:42 or until downloaded"), **Download .docx** (primary), "Delete from server now" (danger ghost), checkbox "Delete from the server as soon as it is downloaded" (on by default); a running job shows an indeterminate bar, the current step ("Converting with LibreOffice", "lamina adds title, language and tags") and "Cancel and delete".

**States.** Consent (drawn) · uploading (determinate bar with bytes) · converting (indeterminate) · ready (drawn) · downloaded and deleted ("Deleted from the server at 11:52", positive badge) · expired (neutral badge, "Expired and deleted") · failed (danger badge, reason in words, "Try again" re-asks consent) · offline (Upload button disabled with "You’re offline"). A plan that excludes server jobs shows the price and plan before the consent block (H11 rule: never after work starts).

**Behaviour.** Upload starts only after consent; consent is per job, never remembered across documents. The result opens as a new, unsaved document tab; the source is unchanged. Job state survives a reload (job id kept locally) but not sign-out.

**Accessibility.** The consent checkbox label repeats the file name; the dialog `aria-describedby` points at the consent summary. Disabled primary has `aria-disabled` and the visible hint explains why. Job status changes are announced (`role="status"` on the badge).

**Platform differences.** Desktop: Office → PDF runs locally when LibreOffice is installed (the dialog shows "On this computer · LibreOffice 24.x found" and no consent block); PDF → Word/Excel is server-only on both. Desktop draws the same panel in the inspector slot.

**Artboards.** `X02-convert.dc.html` (consent, nothing uploaded yet), `X02-convert-progress.dc.html` (one job ready, one converting).

**Todos.** [E13f](../../todos/ready/E13f-office-and-html-to-pdf.md), [E13g](../../todos/ready/E13g-pdf-to-word-excel.md), [E13e](../../todos/ready/E13e-server-jobs-infrastructure.md).

## X03 Revision browser

**Purpose.** Show each incremental save as a revision and open any earlier one read-only, to see what the file looked like before.

**Entry points.** Status-bar item on files with more than one revision ("4 saves"); View › Revisions; palette `revisions.open`; the signature panel's "See what changed after signing" (G05).

**Layout.** Rail becomes the **Revisions rail panel** (260, back button to the rail tabs). Opening a revision adds a **revision notice strip** under the top bar (accent tint) and puts the canvas in read-only state: modes other than Read are disabled, the top bar context reads "Revision 2 of 4 · read only", the window title adds "— revision 2 (read only)". Inspector shows revision details.

**Content and controls.**

- Timeline, newest first: "Revision 4 · Current", "Revision 3 · Signed" (with "Signature valid" badge), "Revision 2 · Open", "Revision 1 · Original"; each with date, author/producer and a one-line summary of changes.
- Notice strip: "You are viewing revision 2, saved 20 Oct 2026, 17:30. It is read only; the file on disk is unchanged." with "Save this revision as…" (ghost), "Compare with current" (secondary, opens X04), **Back to current** (primary).
- Inspector: Saved by, Part of the file (byte range), Changed in this save (links that jump to the page), "14 objects changed · Show in object inspector" (O15), switch "Outline changed areas" (dashed accent outlines on the page).

**States.** One revision only: panel empty state "This file has been saved once, so there’s only one version." · damaged `/Prev` chain: notice "Revisions before 2 couldn’t be read; lamina repaired the file on open" · revision with unsupported change: summary "Changed: 3 objects" with no page links.

**Behaviour.** `revisions.open-revision` loads the prefix of the file read-only (`Document::open_revision`); edits are impossible, not just discouraged. "Save this revision as…" writes the bytes up to that revision's end as a new file (exact, not re-serialised). Escape or "Back to current" returns.

**Accessibility.** Timeline is an ordered list of buttons with `aria-current` on the open revision; the notice strip is a labelled region that receives focus when a revision opens. Changed-area outlines are paired with the list in the inspector.

**Platform differences.** Web identical (rail panel, no window title change).

**Artboards.** `X03-revisions.dc.html`.

**Todos.** [E14b](../../todos/ready/E14b-revision-browser.md).

## X04 Compare two versions

**Purpose.** Show what changed between two files or two revisions: text, drawings and images, comments and pages, side by side or overlaid with a pixel difference.

**Entry points.** File › Compare…; palette `compare.open`; X03 "Compare with current"; S04 file-changed banner "Compare"; drag a second PDF onto the Compare window.

**Layout.** A separate window (desktop) or tab (web) titled "Compare". Top bar: "Compare", two file buttons (Older / Newer, each opens a picker of files and revisions), swap button, a **view switcher** using the mode-switcher pattern (Side by side · Overlay), change navigation (previous, "1 of 9", next), "Export report…", More. Secondary toolbar: *Show* filters as toggles with counts (Text 5 · Images and drawings 1 · Comments 2 · Pages 1), "Scroll together" checkbox, alignment note. Rail: **Changes** list grouped by page (side by side) or **Pages** with change counts (overlay). Canvas: two pages, or one overlay.

**Content and controls.** Side by side: removed text struck through on a danger tint (older side), added text underlined on a positive tint (newer side), changed figures outlined; the current change carries the focus outline on both sides. Overlay: unchanged ink in muted grey; "Only in older" orange `#B4540F`, "Only in newer" blue `#2D4A6B` (blue/orange, distinct in lightness, plus legend); toggles "Both versions" / "Pixel difference", "Older version" opacity slider, "Ignore shifts under 1 px" select; numbered change badge on the page.

**States.** Choosing files (empty: two drop targets "Older" and "Newer") · comparing (progress per page) · no differences (empty state "These versions look the same" with "Text, drawings and comments match") · different page counts ("Page 12 is new" note; unmatched pages listed as Page added/removed) · web over `Limits` ("Comparing the first [N] pages") · report exported (toast).

**Behaviour.** Read-only on both files. Next/previous change: `compare.next-change` / `compare.previous-change`, shortcuts `⌥⌘↓` / `⌥⌘↑` on macOS (palette precedent) and none on Windows/Linux (avoid `Ctrl Alt`). Export report: PDF summary or JSON (`compare.export-report`), a new file.

**Accessibility.** Changes list is the full textual equivalent of the visual diff (each item says what and where, including old → new values); colour is never alone (strike/underline, legend words). Slider has a label and value text.

**Platform differences.** Web: opens in a new tab with the same layout; files must both be opened in the page (no folder access).

**Artboards.** `X04-compare.dc.html` (side by side), `X04-compare-overlay.dc.html` (overlay, pixel difference).

**Todos.** [E14c](../../todos/ready/E14c-compare.md).

## X05 Batch runner

**Purpose.** Apply a saved recipe of steps to many files and get a per-file result, without changing the originals.

**Entry points.** File › Batch…; palette `batch.open`; "Record steps from an open document" from a document's More menu; opening a `.lamina-recipe` bundle.

**Layout.** Its own window "Batch — <recipe>". Top bar: recipe name, "Recipe · saved 27 Oct 2026", Open recipe…, Save recipe, run control (Run on N files / Stop after current files), More. Left (300): **Steps** list. Main: **Files** with add buttons, progress and a results table. Right: step settings inspector.

**Content and controls.**

- Steps: numbered rows (title + what it applies to, from the recipe's selectors: "all pages", "pages labelled ‘draft notes’", "whole document"), "Add a step…", "Record steps from an open document".
- Files table (scrolls horizontally at narrow widths): File · Pages · Status badge · Result. Statuses: Waiting, Step n of 5, Done (positive), Done with warnings (warning; "Step 3: no page is labelled ‘draft notes’"), Skipped (warning; "Signed: saving would break the signature · Run anyway…"), Not processed (danger; "Needs a password · Enter password…").
- Output: copies next to each file with a suffix (step 5), never overwrite.
- Inspector: the selected step's parameters (locked while running), selector choices, bundled font/blob note, note that a non-matching selector is a warning.
- "Export run report (JSON)".

**States.** Empty queue (empty state "Add files or a folder to run ‘Board pack tidy-up’ on them") · ready (primary "Run on 12 files") · running (drawn) · finished ("12 files · 9 done · 2 warnings · 1 not processed", toast) · stopped · recipe from a newer lamina (error with the version).

**Behaviour.** Each file runs in its own transaction; a failure never touches other files or the original. Signed files are skipped by default. Concurrency is shown in the status bar ("2 files at a time").

**Accessibility.** Table with `scope="col"` headers; progress bar with values; status words plus icons. Reordering steps uses Move up/down in each step's More menu (drag optional).

**Platform differences.** Web: no folders; "Add files…" only, outputs downloaded as a zip; optional **server batch** (E14e) shows the server consent block listing every file and the total size before any upload, and the run happens in the Server jobs panel (X02). Desktop runs locally only.

**Artboards.** `X05-batch.dc.html`.

**Todos.** [E14d](../../todos/ready/E14d-batch-runner.md), [E14e](../../todos/ready/E14e-server-batch.md).

## X06 Accessibility checker

**Purpose.** Run the machine-checkable PDF/UA (Matterhorn) checks and font/Unicode checks, take the person to each issue and fix it where a fix is safe.

**Entry points.** Edit mode › Accessibility tool; palette `a11y.check`; Document properties (R08) "Tagged: yes · Check accessibility"; X08 failure "Fix in the checker".

**Layout.** Edit mode with the **Accessibility tool toolbar**: tool name, toggles Check · Tags · Reading order, "Document language…", "Save as PDF/UA…", issue count, Done. Rail (280): check results grouped by kind. Canvas: the page with the issue outlined and labelled. Inspector: the selected issue with its fix.

**Content and controls.**

- Results header: "Check results", "Check again", summary "PDF/UA-1 machine checks · 5 issues left · 2 fixed".
- Groups: Images, Text and fonts, Tables, … each issue as a link row with a warning icon, a plain title ("Figure has no alt text", "Characters can’t be read as text", "Header cells don’t say what they head") and its location.
- "Fixed in this session" group (positive check, "Undo with ⌘Z").
- Footer: "Passed: N machine checks" and "Need a person: …" with a link to the manual checklist.
- Inspector: title, "Page 3 · Matterhorn 13-004", one-sentence explanation of the effect on a screen-reader user, the fix fields (alt text textarea pre-filled from the chart's labels with "Check it before saving"), actions "Set alt text" (primary), "Mark as decorative", "Next issue".

**States.** Not run (empty state with "Check this document") · checking (progress in the rail header) · issues (drawn) · all machine checks pass (positive notice "No machine-detectable problems" plus the "Need a person" list — never "Accessible") · untagged file (single issue "This file has no tags" with "Start tagging" → X07).

**Behaviour.** One-click fixes (set language, display title, role-map repair) are ordinary undoable ops; fixes that need judgement (alt text, artifacts) always go through the inspector. Checks re-run on the affected page after each fix. Save is incremental; signed files show the §7.13 banner because tagging is a Document change.

**Accessibility.** Issues list is a `nav` of links; selecting one moves focus to the inspector's first field. Outline + text label on the page, never colour only.

**Platform differences.** Web identical.

**Artboards.** `X06-accessibility.dc.html`.

**Todos.** [E15d](../../todos/ready/E15d-accessibility-checks.md).

## X07 Tags and reading order

**Purpose.** See and edit the structure tree, the order a screen reader follows, alt text and language, and tag untagged content.

**Entry points.** Accessibility tool › Tags / Reading order; X06 "Start tagging"; right-click on page content › Tag as…; palette `tags.show`, `tags.tag-selection-as`.

**Layout.** Same tool toolbar with Tags and Reading order pressed, plus "Tag selection as" select and "Tag" button (disabled until content is selected). Rail: **Tags** tree (`role="tree"`). Canvas: reading-order overlay — each tagged block outlined (dashed accent) with a numbered badge in the margin; artifacts outlined in `control-border` and labelled "Artifact". Inspector: the selected element.

**Content and controls.** Tree rows: type (H1, P, Figure, Caption, Section, Document · language) and a text excerpt; numbered badges match the page. Inspector: Tag select, Alt text, Language ("Same as document (English, UK)"), Reading order "Move earlier" / "Move later" (disabled at the ends) with the keyboard and drag hint, "Remove tag, keep content" (danger ghost). Tag-as list: Paragraph, Heading 1–6, List, Table, Figure, Artifact (decorative); tables open a small header-scope step.

**States.** Untagged file: rail empty state "This file has no tags" with "Tag automatically" (heuristic, marks the result Approximate for review) and "Tag by hand". Selection with mixed tags: inspector shows "Several tags". Signed file banner as in X06.

**Behaviour.** `tags.move-earlier` / `tags.move-later` (`⌥↑` / `⌥↓` on macOS and `Alt Up` / `Alt Down` elsewhere, active only while the tree or page has focus); dragging a tree row is optional. Each change is one undoable `Document`/`Content` op; ParentTree is maintained invisibly.

**Accessibility.** APG tree pattern (arrow keys, `aria-level`, `aria-expanded`, `aria-selected`); the overlay badges are `aria-hidden` (the tree is the accessible equivalent). Move buttons are the non-drag alternative (WCAG 2.5.7).

**Platform differences.** Web identical.

**Artboards.** `X07-tags.dc.html`.

**Todos.** [E15b](../../todos/ready/E15b-structure-tree-editing.md), [E15c](../../todos/ready/E15c-tagging-tools.md).

## X08 Save as PDF/UA or PDF/A

**Purpose.** Save a copy that conforms to PDF/UA-1, PDF/A-2u/2b, or both, refusing (with a report and fixes) when checks fail.

**Entry points.** File › Save a copy to a standard…; Accessibility toolbar "Save as PDF/UA…"; palette `save.as-pdfua`, `save.as-pdfa`.

**Layout.** Dialog (560) over the dimmed shell.

**Content and controls.** Standard radio group (PDF/UA-1 · PDF/A-2u · PDF/A-2b · PDF/UA-1 and PDF/A-2u together) · **Check result** section: badge ("All machine checks pass" positive, or "Not saved" danger), what lamina will add (identifier in metadata, tagged marker, display title), warning note "Machines can’t check everything" with the manual checklist link · Save as filename with "A new file next to the original" · Cancel / **Save PDF/UA copy**. Failure: heading "3 things stop this file being PDF/A-2b", each problem in plain words with its location and an inline fix ("Embed font", "Remove from copy", "Add sRGB"), "Export the full report", blocked reason in danger text, primary "Fix all 3 and check again".

**States.** Checking (indeterminate bar in the result section; Save disabled) · pass (drawn) · fail (drawn) · saved (toast "Saved Q3-board-report-UA.pdf · Open" ) · signed source (note: the copy is a full rewrite and won't carry the signature; the original keeps it).

**Behaviour.** Fixes apply to the copy only. Conversion runs locally; no server. The open document is unchanged.

**Accessibility.** Result is a `role="status"`; problems are a list whose fix buttons name the action; the blocked reason is text, not only a disabled button.

**Platform differences.** Web: "Save as" becomes "Download"; file name field stays.

**Artboards.** `X08-save-conformance.dc.html`, `X08-save-conformance-failed.dc.html`.

**Todos.** [E15e](../../todos/ready/E15e-pdfua-output.md), [E16e](../../todos/ready/E16e-preflight-and-conversion.md).

## X09 Optimise

**Purpose.** Save a smaller or faster-to-open copy, with an estimate of the result before saving.

**Entry points.** File › Save an optimised copy…; palette `save.optimised`; Document properties size line "Reduce size…".

**Layout.** Dialog (560).

**Content and controls.** Before/after size bars ("Now 2.4 MB", "After ≈ 1.1 MB", one `role="img"` with a text label) · Preset toggles (Smallest · Balanced · Print quality; editing any option turns the preset to none) · options list, each with its estimated saving: Shrink images (ppi input, JPEG quality select, "11 images above 150 ppi. Masks and transparency are kept."), Keep only the characters each font uses, Remove unused and duplicate objects, Pack objects into compressed streams, Fast web view · signed-file warning note · Save as filename · footer "Sizes are estimates from a trial run", Cancel / **Save optimised copy**.

**States.** Estimating (values show "…" and the bars a skeleton) · ready (drawn) · nothing to gain ("This file is already compact" note, Save still allowed) · saved (toast with the real before/after) · web over `Limits` (desktop suggestion).

**Behaviour.** Always a full rewrite (`RequiresRewrite`), always a new file; the signed warning appears before saving, never after.

**Accessibility.** Each option's estimate is in the same row as its checkbox label; inputs labelled.

**Platform differences.** Web downloads the copy; desktop can embed system fonts, web only supplied ones (font option hint changes).

**Artboards.** `X09-optimise.dc.html`.

**Todos.** [E16b](../../todos/ready/E16b-structural-optimiser.md), [E16c](../../todos/ready/E16c-images-and-fonts.md).

## X10 Print production

**Purpose.** Prepare files for commercial print: preflight against a profile, output intent, separations and overprint preview, page boxes, colour conversion, and imposition.

**Entry points.** Edit mode › Print production tool; palette `print.preflight`, `print.impose`; File › Impose…

**Layout.** Edit mode with the **Print production tool toolbar**: Preflight, Separations, Page boxes toggles; "Overprint preview" switch; "Convert colours…", "Impose…"; Done. Rail (280): Preflight. Canvas: the page with trim marks and the problem marked. Inspector: Separations (inks with show/hide checkboxes and coverage at the pointer, total ink vs limit), Output intent, and the fix for the selected error. Imposition: a wide dialog (820) with options left and a live sheet preview right.

**Content and controls.** Preflight profile select (PDF/X-4 · PDF/X-1a · Office printer; rules are a data file, "Edit profile…"), Run, summary counts, results grouped Errors / Warnings with badges (word + colour) and locations. Imposition: Layout radio (2 per sheet, 4 per sheet, Booklet, Custom grid…), Sheet select, Pages range, Creep with help, Printer’s marks checkboxes, preview "sheet 1 of 4" with front/back and previous/next sheet, "16 pages · 4 sheets", Cancel / **Create booklet**.

**States.** Preflight not run · running · pass ("All 29 rules passed" positive) · issues (drawn). Separations off: inspector shows the selected issue only. Imposition with a page count not divisible by 4: note "2 blank pages will be added at the end".

**Behaviour.** Separations and overprint preview are view-only. "Set output intent…" is an incremental change; colour conversion and flattening are rewrites saved as a copy (signed warning as X09). Imposition always creates a new document.

**Accessibility.** Ink rows are labelled checkboxes with the ink name (swatch is decorative); preview sheets are `role="img"` with a description of which pages sit where; prev/next sheet buttons are labelled.

**Platform differences.** Web identical; large print files get the desktop suggestion when over `Limits`.

**Artboards.** `X10-preflight.dc.html` (`annual-review-2026.pdf` page 5 in Edit mode: Preflight and Separations pressed, overprint preview on; PDF/X-4 results "2 errors · 3 warnings · 24 rules passed"; the 180 ppi photo selected and marked on the page with an Error label and the total ink at the pointer; inspector with the fix (Replace image… / Accept for this file), five inks including the spot ink "Northwind Teal", total ink 318% over the 300% limit, output intent Coated FOGRA39; status bar shows the trim and bleed), `X10-imposition.dc.html` (wide dialog over the dimmed shell: Booklet chosen, A3 landscape, pages 1–16, binding left, creep 0.2 mm, crop marks; preview of sheet 1 front with pages 16 and 1, Front/Back toggle, previous/next sheet; Save as `annual-review-2026-booklet.pdf`; note that links and form fields aren't carried into the booklet).

**Todos.** [E16d](../../todos/ready/E16d-colour.md), [E16e](../../todos/ready/E16e-preflight-and-conversion.md), [E16f](../../todos/ready/E16f-imposition.md).

## X11 Layers and actions

**Purpose.** Show, hide and edit optional-content layers; inspect and author actions (go to, open web address, show/hide, reset, JavaScript as text) on buttons, links, fields, pages and file open.

**Entry points.** Layers: status-bar item "4 layers · 1 hidden in your view" (only on files with layers); View › Layers; palette `layers.show`. Actions: Edit mode › Actions tool; selecting a link, button or field in Edit mode (inspector Actions section); palette `actions.list`.

**Layout.** Layers: **Layers rail panel** (back to tabs) with eye toggles and a lock glyph; inspector for the selected layer in Edit mode. Actions: Edit toolbar with "Actions" pressed; rail lists every action in the document grouped by place (When the file opens, Buttons, Links, With JavaScript); the selected button outlined on the page; inspector (320) with the action list.

**Content and controls.** Layers rail: explanation "The eye shows or hides a layer for you only. It doesn’t change the file.", rows (eye button `aria-pressed`, name, "· hidden", locked icon with label), "Show all", "Reset to file’s view". Layer inspector: Name, *When someone opens the file*: Visible switch, Prints select, Lock checkbox; Order (Move up/Move down); New layer; Delete layer… (confirm dialog). Actions inspector: When select (Clicked, Pointer enters, Pointer leaves, Gets focus), ordered steps with More menu each, the JavaScript step with badge "Not run by lamina", a monospace textarea, and "Saved as text. lamina never runs it; other readers may, if their settings allow."; "Add an action…" with the list of supported kinds; "Remove all actions".

**States.** File without layers: no status item and no panel. Locked layer: eye disabled with "Locked by the author" tooltip text. File with JavaScript: status-bar note "This file contains JavaScript · lamina doesn’t run it".

**Behaviour.** Eye toggles are view state (not undoable, not saved); inspector edits are `UpdateOcg`/`SetOcgDefaults`/`SetActions` ops, undoable and incremental. JavaScript is never executed (ADR 0021).

**Accessibility.** Eye buttons are named "Hide <layer>"/"Show <layer>" with `aria-pressed`; step lists are ordered lists; reorder uses buttons.

**Platform differences.** Web identical (layers panel is a reader feature on both shells).

**Artboards.** `X11-layers.dc.html` (`mill-lane-A102.pdf` in Edit mode with the Layers rail panel: Dimensions locked, Electrical hidden, Furniture selected with its eye button focused; inspector for Furniture with Name, Visible switch, Prints select, Lock checkbox, Move up/Move down, Delete layer…; status-bar item "4 layers · 1 hidden in your view"), `X11-actions.dc.html` (`annual-review-2026.pdf` page 2 with the Actions tool: rail grouped by place, the "Read the full accounts" button selected on the page; inspector with When = Clicked and two ordered steps, Open web address and Run JavaScript with the "Not run by lamina" badge and the JavaScript textarea focused; status-bar note about JavaScript).

**Todos.** [E17c](../../todos/ready/E17c-layers.md), [E17b](../../todos/ready/E17b-action-editor.md).

## X12 Measurement and media

**Purpose.** Measure distance, perimeter and area at the drawing's scale and save the measurements as annotations; show embedded video/audio with a poster and play control, and 3D models as their saved picture with a notice.

**Entry points.** Measure: Comment mode › Measure tool (Tools menu, palette `measure.distance`, `measure.area`); appears as a context note "This page has a scale (1:100) · Measure" on pages with a viewport `/Measure`. Media: on the page; selecting it opens the inspector.

**Layout.** Measure: Comment mode with the **Measure tool toolbar** in the secondary-toolbar slot; the page centred, with measurements drawn over it; inspector (300) for the selected measurement; no rail. Media: Read mode; the page with posters in place of the media; inspector (300) for the selected media item.

**Content and controls.** Measure toolbar: Distance · Perimeter · Area (`measure.distance`, `measure.perimeter`, `measure.area`), Scale "1:100 · from the drawing", Calibrate…, Units select (m, cm, mm, ft and in), "Enter points…", hint "Click each corner, double-click to finish · Esc to cancel", Done. Canvas: measurements with value labels; the selected one has square handles. Inspector: Area, Perimeter, Sides, Scale (definition list), Label, Precision (0.1 · 0.01 · 1), "Edit points as numbers…", Copy value, Delete measurement. Status bar: pointer coordinates in drawing units (and latitude/longitude on geospatial PDFs). Media: video poster with a centred 64 px play button and a duration chip; 3D poster with an inline info note "3D model · shown as its saved picture. lamina can’t turn 3D models yet." and "Save model as…"; inspector: File, Size, Length, Format, Captions, Playback ("Only when you press play"), "Save video as…".

**States.** No scale in the file: toolbar shows "Scale: not set · Calibrate…" and values are in page units (pt) until calibrated by drawing a known length. Calibrate: small dialog "This line is [ ] m long". Video playing: native controls inside the frame, Escape stops focus trap. Unsupported codec on web: poster stays with "Can’t play this format here · Save video as…". 3D: never interactive in this round.

**Behaviour.** Measurements are annotations (Comment-mode rules for signed files); they are undoable and incremental. Media never autoplays and never fetches remote renditions without the consent rule (remote media: "This video is on example.org · Play from there?" names the host first).

**Accessibility.** "Enter points…" and "Edit points as numbers…" are the non-drag alternative for drawing measurements (WCAG 2.5.7); value labels on the page are mirrored in the inspector; the play button names the video and its length.

**Platform differences.** Web: media limited to browser codecs; measurement identical. Desktop plays more formats.

**Artboards.** `X12-measure.dc.html` (`mill-lane-A102.pdf` in Comment mode with the Measure toolbar, Area pressed, scale 1:100 from the drawing; the living-room area 21.6 m² selected with square handles and a 4.40 m distance across the kitchen; inspector with area, perimeter, sides, Label focused, Precision; pointer coordinates in the status bar), `X12-media.dc.html` (`annual-review-2026.pdf` page 6 in Read mode: video poster with the focused play button and 2:14 chip, 3D model as its saved picture with the info note; inspector with file, size, length, format, captions, playback policy and Save video as…).

**Todos.** [E17e](../../todos/ready/E17e-measurement.md), [E17d](../../todos/ready/E17d-media.md), [E17f](../../todos/ready/E17f-3d-detection.md).

## New components

### Server consent block

The required step before any upload (X01 server option, X02, X05 web server batch). A white section with a `control-border` edge inside a dialog, never a separate pop-up:

- Share icon in `accent` + heading "This conversion runs on lamina’s server" + one sentence on why the device can't do it alone and that nothing is sent until the person agrees.
- A definition list with four fixed rows: **What is sent** (file name, "the whole file" or page range, size), **Where to** (host name, region placeholder, "over HTTPS"), **How long it stays** (deleted on download or after 24 hours, whichever comes first; "You can delete it sooner"), **What it is used for** ("This conversion only").
- A checkbox whose label repeats the file name ("Upload Q3-board-report.pdf for this conversion"), unchecked by default, never remembered.
- The dialog's primary ("Upload and convert") stays disabled with a visible reason until it is ticked.

Afterwards the network indicator names the host ("7 network requests · to jobs.lamina.example only · 2 uploads you approved") and the Server jobs panel lists each upload with delete controls.

### Tool toolbar

A secondary toolbar for a tool that works on the open document inside a mode (Accessibility and Print production in Edit, Measure in Comment). Same metrics as §7.24; starts with the tool's 16 px icon and name in Newsreader 15/500, ends with a context note and a **Done** secondary button that returns to the mode's own toolbar. Only one toolbar is ever shown (§8.1).

### Rail panel

A rail view that replaces the tabs for one task (Revisions, Layers): header with a 28 px back button ("Back to sidebar tabs"), Newsreader 18 title, a 12 px `muted` count; may be 260–280 wide. Used where a fourth rail tab would not fit 220 px.

### Revision notice strip

A 40 px strip under the top bar in `accent-tint` (`#E1E6E3`) with a clock icon, a bold lead in `accent`, the read-only fact, and actions right. Used only while a past revision is open; the save banner (§7.13) uses the warning tint, so the two never look alike.

### Job panel (inspector slot)

A docked panel for long-running work on this document (OCR, server jobs): title + context line, progress (§7.22), a per-item list, review fields, actions pinned to the bottom. It never covers the page and can be closed while work continues (the status bar keeps a "Making searchable · 1 of 2 pages" or "2 server jobs" item that reopens it).

### Timeline

Ordered list with a 9 px dot and a 1 px `chip-border` connector per row; the open item has a filled `accent` dot and the selected-row treatment (`accent` border, `accent-tint`).

### Diff marks

Removed: `danger-tint` background + strikethrough; added: `positive-tint` + underline; overlay colours orange `#B4540F` (6.9:1 on white) and blue `#2D4A6B` with a legend; the current change gets the 2 px `focus` outline.

### Reading-order badge

22 px `accent` circle with white 12/600 number, placed in the page margin beside each tagged block, matching the number in the tag tree; artifacts get a neutral "Artifact" tag instead.

### Wide dialog with preview

Dialog up to 820 wide when a live preview is the point (imposition): options column (min 260) and a `gutter`-coloured preview column (min 400) that stack below 700 px.

### New icons

Drawn on the §5 grid, stroke 1.5:

| Name | Markup |
| --- | --- |
| Clock / revision | `<circle cx="12" cy="12" r="7.5"></circle><path d="M12 8v4l3 2"></path>` |
| Swap | `<path d="M5 9h13l-3-3M19 15H6l3 3"></path>` |
| Arrow up / down | `<path d="M12 19V5M7 10l5-5 5 5"></path>` / `<path d="M12 5v14M7 14l5 5 5-5"></path>` |
| Chevron right / left | `<path d="M10 7l5 5-5 5"></path>` / `<path d="M15 5l-7 7 7 7"></path>` (= Back) |
| Accessibility | `<circle cx="12" cy="5.5" r="1.5"></circle><path d="M5 9l7 1.5L19 9M12 10.5v4l-3 5.5M12 14.5l3 5.5"></path>` |
| Tag | `<path d="M4.5 4.5h7l8 8-7 7-8-8z"></path><circle cx="8.5" cy="8.5" r="1"></circle>` |
| Layers | `<path d="M12 4.5l8 4-8 4-8-4z"></path><path d="M4 12.5l8 4 8-4M4 16.5l8 4 8-4"></path>` |
| Eye / eye off | `<path d="M3 12c2.5-4 5.5-6 9-6s6.5 2 9 6c-2.5 4-5.5 6-9 6s-6.5-2-9-6z"></path><circle cx="12" cy="12" r="2.5"></circle>` (+ `<path d="M4 4l16 16"></path>`) |
| Lock | `<rect x="5.5" y="10.5" width="13" height="9" rx="1"></rect><path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5"></path>` |
| Ruler (measure) | `<path d="M3.5 15.5l12-12 5 5-12 12z"></path><path d="M7 12l2 2M10 9l2 2M13 6l2 2"></path>` |
| Printer | `<path d="M7 9V4.5h10V9"></path><rect x="4.5" y="9" width="15" height="7" rx="1"></rect><path d="M7 14h10v5.5H7z"></path>` |
| Server | `<rect x="4.5" y="5" width="15" height="6" rx="1"></rect><rect x="4.5" y="13" width="15" height="6" rx="1"></rect><path d="M8 8h.5M8 16h.5"></path>` |
| Download | `<path d="M12 4.5V15M8 11l4 4 4-4"></path><path d="M5 19.5h14"></path>` |
| Info | `<circle cx="12" cy="12" r="8"></circle><path d="M12 11v5M12 8v.5"></path>` |
| Action (bolt) | `<path d="M13 3.5L6 13h5l-1 7.5L17 11h-5z"></path>` |
| Play | `<path d="M8 5.5v13l10-6.5z"></path>` |
| 3D cube | `<path d="M12 3.5l7.5 4v9L12 20.5l-7.5-4v-9z"></path><path d="M4.5 7.5L12 11.5l7.5-4M12 11.5v9"></path>` |

## Proposed design-language additions

These are the components above that other areas could reuse; each is drawn in this area's artboards and should move into the [design language](design-language.md) when it is next revised:

- **Tool toolbar** (§7.24 variant): X06, X10, X12. Name the slot rule in §8.1: one secondary toolbar at a time, the tool toolbar replaces the mode's own.
- **Rail panel**: X03, X11. Proposed as the standard way to give the rail a fourth task without a fourth tab.
- **Severity badge pair**: "Error" (`danger` on `danger-tint`) and "Warning" (`warning` `#8A5300` on `warning-tint` `#F0E8DC`), 18 px, 11/600, always a word and never colour alone (X06, X08, X10, X11's "Not run by lamina").
- **Wide dialog with preview** (X10 imposition) as a size step of the dialog (§7.12): 560 default, 820 with preview.
- **Media poster**: a poster block with a centred 64 px white play button (named with the title and length), a dark duration chip, and an inline info note for content lamina can't play (X12).
- **Measurement marks**: `accent` fill at 12% with a 2 px `accent` edge for the selected shape, 8 px square white handles, value labels as filled `accent` chips; other measurements in `warning` brown with a white chip (X12).
- **Monospace text** for code-as-text fields (JavaScript in X11): the system stack `ui-monospace, 'SF Mono', Menlo, Consolas, monospace` at 12/1.5; the design language has no monospace token yet.
- **Icons** in [New icons](#new-icons), plus three tool icons drawn for X12 on the same grid: Distance `<path d="M4 12h16M4 9v6M20 9v6"></path>`, Perimeter `<path d="M5 6l12-1 2 12-13 2z"></path>`, Area (Perimeter + `<path d="M9 10l6 6M8 14l3 3M12 8l5 5"></path>`).
