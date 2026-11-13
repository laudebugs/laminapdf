# Paper: Edit session and organise

Screens O01–O08 of the [screen inventory](screens.md): the edit session and its history, saving (Save, Save as, Save a copy or Download, Save as optimised) with the signed-file constraint banner, closing with unsaved changes, the editable document properties and XMP editor, the in-app capability page, Organise mode with its page grid, inserting pages and merging documents, and splitting a document. Everything follows the [Paper design language](design-language.md), including its [decisions on area proposals](design-language.md#decisions-on-area-proposals) and [command-id appendix](design-language.md#appendix-command-ids); its proposals are merged there. O09–O17 are designed separately in [Pages and navigation](pages-navigation-extraction.md).

- **Canvas:** [lamina Paper — Edit session and organise](https://claude.ai/artifact/R2ZvF977LNg99a85zQiEeH) (private until the owner shares it).
- **Plan:** [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) and [§E2](../../plans/full-editor.md#e2-page-organisation); save behaviour from [ADR 0007](../../decisions/0007-save-planner.md); undo from [ADR 0006](../../decisions/0006-op-log-and-undo.md); layout rules from the [UI/UX plan](../../plans/ui-ux.md) §4–§5.
- **Sample file:** `Q3-board-report.pdf` (Northwind Housing Trust, 24 pages). For the signed states it carries either an **approval** signature by “M. Adeyemi, Chair” on page 24 (field `Chair_Approval`, 28 Oct 2026, 11:40) or a **certification** (DocMDP P2, form filling only) by “Northwind Housing Trust” (field `Secretary_Signature`, 27 Oct 2026, 09:12). Other files (`Site-survey-Oct-2026.pdf`, `Rent-arrears-appendix.pdf`) and the custom XMP schema `nwt:` (`http://ns.example.org/northwind/report/1.0/`) are fictional.

Every board has the `accent` tweak (Teal, Blue, Plum, Green; design language [§2.8](design-language.md#28-accent-colours)); every accent-derived colour, link and focus ring follows it.

Where this area lives in the shell:

- **Edit session** state is always in the top bar, which is the design language's Editor variant (§7.1) on every board: sidebar toggle · file name + save state (`aria-live="polite"`) · divider · Undo · Redo (always shown, disabled with nothing to undo or redo) · mode switcher · page n/N (Organise: thumbnail size) · zoom · Find · View · Commands · Save split button (primary, only while there are changes) · More. History is a contextual tab in the left rail (§8.10), present in every mode once an edit session exists.
- **Organise** is a mode in the mode switcher. Its rail stays open with Outline · History and no Pages tab (the canvas *is* the page grid, design language §8.10); the sidebar toggle still hides it. It swaps in the Organise toolbar of design language §8.1 and opens the inspector for selected pages. History is reached in Organise through the rail's History tab, like in every other mode.
- **Document properties** (`file.properties`) is one dialog: read-only in Read mode (R08), editable in the editor modes (O04, design language decision 19). File › Document properties… on desktop, More › Document properties… on the web. The R08 facts are its **File** tab.

**Desktop menu additions for the editor** ([D4b](../../todos/ready/D4b-native-menus.md), [E1d](../../todos/done/E1d-edit-session-undo-ui.md)), using the design language's labels: File › Save `⌘S`, Save as… `⇧⌘S`, Save a copy… (`file.save-copy`), Save as optimised…, Document properties… (`file.properties`, also in [Desktop shell S03](desktop-shell.md#s03-native-menus)'s File menu), Merge documents…, Split document…; Edit › Undo `⌘Z` and Redo `⇧⌘Z` / `Ctrl Y` with the step in the label (“Undo rotate page 16”), Show history; Help › What lamina can do (`help.capabilities`).

Command ids use the registry’s `namespace.verb-object` form ([U1b](../../todos/done/U1b-command-registry-and-palette.md), design language §8.8) and match the [command-id appendix](design-language.md#appendix-command-ids): `pages.*` for every page operation in every mode, `document.*` for merge, split and metadata, `file.*` for saving. Every command is in the palette; only those listed with a shortcut have one. Shortcuts are shown as macOS / Windows and Linux.

**Signed files, one rule for the whole area** (ADR 0007 §2–§3, design language §7.13):

- **Approval signatures only:** every edit here is `AppendOnly`. It is applied at once, saved as an incremental update, and the signatures stay valid. Moving, inserting and duplicating pages, and rotating other pages, are allowed. Deleting or rotating the page that holds a signed signature's widget is a conflict (`Restriction::SignedWidget`, from [E2r](../../todos/ongoing/E2r-planner-page-checks.md)'s checks; a rule stricter than the standard, which the [E2 plan §6](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint) recommends) and asks first (O06).
- **Certification that doesn't allow the change (`Conflict`):** the change is refused before it is applied (`would_violate`). The constraint banner appears and says nothing changed (“**Deleting pages would invalidate the certification.** Northwind Housing Trust certified this file for form filling only, so nothing has been deleted.”) with Save a copy · Apply anyway… · Details, as drawn for cropping in [O09](pages-navigation-extraction.md). Apply anyway… opens the alertdialog that names the signature and confirms with a danger-filled button naming the result (“Delete page 7 and invalidate certification”). After that the change is applied and undoable, and the banner switches to “**Saving will invalidate the certification.**” until the change is undone or saved (drawn in `O02-save-conflict`).

## O01 Edit session and history

**Purpose.** Show that the document has unsaved changes, let people step back and forward through them, and list every change so they can return to any point.

**Entry points.** Any edit starts the session. Undo and Redo buttons in the top bar; Edit menu (desktop native menu, web More menu); palette; `⌘Z` / `Ctrl Z` and `⇧⌘Z` / `Ctrl Y` (also `Ctrl Shift Z`). The History tab in the left rail; palette “Show history” (`edit.history`).

**Layout.** Desktop window, Comment mode. Window title “Q3-board-report.pdf — Edited” and a dot in the close button (macOS convention). Top bar (Editor variant): sidebar toggle · file name · “4 unsaved changes” · divider · Undo · Redo · mode switcher · page, zoom, Find, View, Commands · Save split button · More. Comment toolbar. Left rail, **History** tab (Pages · Outline · History). Canvas page 3. Status bar unchanged. A toast sits above the status bar after an undo.

**Content and controls.**

| Control | Does | Command id |
| --- | --- | --- |
| Save state | “No unsaved changes”, “1 unsaved change”, “4 unsaved changes”, “Saving…”, “Saved”. Counts transactions since the last save (semantic dirty, ADR 0006 §7). `aria-live="polite"` | — |
| Undo | Undoes the last transaction. Its accessible name and tooltip name the step: “Undo note added” `⌘Z` | `edit.undo` |
| Redo | Redoes the next undone transaction: “Redo rotate page 5” | `edit.redo` |
| History tab | Heading “History” with a step count, then an ordered list: “Opened file · No changes · 16:02” first, one row per transaction (label, page, time). The current point is the tinted row (`aria-current="step"`); undone steps are struck through and say “Undone”. Choosing a row undoes or redoes to that point in one action. The list scrolls inside the rail | `edit.history`, `edit.go-to-step` |
| Footer note | “Choose a step to go back to it. History is kept after Save and cleared by Save as optimised.” | — |
| Toast | “Undid “Rotate page 5”” with a Redo action (4 s, design language §7.20) | — |

**States.**

- **Clean:** “No unsaved changes”, Undo and Redo disabled (`disabled` text colour), no Save button, history shows only “Opened file”.
- **Dirty:** as drawn.
- **Undo partial or skipped** (`UndoOutcome::Partial` / `Skipped`, ADR 0006 §4): the toast says what happened, with the usual 4 s timing (§7.20): “Undid part of “Merge documents”: 2 bookmarks had been changed since, so they stay”. Because a toast must never be the only place, the History row keeps a `warning` “Partly undone” caption, and the History tab shows a `warning-tint` notice at its top with the same sentence until the next edit.
- **Depth limit:** after 500 transactions the oldest drop; the first row reads “Earlier steps no longer kept”.
- **After a save:** an incremental save that freed nothing keeps the history and adds a “Saved · 16:20” divider row. A save that freed objects (an incremental save after deleting pages, for example) clears it to a single “Saved · 16:20 · earlier steps cleared” row; Save as optimised and any other full rewrite clear it to “Saved as optimised · 16:20” (ADR 0007 §8).
- **Blocked:** while a save is running or the document is `SavedNotRebased`, Undo, Redo and every edit are disabled and the state reads “Saving…”.

**Behaviour.** Undo and redo apply per document, local changes only. Only touched pages re-render. Typing in one field or note coalesces into one step; a drag or batch is one step. The window title, the tab (desktop tabs, browser tab title “• Q3-board-report.pdf”) and the status of every close action follow the dirty state.

**Accessibility.** Undo/Redo are real buttons with names that include the step; outcomes are announced through the toast’s `role="status"` live region. The history list is an `<ol>` of buttons; Up/Down move, Enter goes to the step. `aria-keyshortcuts` on Undo and Redo. Single-key shortcuts are not used here.

**Naming.** “History” here is the edit-history rail tab (§8.10, `edit.history`); the reader's Back/Forward group is also called History in §7.1. The review suggested renaming one of them; the design language kept both names, so this area keeps “History” for the tab (left for the owner).

**Platform differences.** Web: no window title; the browser tab title gets a leading “•”; Redo is `Ctrl Y` on Windows/Linux and `⇧⌘Z` on macOS (tested in Chromium, Firefox and WebKit). Desktop Windows/Linux: Edit menu in the window menu bar, “Edited” shown as “•” in the title. Phone/tablet: Undo and Redo move into the More sheet; history is a sheet.

**As built (web, E1d).** Until saving arrives (E1e), the save state counts the steps since the file opened and closing asks with Cancel and Close without saving only (O03 without Save). On the web, Undo, Redo and Show history sit at the top of the More menu, with Rotate page right and left, the reader's one edit until Organise. Choosing a History row that moves one step shows the same toast as Undo or Redo. Undo and Redo have no hover tooltip yet; their accessible names carry the step and `aria-keyshortcuts` the keys.

**Artboards:** `O01-edit-session.dc.html`.

**Todos:** [E1d](../../todos/done/E1d-edit-session-undo-ui.md).

## O02 Save, Save as and Download a copy

**Purpose.** Save only when asked, and say what a save will do to signatures before anything is written (principle 6; ADR 0007).

**Entry points.** The Save button (primary, top bar, only with changes) and its chevron “Save options”; File menu; palette; `⌘S` / `Ctrl S` (Save), `⇧⌘S` / `Ctrl Shift S` (Save as, desktop only). The constraint banner’s own buttons.

**Layout.** Desktop window. The Save split button opens a menu under the top bar (menu layer, z 30). The banner is a strip under the secondary toolbar. Confirmations are dialogs over the scrim.

**Content and controls.**

| Control | Does | Command id |
| --- | --- | --- |
| Save | Saves in place. Incremental by default (`SaveMode::Auto`): keeps every original byte and appends a revision. Menu text says what it will do for this file: “Adds your 2 changes after the signature, as a new revision. The signature stays valid.” | `file.save` (`⌘S` / `Ctrl S`) |
| Save as… | Saves to a new file the same way and switches the window to it; the original is unchanged | `file.save-as` (`⇧⌘S` / `Ctrl Shift S`, desktop) |
| Save a copy… (desktop) / Download a copy (web) | Writes the document as it is now, with the unsaved changes, to a new file and keeps working on the current one; no shortcut. Planned like any save: under a certification `Conflict` it opens the same remove-signature confirm, worded for the copy (“Save a copy without the certification?”, button “Remove signature and save copy”), and the original on disk stays certified | `file.save-copy` |
| Save as optimised… | Full rewrite: drops earlier revisions and unreferenced objects. Disabled with a reason on `AppendOnly` files (“Not available for this file: a full rewrite would invalidate M. Adeyemi’s signature.”) | `file.save-optimised` |
| How saving works | Two-sentence explanation inside the menu: incremental vs full rewrite | — |
| Constraint banner | Before the change: refuses it and offers Save a copy · Apply anyway… · Details (see the area's signed-file rule above). After Apply anyway (drawn): “**Saving will invalidate the certification.** You deleted page 7 with Apply anyway; Northwind Housing Trust certified this file for form filling only. Saving removes the certification, and lamina asks first. Save a copy leaves the certified original on disk as it is.” Buttons “Save a copy…” and “Details” | `file.save-copy`, `file.show-constraint-details` |
| Details | Opens a popover (§7.28) anchored to the button: each signature with signer, kind, field and date, the DocMDP level in words (“Form filling and signing only”), and the changes in this session that it doesn't allow, each with “Show” (selects the page or object) | `file.show-constraint-details` |
| Remove-signature confirm | `alertdialog` (560 px) “Remove the certification and save?” naming every signature that the save removes (signer, kind badge, field, date, current validity), what happens to the field, and the alternative. Buttons Cancel (default focus), Save a copy… (opens the same confirm worded for the copy), “Remove signature and save” (danger, filled) | `file.save-invalidating-signatures` |
| File changed on disk | With unsaved changes, the `warning-tint` strip of Desktop shell S04 appears when the change is detected (Compare · Keep mine · Reload from disk); the next Save, with or without Keep mine, opens the 440 px `alertdialog` “This file changed on disk” with three explained choices: Save as… (primary), Reload (discards changes), Overwrite (danger ghost) | `file.compare-with-disk`, `file.keep-mine`, `file.reload-from-disk`, `file.save-as`, `file.overwrite` |
| Web Download | Without the File System Access API, the primary button is “Download” with a download icon; the menu offers Download (`Ctrl S`) and Download optimised and explains why. Download optimised follows Save as optimised: disabled with the same reason on `AppendOnly` files, and asks first on a `Conflict`. Download a copy stays in the More menu | `file.download`, `file.download-optimised` |

**States, one per `SaveConstraint.kind()`.**

- **Free:** Save is incremental; Save as optimised is enabled; no status note.
- **AppendOnly** (signed, no conflict; drawn in `O02-save`): status-bar note “Signed file. Saving adds your changes after the signature; the signature stays valid.”; Save as optimised disabled with the named signature.
- **RequiresRewrite** (applied redaction, encryption change, repaired base): the Save item says “Rewrites the whole file. Earlier versions are dropped, so removed content is gone.”; plain Save does a full rewrite (no separate choice). A repaired base adds “lamina repaired this file when opening it, so it can’t add to the old structure.”
- **Conflict** (drawn in `O02-save-conflict`, after Apply anyway): the offending edit was refused until Apply anyway… confirmed it (see the signed-file rule above); from then the banner says saving will invalidate the certification. Save opens the remove-signature confirm; Save a copy opens the same confirm for the copy and leaves the original on disk certified. The banner disappears when the edit is undone.
- **Blocked by the redaction report** (after Apply redactions, [G10](sign-protect-redact.md#g10-apply-redactions-and-verification-report), design language §8.5): while any check fails, Save and the Save options items are `aria-disabled` with `aria-describedby` on the reason (“Saving is blocked: 1 leak found.”); the menu shows the reason in place of the item descriptions and offers “Show report”. Save a copy is blocked the same way.
- **Saving:** state “Saving… 62%” with the determinate bar (design language §7.22) under the top bar for files over a few MB; edits blocked.
- **Saved:** “Saved” for 2 s, then “No unsaved changes”; the history keeps or clears as in O01.
- **Errors** (disk full, permission denied, File System Access permission revoked): a `danger-tint` notice under the top bar, “Couldn’t save Q3-board-report.pdf: the disk is full. Your changes are still here.” with “Save as…” and “Try again”. The document stays dirty and the file is untouched.
- **Conformance warnings** (E1c): before saving a PDF/A file whose edit breaks the claim, a dialog “Save without the PDF/A claim?” lists each warning; when save-time metadata was left alone to keep a certification valid, the status bar says “Modified date not updated, to keep the certification valid”.
- **External change while clean** (desktop): the file reloads, keeping the page position, and a toast says “Reloaded: Q3-board-report.pdf changed on disk” (design language §8.4).
- **External change while dirty** (desktop, drawn in `O02-save-external-change`): the strip and, at Save, the dialog above. **Overwrite** can't append to the changed file, so it rewrites the whole file: lamina writes the bytes it kept from opening plus the session as a new incremental revision to a temporary file and renames it over the changed file. Signatures in the version you opened therefore stay valid; the other app's version is lost. If the session is a `Conflict`, the remove-signature confirm comes first. **Keep mine** only dismisses the strip; it writes nothing. **Reload** discards the session and opens the newer file.

**Behaviour.** Planning happens before writing: the menu and banner read the planner’s constraint, never guess. Every save path, including Save a copy, Download and Overwrite, goes through the same planner. Desktop writes incrementally in place by appending (`IncrementalOutput::AppendedOnly`), and full rewrites to a temp file then rename; the file watcher ignores lamina’s own writes. The override sets `invalidate_signatures: true` only after the confirm; the saved file has those signature values removed and the fields kept, unsigned (ADR 0007 §3). Save a copy never changes the open file’s dirty state.

**Accessibility.** The split button is two buttons (“Save”, “Save options” with `aria-haspopup="menu"`, `aria-expanded`); the menu is `role="menu"` with `menuitem`s whose descriptions are part of their content; the disabled item uses `aria-disabled` plus `aria-describedby` on its reason, so it stays focusable and readable. The banner is a labelled `region` and its lead words are announced once through a polite live region when it appears. Destructive confirms are `alertdialog` with Cancel focused first.

**Platform differences.** Web with File System Access (Chromium): Save writes in place after the browser’s permission prompt; Save as uses the save picker. Web without it (Firefox, WebKit; drawn in `O02-save-web`): Save becomes Download, the document rebases onto the downloaded bytes, Save as and the external-change dialog don’t exist, and `Ctrl Shift S` is not bound (Firefox uses it for screenshots). Windows/Linux desktop: same menu with `Ctrl` chips; the window close button shows no dot. Phone: Save is in the top bar overflow; the banner sits under the top bar and wraps to two lines; the confirm becomes a sheet.

**Artboards:** `O02-save.dc.html` (AppendOnly menu, Save options open), `O02-save-conflict.dc.html` (certified file after Apply anyway: banner + remove-signature confirm, in Organise), `O02-save-external-change.dc.html` (strip + dialog at Save, Comment mode), `O02-save-web.dc.html` (Download menu).

**Todos:** [E1e](../../todos/done/E1e-save-and-constraint-banner.md).

## O03 Close with unsaved changes

**Purpose.** Never lose edits when a window, tab or the app closes.

**Entry points.** Window close button, `⌘W` / `Ctrl W` (desktop), Quit (`⌘Q`), closing a document tab; on the web, closing the browser tab triggers the browser’s own `beforeunload` prompt (its text can’t be customised).

**Layout.** `alertdialog` over the dimmed window, 440 px, no close icon (it asks a required question).

**Content and controls.** Title “Save changes to Q3-board-report.pdf?”; body “You have 4 unsaved changes. If you don’t save, they’re lost.”; for signed files the same signed-file line as the status bar. Buttons: “Don’t save” (secondary, left), Cancel, Save (primary, default focus). Commands `file.save`, `file.discard-changes`.

**States.**

- **Signed, AppendOnly:** the signed-file line (drawn).
- **Conflict:** Save is renamed “Save a copy…”, and the body adds “Saving this file would invalidate Northwind Housing Trust’s certification.” Save a copy… opens the remove-signature confirm worded for the copy (O02); the original on disk stays certified and the window closes after the copy is written. Saving the file itself with the override is still possible from the banner, never from this dialog.
- **Blocked by the redaction report:** Save is `aria-disabled` with the reason under the body (“Saving is blocked: 1 leak found. Show report”); the choices are Don't save, Cancel and Show report (primary), which closes this dialog and opens the [G10](sign-protect-redact.md#g10-apply-redactions-and-verification-report) report.
- **Several dirty documents on Quit:** “2 documents have unsaved changes” with a list (file name + count + per-row Save checkbox, all checked), buttons “Don’t save any”, Cancel, “Save 2 documents”.
- **Save fails from this dialog:** the dialog stays, with the O02 error notice inside it; the window doesn’t close.

**Behaviour.** Save runs the normal save (O02), then closes. Don’t save discards the overlay and closes. Escape is Cancel. Clean documents close without asking.

**Accessibility.** `role="alertdialog"`, labelled and described; focus starts on Save and is trapped; `⌘⌫` / `Alt N` activates Don’t save (macOS and Windows conventions), shown in the button’s tooltip only.

**Platform differences.** Web: only the browser prompt on tab close; in-app “Close document” uses this dialog. Windows/Linux: button order Save, Don’t save, Cancel (platform convention), drawn left to right in that order. Phone: a sheet at the large detent with full-width buttons.

**Artboards:** `O03-close-unsaved.dc.html`.

**Todos:** [E2i](../../todos/ready/E2i-signed-gating-and-close.md) (close guards themselves come from [E1d](../../todos/done/E1d-edit-session-undo-ui.md)).

## O04 Document info and XMP editor

The screen ID keeps the inventory's name (“Document info”); the dialog and command are “Document properties…” (`file.properties`, design language decision 19).

**Purpose.** Edit title, author, subject, keywords, dates, producer and creator (Info and XMP kept in sync), remove selected metadata, and edit any XMP property including custom and PDF/A extension schemas.

**Entry points.** File › Document properties… (desktop), More › Document properties… (web), palette “Document properties” (`file.properties`); clicking the file name subtitle in the top bar. No shortcut (`⌥⌘I` and `Ctrl Shift I` open developer tools). In Read mode the same command opens the read-only R08 view with “Edit in Comment, Organise or Edit” (§8.11); the boards are drawn in Comment mode.

**Layout.** Dialog over the dimmed shell. Description tab: 560 px. XMP tab: the dialog widens to the 820 px wide dialog (§7.18, two-pane editor). Header: title “Document properties” over “Q3-board-report.pdf · PDF 1.7 · PDF/A-2b”, close icon. Tabs: Description · XMP · File (the read-only facts from R08: version, page size, fonts, security, tagged). Footer: left “Remove metadata…” (danger ghost), right Cancel and Apply.

**Content and controls (Description).**

| Control | Does | Command id |
| --- | --- | --- |
| Title, Author, Producer | Text inputs | `document.set-metadata` |
| Subject | Textarea | same |
| Keywords | Chip input: each keyword a 26 px chip with a 24 px remove button (“Remove keyword repairs”), so the target meets the 24 px desktop minimum; Enter or comma adds | same |
| Created | Date-time input, `14 Oct 2026, 10:20` format, locale-aware | same |
| Modified | Read-only date plus checkbox “Set to the time I save” (on by default; the shell passes the clock at save) | — |
| Creator (pending removal) | Struck-through value, “Will be removed” danger badge, “Keep” to cancel | `document.remove-metadata` |
| Notices | Warning: “Removed values stay in earlier versions…” whenever a removal will be saved incrementally. Info: “PDF/A-2b file…” when the file claims PDF/A. A `ConformanceWarning` that would break the claim shows as a warning notice under the field that causes it | — |
| Remove metadata… | Opens a checklist of every Info/XMP field with a value; checked ones become pending removals | `document.remove-metadata` |

**Content and controls (XMP).** Toolbar: Properties / Raw XML toggle, filter input, “Add property…”. Left: a tree of schemas (Dublin Core, PDF, XMP basic, Media management, PDF/A identification, then custom schemas) and properties with their values. Info-mapped properties show “Description tab” and open there. `pdfaid:` is read-only (“Kept by lamina”). Right: the selected property: name, schema and namespace URI, Value, Type (Text, Date, Integer, Boolean, URI, Seq, Bag, Alt), and for PDF/A files “Description in the extension schema”, with the info notice that lamina adds the PDF/A extension schema declaration. “Remove property” (danger ghost). Footer counts pending changes (“1 property added”). Commands: `xmp.set-property`, `xmp.remove-property`, `xmp.replace-packet`.

**States.**

- **No XMP packet:** the XMP tab says “This file has no XMP metadata yet. lamina creates it when you apply a change.”
- **Raw XML:** a monospace textarea of the packet; invalid XML shows an error notice with line and column (“Line 14: the rdf:Description element isn’t closed”), the field gets `aria-invalid`, and Apply is disabled until it parses.
- **Signed file:** a note under the header repeats the save effect (metadata edits are allowed only as incremental updates; certified P1 files show the banner text instead and Apply opens the O02 confirm).
- **PDF 2.0 file:** Info fields that 2.0 deprecates are shown read-only with “PDF 2.0 keeps this in XMP only”.

**Behaviour.** Apply turns the form into one transaction (one undo step, “Document properties changed”); nothing touches the file until Save. Cancel or Escape discards. Changes to Info-mapped XMP properties update the Description tab and vice versa.

**Accessibility.** Tabs follow the tablist pattern; every input has a visible `<label>`; chips are buttons in a group; the tree is an ARIA tree (`treeitem`, `aria-level`, `aria-expanded`, roving tabindex, Left/Right collapse and expand); errors use `aria-describedby`.

**Platform differences.** Web identical (opened from More). Phone: the dialog becomes a full-height sheet; XMP shows the tree, and selecting a property pushes the detail view with a Back button.

**As built (E1f).** The web reader's dialog has the Description tab and a File tab (R08's read-only facts, with Copy all); the XMP tab waits for [E3e](../../todos/ready/E3e-xmp-editor.md). Differences from the boards:

- **Modified** is read-only, with "Saving sets this to the time you save" and no checkbox: every save stamps the modification date ([E1e](../../todos/done/E1e-save-and-constraint-banner.md)), so the checkbox could only ever be on.
- **Keyword chips** have 26 px remove buttons, the smallest icon button; they still meet the 24 px minimum.
- **The two conformance warnings** (more than one author in PDF/A-1, no title in PDF/UA-1) appear under the field before Apply. The dialog applies the save planner's two rules itself, since the SDK reports conformance warnings only from a save plan; `apps/web/e2e/properties-warnings.e2e.test.ts` holds them to the SDK's. In a PDF/A file whose Info and XMP disagree, Apply writes the value shown to both, and the PDF/A notice names those fields.
- **XMP metadata that can't be read** gets a warning notice: Apply replaces it.
- **The dialog opens from** More › Document properties… and the palette; the top bar's file name doesn't open it yet.

**Artboards:** `O04-document-info.dc.html`, `O04-xmp-editor.dc.html`.

**Todos:** [E1f](../../todos/done/E1f-metadata-ui-capability-page.md), [E3e](../../todos/ready/E3e-xmp-editor.md).

## O05 Capability page

**Purpose.** Show, for every operation, what lamina reads, writes and round-trips in each part (core, library, web, desktop), proved by tests (principle 7).

**Entry points.** Help › What lamina can do (desktop), More › What lamina can do (web), palette (`help.capabilities`), links from the docs site and from any “Unsupported” notice in the app.

**Layout.** Web app page (no mode switcher): top bar with Back (“Back to Q3-board-report.pdf”) and the page title, Commands, More. Content column max 1080 px on `ground`: Display heading “What lamina can read, write and round-trip”, one-paragraph lede, a status legend, filters, the table, a provenance line.

**Content and controls.**

| Control | Does |
| --- | --- |
| Legend | Four badges with definitions from [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix): Round-trip (positive, with check), Write (accent), Read (neutral), Unsupported (dashed neutral) |
| Search | Matches operation names and IDs (“merge”, “C.06”) |
| Group | Select: All groups, then each group A–M |
| Show | Toggle buttons Core · Library · Web · Desktop; hide columns |
| Table | One section per group (row-group header in Newsreader), rows: ID · operation · four status badges · target round. Caption “15 of 15 operations in 2 groups” updates with filters |
| Provenance | “Built from the capability manifest when this version of lamina was made · [VERSION] · How statuses are tested” |

The statuses drawn illustrate the state after rounds E1–E2 (so J.04 Manage XMP, round E3, shows Read only); they are not a release claim. The Group select uses the same group names as the table (“C · Page organisation and geometry”).

**States.** Filtered (caption updates, empty groups hidden). No matches: empty state “No operations match “stamp”” with “Clear filters”. Narrow widths: the table scrolls inside its box; filters wrap.

**Behaviour.** The data is bundled at build time (`cargo xtask capabilities --json`); the page makes no request. Filters persist per viewer in local storage.

**Accessibility.** A real `<table>` with `<caption>`, `scope="col"`, `scope="row"` and `scope="rowgroup"`; badges carry words, not colour alone; the filter area is `role="search"`.

**Platform differences.** Desktop: opens as a window tab with the same content. Phone: the table becomes one card per operation (ID and name, then four label–badge pairs).

**As built (E1f).** The page lives at `/capabilities` in the web reader, over the open document. More › What Lamina PDF can do (`help.capabilities`) pushes the address, and Back (the page's or the browser's) returns to the document as it was. A link from the docs site opens it directly, and Back then goes to the reader. The data is `docs/capabilities/matrix.json`, which `cargo xtask capabilities` writes beside the README and the reader bundles. Differences from the board:

- **The bar** holds Back alone, without the title, Commands and More: the page covers the reader, and Back is the way out of it.
- **On a phone** the table scrolls inside its box rather than becoming one card per operation.
- **"How statuses are tested"** opens the matrix's README on GitHub, which explains each status's evidence.
- **"Unsupported" notices** don't link here yet; no notice in the reader says an operation is unsupported so far.

**Artboards:** `O05-capabilities.dc.html`.

**Todos:** [E1f](../../todos/done/E1f-metadata-ui-capability-page.md).

## O06 Organise mode

**Purpose.** Reorder, rotate, insert, duplicate, extract, replace and delete pages in a grid, with a keyboard and a non-drag path for every move (WCAG 2.5.7) and one undo step per batch.

**Entry points.** Organise in the mode switcher; palette “Organise mode” (`mode.organise`); dropping a PDF or image on a desktop window in any mode switches here with an insertion bar at the drop point.

**Layout.** Desktop window. Top bar with Organise selected; the page-number input is replaced by a thumbnail size control (− 100% +). Organise toolbar (§8.1, drawn on every Organise board): Insert ▾ · Rotate left · Rotate right · Extract · Delete │ Move to page… │ Page setup ▾ · Bookmark ▾ · More page actions (⋯) · context note (“2 pages selected”, “24 pages · none selected”). Rail (220 px) with Outline · History (no Pages tab: the grid is the pages); the outline is editable here ([O11](pages-navigation-extraction.md)). Canvas: the page grid on `gutter`, tiles 132 × 171 (landscape pages 171 × 132), page number under each. Inspector (288 px) for the selection. Status bar: network · signed-file note when relevant · “Pages 5–6 of 24”.

**Content and controls.**

| Control | Does | Command id |
| --- | --- | --- |
| Page tile | Click selects; `⇧`-click extends; `⌘`/`Ctrl`-click toggles. Double-click opens the page in Read mode | `pages.select` |
| Selected tile | `accent-tint` tile, 2 px `accent` outline on the page, check badge, number in 600 | — |
| Drag | Drags the selection; source tiles fade to 55% (their accent outline then drops to about 2.7:1 on `accent-tint`; the check badge still carries the selected state); a 2 px `accent` insertion bar with end dots shows the drop point between tiles (no live reflow); the ghost shows a two-page stack and the count. Auto-scrolls near edges. Drop applies `MovePages` as one transaction | `pages.move` |
| Insert ▾ | Blank pages… · Pages from a file… · Pages from images… (O07) | `pages.insert-blank`, `pages.insert-from-file`, `pages.insert-images` |
| Rotate left / right | Rotates the selection 90° | `pages.rotate-left` (`⇧R`), `pages.rotate-right` (`R`) |
| Extract | Extracts the selection as a new file (desktop: save dialog; web: download) | `pages.extract` |
| Delete | Deletes the selection; “Delete 2 pages” in the inspector | `pages.delete` (`⌫` / `Delete`) |
| Move to page… | Dialog: Position radios (After page · Before page · At the start · At the end), page number input with section hint, “New order” preview, Cancel / Move | `pages.move-to` |
| Page setup ▾ | Crop… · Resize… · Page boxes… · Page labels… · Make landscape · Make portrait (designed in O09–O10) | `pages.crop`, `pages.resize`, `pages.edit-boxes`, `pages.edit-labels`, `pages.make-landscape`, `pages.make-portrait` |
| Bookmark ▾ | Add bookmark · Create a contents page… · Named destinations… (designed in O11) | `organise.add-bookmark`, `organise.create-contents-page`, `organise.named-destinations` |
| More page actions | Duplicate · Replace with pages from a file… · Reverse order · Export as images… · Merge documents… · Split document… | `pages.duplicate`, `pages.replace`, `pages.reverse`, `pages.export-images`, `document.merge`, `document.split` |
| Context menu | For the right-clicked page or selection (§7.26, 32 px items): Insert blank page before · Insert blank page after · Insert pages from a file… │ Rotate left `⇧R` · Rotate right `R` · Duplicate · Move to page… │ Extract… · Replace with pages from a file… · Export as images… │ Crop… · Resize… · Page labels… │ Delete page `⌫` (last, in `danger`) | as above |
| Inspector | Title “Pages 5–6”, section context, Rotation (two 30 px buttons and the angle), Size, Page labels, Linked from (“1 bookmark · 2 links”), actions Duplicate · Extract… · Delete 2 pages | — |
| Thumbnail size | 75%–200%, remembered per viewer | `view.thumbnail-size` |

**States.**

- **Nothing selected:** no inspector; toolbar actions that need a selection are disabled; note “24 pages”.
- **Dragging:** drawn in `O06-organise`.
- **Approval-signed file, deleting or rotating the signature's page:** [E2r](../../todos/ongoing/E2r-planner-page-checks.md)'s checks report `Restriction::SignedWidget { field }` for [E2b](../../todos/ready/E2b-page-structure-ops.md)'s `DeletePages` and `RotatePages` on the page that holds a signed signature's widget (deleting removes the signature's appearance and leaves its widget pointing at a missing page; rotating turns the signed appearance), so the save would be `Conflict`. The op is refused before applying and the `alertdialog` drawn in `O06-delete-signed` names the signature and offers “Delete page 23 only” (primary), Cancel (focus) and “Remove signature and delete…” (danger ghost, leads to the O02 confirm). For a rotate the same dialog reads “Rotate page 23 only” and “Remove signature and rotate…”. Moving, inserting or duplicating pages, and deleting or rotating other pages, on an approval-only file is `AppendOnly` and needs no dialog ([E2i](../../todos/ready/E2i-signed-gating-and-close.md)).
- **Certified file, any page change:** refused before applying (`would_violate`, the area's signed-file rule): a drop, delete, rotate or insert changes nothing and shows the banner “**Moving pages would invalidate the certification.** … nothing has been moved.” with Save a copy · Apply anyway… · Details. A dropped drag animates back to its place. Apply anyway… confirms and then applies the change as one undo step.
- **Delete with references:** the toast after deleting says what changed: “Deleted 2 pages. 1 bookmark now points to page 4.” with Undo.
- **Duplicate on a form:** toast “Duplicated page 6. Its 2 fields were copied as new fields; the signature field wasn’t copied.”
- **Large documents:** low-resolution thumbnails first; while a page renders, its tile is a white page with `hairline` (`#E4DED3`) text-line blocks, so it stays visible on the `gutter`.

**Behaviour.** Every action is one transaction and one undo step, including multi-page moves. Page labels, bookmarks, links and named destinations follow pages (E2b); deleting reports what it cleaned up. Nothing is written until Save.

**Accessibility.** The grid is a `listbox` with `aria-multiselectable="true"` and `role="option"` buttons (a reflowing grid has no fixed rows, so the ARIA `grid` pattern named in E2h doesn’t fit; this is a recommendation for E2h to confirm). Arrow keys move focus in two dimensions; Space toggles selection; `⇧`+arrows extend; `⌘A` selects all; `⇧⌘←` / `⇧⌘→` (`Ctrl Shift ←/→`) move the selection one position; `⇧⌘↑` / `⇧⌘↓` one row. Each move is announced: “Moved pages 5–6 after page 9”. Single-key shortcuts `R`, `⇧R`, `⌫` work only while the grid has focus and follow the Single-key shortcuts setting. Shift+F10 or the context-menu key opens the page menu. Move to page… is the single-pointer alternative to dragging.

**Platform differences.** Web: identical except outputs (extract and export download a file or a zip) and no drop from the OS onto the window icon; Windows/Linux use `Ctrl` chips and the Delete key. Phone: O17. Tablet: the same grid with 44 px targets; long-press starts a drag, and the bottom action bar holds Rotate, Move to…, Duplicate, Delete, More.

**Artboards:** `O06-organise.dc.html` (drag with insertion bar), `O06-organise-menu.dc.html` (page context menu), `O06-move-dialog.dc.html` (Move to page…, 440 px), `O06-delete-signed.dc.html` (approval signature, 560 px).

**Todos:** [E2h](../../todos/ready/E2h-page-organiser-ui.md), [E2b](../../todos/ready/E2b-page-structure-ops.md).

## O07 Insert pages and merge documents

**Purpose.** Bring pages in from another PDF or from images, and combine whole documents with their bookmarks, fields and tags.

**Entry points.** Organise › Insert ▾; More page actions › Merge documents…; File › Merge documents… (desktop); palette (`pages.insert-from-file`, `pages.insert-images`, `document.merge`); dropping files on the grid (insert at the drop point); on desktop, opening several PDFs together from Finder/Explorer offers “Merge into one document”.

**Layout.** Dialogs over the dimmed Organise shell, 560 px (the form width, §7.18), with a separated footer.

**Content and controls (Insert pages).** Tabs Blank pages · From a PDF · From images.

- **From a PDF** (drawn): file card (name, pages, size, comment count, “Choose another…”); “Pages to insert” with a range input (“1–4”, help “For example 1–3, 6 · 4 of 6 selected”), “Select all”, and checkable source thumbnails (checked: `accent-tint` tile, 1 px `accent` edge and the checkbox; never the 2 px focus ring); Insert (After page · Before page · At the start · At the end · Replace selected pages) and Page (number with section hint: page 11 is “in 4. Surveyor capacity”); options “Bring comments and links with the pages” (on) and “Bring form fields (renamed if a name is already used)” (off); a line saying which numbers the pages get. Primary “Insert 4 pages”.
- **Blank pages:** count, size (Same as current page, Letter, A4, custom), orientation, position.
- **From images:** image list with order controls, page size (Fit image, Letter, A4), margin, orientation from EXIF; one page per image.

**Content and controls (Merge).** Order list (`<ol>`, the ordered file list of §7.31): this document first and fixed when adding to it; each other file with the stroked Drag grip (§5), move up/down and remove buttons; “Add files…”. Result: “Add to Q3-board-report.pdf” or “Create a new document”. Bookmarks: one per added file · keep as they are · none. Form fields with the same name: rename the added field · keep them linked. A warning notice when a conformance claim will be dropped (“The result won’t claim PDF/A…”). Primary “Merge 3 files”. Commands `document.merge`, `document.merge-add-files`.

**States.** Loading a source (skeleton thumbnails); encrypted source (password prompt R03 inline in the file card); unreadable source (`danger` notice, primary disabled); invalid range (“Page 8 doesn’t exist; this file has 6 pages”, `aria-invalid`); signed target (the area's signed-file rule: approval-only files insert as an incremental save; on a certified file the insert is refused until Apply anyway…); done (toast “Inserted 4 pages after page 11” with Undo); merge report after merge (toast “Merged 3 files · 2 renamed fields · Details”, Details opens a panel listing renamed fields and destinations and dropped items).

**Behaviour.** Insert and merge are one transaction each. Source files are read, never changed, and their bytes stay in memory for undo (blob store). “Create a new document” opens the result in a new window or tab, unsaved.

**Accessibility.** Source thumbnails are labelled checkboxes; the order list is reorderable with buttons as well as by drag; every select and input has a label; counts in button labels update live.

**Platform differences.** Web: “Choose another…” and “Add files…” open the file picker; no drop from Finder onto the app icon. Windows/Linux: “Explorer” in the hint. Phone: insert from a file only, as a sheet; merge is in More.

**Artboards:** `O07-insert-pages.dc.html`, `O07-merge.dc.html`.

**Todos:** [E2c](../../todos/ready/E2c-cross-document-pages.md), [E2d](../../todos/ready/E2d-merge-and-split.md), [E3h](../../todos/done/E3h-images-to-pdf-and-page-export.md).

## O08 Split document

**Purpose.** Split a document into several new files by page count, page ranges or top-level bookmarks, leaving the original unchanged.

**Entry points.** More page actions › Split document…; File › Split document… (desktop); palette (`document.split`, accepts an argument: “split every 5”, “split 1-6, 7-15”).

**Layout.** Dialog, 560 px, over the dimmed Organise shell.

**Content and controls.** “Split by” radio group: Every [n] pages · Page ranges [input] · Top-level bookmarks (with count). The inactive options’ inputs are disabled (§7.0 Disabled: `disabled` text on a `hairline` border). Preview “You’ll get 6 files”: one row per output with file name and page range, from the outline: Chair’s summary 1–2, Repairs backlog 3–6, Rent collection 7–10, Surveyor capacity 11–15, Risk register 16–21, Appendix A. Data notes 22–24. File names select (Number and bookmark · File name and number · File name and page range). Save to: folder path with “Choose…” (desktop). Checkbox “Keep each part’s bookmarks, comments and page labels” (on). Primary “Split into 6 files”.

**States.** By count and by ranges update the preview live; invalid ranges show an inline error (“16–30: this file has 24 pages”) and disable the primary; overlapping ranges are allowed and noted (“Pages 7–8 are in two files”); no bookmarks (option disabled with “This file has no bookmarks”). Progress: determinate bar in the dialog (“Writing file 3 of 6”). Done: dialog closes, toast “Split into 6 files in Q3 parts · Show in Finder”. A file name that exists: “2 files already exist in this folder” with “Replace” / “Keep both” choice.

**Behaviour.** Split writes each part as a new file with a full write (each part is a new document, so no signature constraint applies to it); it changes nothing in the open document and adds no undo step. Signatures and certification are not carried into the parts: a signature covers the whole original file, so each part's signature fields are kept unsigned, and the dialog says so on a signed file (“The parts won’t be signed: Northwind Housing Trust’s certification covers the whole file.”). Unsaved changes are included in the parts.

**Accessibility.** Fieldset with legend for the radio group; each inline input labelled; the preview is an `<ol>` labelled by its heading and updated with a polite announcement of the file count.

**Platform differences.** Web: no folder choice; the parts download as one zip (“Q3-board-report parts.zip”) or, for two files, separately. Phone: in More; a sheet with the same options.

**Artboards:** `O08-split.dc.html`.

**Todos:** [E2d](../../todos/ready/E2d-merge-and-split.md).

## New components

Merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29; see its decisions table. Since then the boards follow those decisions: the Editor top bar with View (decision 27), the Organise toolbar and rail (decision 2), `pages.*` ids (decision 17), the menu with descriptions at 14/600 over 12 px (decision 10), dialog widths 440 · 560 · 820 (decision 11) and “Document properties…” (decision 19).

## Proposed design-language additions

Merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29; see its decisions table.
