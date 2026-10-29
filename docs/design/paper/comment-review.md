# Paper: comment and review

The Comment and review area (screens C01–C09 in the [screen inventory](screens.md#c-comment-and-review-e4e5)): marking up and drawing on pages, free text and callouts, comment threads with review status, stamps and file attachments, moving comments in and out (XFDF, FDF, Markdown), flattening, the comment summary, and annotating on a phone and a tablet. It extends the foundation board F02 (desktop, Comment mode) and follows the [Paper design language](design-language.md) throughout.

- **Canvas:** [lamina Paper — Comment and review](https://claude.ai/artifact/QCantcmCvDnr79QABZAHUo) (17 artboards, one row per screen, a sticky note per row). The published canvas predates the 2026-10-29 review fixes; the files in `comment-review/project/` are the source of truth until it is republished.
- **Plan:** [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and [§E5](../../plans/full-editor.md#e5-review-stamps-and-interchange); layout rules in [ui-ux §5](../../plans/ui-ux.md#5-design).
- **Proposed** marks values and behaviour that the design language does not already define; they are the defaults until a reviewed board replaces them.

Rules that hold for every screen here:

- **Comment mode has one secondary toolbar** (design language §8.1), now in full: Select │ Highlight · Underline · Strike · Caret · Note · Text box ▾ · Ink · Shapes ▾ · Stamp ▾ · Attach file │ Colour · Width │ … · "n comments on this page" · Comments. The colour and width controls appear only for tools that use them. To keep it on one row (proposed):
  - Underline, Strike and Caret are 28 × 28 icon-only toggles with an `aria-label` and a tooltip naming the tool and its key ("Underline (U)"); every other tool keeps its label.
  - Text box ▾ is a split tool button (§7.24): the toggle shows the last-used text tool, the 20 px chevron part ("Choose text tool") opens the menu.
  - The "n comments on this page" note hides while Colour and Width are shown; the Comments button then carries the count in its name ("Comments, 5 on this page").
  - Measured this way the toolbar is about 1,350 px wide with Colour and Width showing, so it fits one row from a 1,366 px window up; below that it wraps onto two rows (this replaces §8.1's 1,024 px rule for Comment; see the proposed additions). Below 600 px the phone shell (C08) takes over.
- **Shell** (design language §7.1, §7.8, §8.10): every desktop board uses the Editor top bar (save state, a divider, Undo and Redo naming the step, the mode switcher, page, zoom, Find, View, Commands, Save, More) and the status bar's "0 network requests for this document · Offline-safe: nothing uploaded". The rail shows Pages · Outline, plus Signatures on a signed file; there is no Bookmarks tab.
- **Two colour sets** (proposed, see [design language §2.7](design-language.md#27-fixed-content-palettes-proposed)): fill colours (the four highlight colours) for highlight, fills and note icons; line colours (Black, Red, Blue, Green) for underline, strike, caret, ink, shapes, lines and text. The toolbar swaps the set with the tool.
- **Creating an annotation selects it** and opens the docked inspector (§8.2). Escape returns to the Select tool, then deselects.
- **Every change is one undo step** (ADR 0006) and nothing is written until Save (§8.4). All annotation ops are class `Annotation` and save incrementally; flattening is class `Content`. On signed files the status-bar note or the banner says what Save will do (§7.13).
- **Single-key tool shortcuts** (proposed) work only while the page has focus and can be turned off in Settings (WCAG 2.1.4): V Select, H Highlight (from the files), U Underline, N Note, T Text box, P Ink, R Rectangle, A Arrow, ⌫ Delete selected (from the files). Strike, Caret, the other shapes, Stamp and Attach file have no shortcut; they are in the palette and menus.

## C01 Comment tools

**Purpose.** Pick a markup or drawing tool and its colour and width, then mark the page.

**Entry points.** The Comment mode button in the top bar; palette "Comment mode"; selecting a comment tool from the palette (for example "Highlight selection", `H`) switches to Comment mode; the selection mini toolbar's Highlight and Comment buttons create the annotation in any mode and then switch to Comment mode, since Read mode never changes the file (design language §8.11).

**Layout.** Desktop window: top bar (Comment active), the Comment secondary toolbar, left rail (Outline), canvas, status bar. No inspector until something is selected. The status bar's middle slot shows a one-line tool hint while a creation tool is active (proposed; the signed-file note takes the slot back once there are unsaved changes on a signed file).

**Content and controls.**

| Control | Label | Does | Command id |
| --- | --- | --- | --- |
| Toggle | Select | Select, move and resize existing annotations; text selection still works | `comment.select` |
| Toggle | Highlight | Drag across text to highlight it (snaps to text runs) | `comment.highlight` |
| Icon toggle | Underline (tooltip "Underline (U)") | Underline text | `comment.underline` |
| Icon toggle | Strike (tooltip) | Strike-through text (a deletion suggestion) | `comment.strike` |
| Icon toggle | Caret (tooltip) | Click between characters to suggest an insertion; opens a note | `comment.caret` |
| Toggle | Note | Click to place a sticky-note icon; focus goes to its note field | `comment.note` |
| Split toggle | Text box ▾ | Toggle part: the last-used text tool. Chevron part "Choose text tool" (`aria-haspopup="menu"`): Text box, Callout, Typewriter text | `comment.text-box`, `comment.callout`, `comment.typewriter` |
| Toggle | Ink | Freehand drawing | `comment.ink` |
| Menu button | Shapes ▾ | Menu (radio items): Rectangle `R`, Ellipse, Line, Arrow `A`, Polygon, Polyline; footer hint "Hold Shift for squares, circles and 45° lines" | `comment.shape` (`kind`: rectangle … polyline) |
| Menu button | Stamp ▾ | Opens the stamp picker (C05) | `comment.stamp` |
| Toggle | Attach file | Click a spot, then pick a file (C05) | `comment.attach-file` |
| Swatches | Colour | Four fill or four line colours, 24 px, named | `comment.set-colour` (colour name) |
| Select | Width | 0.5–8 pt, for line tools | `comment.width` |
| Text | n comments on this page | Count on the current page; hidden while Colour and Width are shown | — |
| Toggle | Comments | Opens the Comments panel (C04). Secondary-button edge (`control-border`), unlike the borderless tool toggles, because it opens a panel rather than choosing a tool. Its name carries the page count while the note is hidden | `comments.toggle-panel` |

**States.**

- *Tool active:* the toggle is selected (accent border and tint); the cursor is a crosshair (I-beam for text tools); the hint in the status bar says how to use it and the keyboard alternative.
- *Menu open:* drawn on the artboard: Shapes ▾ open, Rectangle checked, keyboard focus on Arrow.
- *Certified file* (artboard `C01-comment-tools-certified`, drawn at DocMDP P2 with one highlight added): when the certification forbids annotations (P1 or P2), the constraint banner of design language §7.13 sits under the toolbar from the moment Comment mode opens. It is a `region` named "Signature warning". Wording, with a clause for each level:
  - P2: "**Saving will invalidate the signature.** This file is certified by Northwind Housing Trust for form filling only, and comments are a change it doesn’t allow. Save a copy instead to keep the original signed."
  - P1: "… This file is certified by Northwind Housing Trust to allow no changes. …"

  Actions: **Save a copy…** (secondary, `file.save-copy`; "Download a copy" on the web, §8.4) and **Details** (ghost, `file.show-constraint-details`, opens the signature panel). Save stays Save and shows once there are changes. As in O02's Conflict state, Save opens the remove-signature confirm ("Remove the certification and save?", `file.save-invalidating-signatures`), which is the explicit override of ADR 0007 §3. Undoing every comment removes nothing from the banner while the mode is open, because the next mark would bring it back. The status bar shows a positive "Certified" badge with "Northwind Housing Trust · form filling only". The rail adds the Signatures tab (§8.10). With P3 or approval signatures only there is no banner; the F02 status-bar note "Signed file. Saving adds your changes after the signature; the signature stays valid." applies.
  - *Why the banner shows on mode entry* (proposed, agreed with the Fill & Sign P1 rule in [forms](forms.md)): every tool in Comment mode makes a change that P1 and P2 forbid, so waiting for the first mark would only warn after the fact. Fill & Sign shows it on entry for P1 only, because P2 allows filling. Other areas keep §7.13's "first offending edit" timing.
- *Disabled:* in a file opened read-only (permissions forbid annotating, ISO 32000-2 Table 22 bit 6), the tools are disabled with the reason in their tooltip and a status-bar note "Comments are turned off for this file by its author".

**Behaviour.** Choosing a tool stays on it until Escape or another tool (sticky tools); double-click a tool to make it one-shot (proposed). Colour and width apply to the next mark and are remembered per tool on this device (E4f "recent colours and defaults per tool"). Each mark adds one `AddAnnot` op with a generated appearance stream, so other viewers show it identically.

**Accessibility.** The toolbar is `role="toolbar"`; arrow keys move between tools, Tab leaves it. Menus are `role="menu"` with `menuitemradio`. Keyboard creation: with a text tool active, select text with the keyboard (caret browsing) and press ↵ to mark it; with a shape, Note or Stamp tool, ↵ places a default-size item at the centre of the visible page, which then moves with the arrow keys (C02). Swatches have colour names.

**Platform differences.** Web and desktop are identical; desktop also lists every tool in the native Comment menu (D4b). Windows/Linux chips use words (`Ctrl K`). Below about 1,366 px the toolbar wraps onto two rows; on a tablet without a pen it uses the touch metrics of design language §6.2 (56 px bar, 44 px toggles) with the same tools; at phone width see C08.

**Artboards.** `C01-comment-tools.dc.html`, `C01-comment-tools-certified.dc.html`.

**Todos.** [E4f](../../todos/ready/E4f-annotation-tools-ui.md).

## C02 Ink and shapes

**Purpose.** Draw freehand ink and shapes, then adjust a selected one.

**Entry points.** Ink (`P`) and Shapes ▾ (`R`, `A`) in the Comment toolbar; palette "Draw ink", "Draw rectangle", …

**Layout.** In progress: toolbar with Ink selected, Colour (line colours) and Width; rail closed in the artboard to show the page wider; no inspector. Selected: Select tool active, the shape inside a 1 px `accent` outline with eight 8 px white square handles (1.5 px `accent` border, 24 px hit area, design language §7.32), inspector open.

**Content and controls (inspector "Rectangle").** Line colour (four line colours) · Fill (No fill + four fill colours) · Width (select, pt) · Opacity (number, %) · Border: Solid / Dashed / Cloudy (toggle group; cloudy is `/BE`) · Note (textarea, the annotation's `/Contents`) · Author and time · Reply · Delete rectangle. Lines and arrows add **Line start** and **Line end** selects (None, Open arrow, Closed arrow, Circle, Square, Diamond, Butt, Slash: every `/LE` style). Ink adds **Smoothing** (On/Off, proposed). Command ids: `annot.set-colour`, `annot.set-fill`, `annot.set-width`, `annot.set-opacity`, `annot.set-border-style`, `annot.set-line-ends`, `annot.delete`.

**States.**

- *Drawing* (artboard `C02-ink-drawing`): the stroke follows the pointer in the chosen colour; the pen cursor previews colour and width; the status bar reads "Drawing ink · release to finish the stroke · Esc cancels it" (`role="status"`). Strokes drawn within 1 s of each other join one Ink annotation (proposed), so a handwritten word is one comment.
- *Selected* (artboard `C02-shape-selected`): a cloudy blue rectangle round the West row of the chart.
- *Multiple selected:* handles round the group; the inspector shows shared properties and "3 items".

**Behaviour.** Drag a handle to resize, the body to move; Shift keeps proportions. Arrow keys move 1 pt, Shift+arrows 10 pt, Alt+arrows resize from the bottom-right handle (the non-drag alternative, WCAG 2.5.7). Each property change is one `UpdateAnnot` op; dragging coalesces into one undo step. The `/Rect` is recomputed to include stroke width and line endings (E4d).

**Accessibility.** Each annotation on the page is a button; a selected shape is announced as "Rectangle, cloudy border, blue, selected"; Tab moves between annotations on the page in reading order; Shift+F10 opens its context menu. The border styles are real toggle buttons; the inspector is reachable with F6 (desktop) or Tab.

**Platform differences.** Pressure-sensitive width where `PointerEvent.pressure` is meaningful (pens, Force Touch trackpads); a mouse draws constant width. Tablet and phone: C08 and C09.

**Artboards.** `C02-shape-selected.dc.html`, `C02-ink-drawing.dc.html`.

**Todos.** [E4d](../../todos/ready/E4d-appearances-ink-and-shapes.md).

## C03 Free text and callouts

**Purpose.** Type on the page: a text box, a callout with a leader line, or typewriter text (no border, for filling flat forms).

**Entry points.** Text box ▾ (`T`) in the toolbar; palette "Add text box", "Add callout", "Add typewriter text"; double-click an existing text box with Select.

**Layout.** The callout is edited in place on the page: the text area (a `contenteditable` `role="textbox"`) with a caret inside a 1 px `accent` outline, the two side handles only (text boxes don't take corner handles, §7.32), and two round 10 px handles on the leader line (knee and arrow end). The inspector shows text properties; the toolbar's Text box split button reads "Callout".

**Content and controls (inspector "Callout").** Font (Helvetica, Times, Courier and the document's embeddable fonts; the standard 14 by default) · Size (pt) · Bold, Italic (the rich-text subset kept in `/RC`) · Alignment left, centre, right (`/Q`) · Text and line colour (four line colours) · Fill (White, the four fill colours Yellow, Green, Blue, Pink, and No fill) · Border width · Line end · a note "Bold and italic are kept as rich text; other viewers that ignore it show plain text." · Done editing · Delete callout. Command ids: `annot.toggle-bold`, `annot.toggle-italic`, `annot.set-alignment`, `annot.set-font`, `annot.set-font-size`, `annot.done-editing`.

**States.** *Editing* (drawn); *selected, not editing* (handles, no caret); *empty new box* shows the placeholder "Type here" until typed and is removed if left empty; *missing glyphs*: characters the chosen font lacks are drawn with a fallback font and a warning notice in the inspector "Some characters use a fallback font" (proposed); *Acrobat-made box*: editing keeps its `/RC` and regenerates the appearance (E4e).

**Behaviour.** Typing coalesces into one `UpdateAnnot` op per editing session. Escape or "Done editing" leaves the text and keeps the callout selected; Escape again deselects. The box grows downward as text wraps; drag the side handles to change wrap width. The leader line's knee and end move by drag, or by Tab to a handle and the arrow keys.

**Accessibility.** The text area has an accessible name ("Callout text") and is multi-line; formatting commands also work with ⌘B / ⌘I (Ctrl B / Ctrl I) while editing. Leader-line handles are focusable with names ("Callout arrow end", "Callout knee").

**Platform differences.** Desktop can offer system fonts (passed as bytes, E6c) in addition to bundled ones; web lists bundled fonts only. On touch the text area uses 16 px minimum UI text for its toolbar but keeps the document's point size on the page.

**Artboards.** `C03-callout.dc.html`.

**Todos.** [E4e](../../todos/ready/E4e-freetext-callouts.md).

## C04 Comments panel

**Purpose.** See every comment in the file as threads, reply, set review status, filter and search, jump to a comment, and delete in bulk.

**Entry points.** The Comments toggle at the right end of the Comment toolbar; palette "Show comments"; clicking a comment's Reply in the inspector; the status-bar count on a page with comments (proposed). Read mode can open it too (from More › Comments), so reviewers can read threads without entering Comment mode.

**Layout.** A docked panel on the right, 360 px (proposed), `surface`, hairline left, in the inspector's place: while the panel is open, selecting an annotation expands its thread in the panel instead of opening the inspector; "Properties" in the thread's More menu swaps the dock to the inspector, and "Back to comments" returns (proposed: one right dock, never two). Header: "Comments" (Newsreader 20) with "14 comments · 9 open"; More (Import, Export, Summary, Flatten, Delete all shown…) and Close. Below: search field, Filter ▾ button, sort select (Page order, Newest first, Author), and the threads grouped by page: each group is an `h3` in overline style ("Page 3 · Repairs backlog") followed by its own list named after the page.

**Content and controls.**

- **Thread row:** type mark (fill swatch or line glyph), author, type, date, status badge, quoted text for text markup (Newsreader italic), note, reply count. The row's header is a button that selects the annotation and scrolls the page to it (`comment.go-to`).
- **Open thread** (selected, accent tint with an inset accent ring): replies listed under the parent (indented, initials avatar, author, time), a Reply field and button (`comment.reply`), and a **Status ▾** menu: None, Accepted, Rejected, Cancelled, Done (`comment.set-status`; `/StateModel /Review`, "Done" is `Completed`), plus a **Mark as checked** checkbox above the Status row (`/StateModel /Marked`, proposed label; drawn unticked).
- **Status badges** (design language §7.23): Accepted positive with a check; Rejected danger with a cross; Done accent with a check; Cancelled neutral; open threads have no badge. The line under the note names who set it ("Accepted by A. Mensah").
- **Filter menu** (artboard `C04-comments-filter`): checkbox items with counts, grouped Status (Open, Accepted, Rejected, Cancelled, Done), Author (with "(you)"), Type (Highlights and text marks, Notes and text boxes, Drawings and shapes, Stamps and files), and "Date and page range…". Active filters show as removable chips under the button, with Clear. The header count changes to "3 of 14 shown". Command ids: `comments.filter`, `comments.search`, `comments.sort`.
- **Bulk selection:** each row gets a checkbox and "Select all shown" appears; a footer toolbar reads "3 selected" with Set status ▾ and "Delete 3…" (`comments.delete-selected`, the `DeleteAnnots` op).

**States.**

- *Empty file:* empty state (§7.21) "No comments yet" / "Choose a tool above, or select text and press H."
- *No matches:* "No comments match" / "Clear filters" (ghost button).
- *Thread with a missing parent or a reply cycle* (from other apps): shown flat under a neutral notice "Some replies in this file point to missing comments; lamina shows them on their own" (E5b's tolerant thread model).
- *Deleting:* "Delete 3 comments and their replies?" (`alertdialog`, danger-filled "Delete 3"); undoable until saved.

**Behaviour.** Replies (`AddReply`) and status (`SetReviewState`) are separate ops, each one undo step, written as standard reply annotations so Acrobat shows them as threads. Search matches note text, replies, quoted text and author, highlighting matches in the rows. The list follows the page: scrolling the document doesn't move the list, but selecting on the page scrolls the list to that thread. Author name comes from Settings › Your name (asked once, the first time you comment; stored on this device).

**Accessibility.** The threads sit in a group named "Comment threads"; each page has an `h3` heading and a `role="list"` named after the page; each thread is a list item whose header button (type mark, author, type, date, status) is in the Tab order; replies are an ordered list. The filter menu uses `menuitemcheckbox`. The open thread's reply field is focused when you choose Reply. Status is never colour alone (word plus icon). Live region announces "Reply added" and filter result counts.

**Platform differences.** Web and desktop are identical (the inventory lists D, W). Between 600 and 1023 px the panel opens docked and closes the rail; if the canvas would fall below 560 px it covers the canvas as a full-height panel with a Close button (proposed). Phone: C08's comment sheet; the full list is a sheet from More › Comments.

**Artboards.** `C04-comments-panel.dc.html`, `C04-comments-filter.dc.html`.

**Todos.** [E5b](../../todos/ready/E5b-comment-threads-and-review.md).

## C05 Stamps and file attachments

**Purpose.** Stamp a page (standard text stamps, dynamic stamps with your name and the time, custom image stamps) and attach a file to a spot on a page.

**Entry points.** Stamp ▾ and Attach file in the Comment toolbar; palette "Add stamp…", "Attach file to page…"; dropping a file onto the page in Comment mode attaches it (proposed).

**Layout.** The stamp picker is a popover under Stamp ▾ (`role="dialog"`, not modal, 344 px) with tabs Standard · Dynamic · Custom. Placing follows: the chosen stamp rides the pointer until a click; the status bar says "Pick a stamp, then click the page to place it · ↵ places it at the centre". A selected attachment opens the inspector.

**Content and controls.**

- **Standard:** Approved, Not approved, Reviewed, Draft, Final, Confidential, For comment, Void (text appearances, line colours).
- **Dynamic:** Approved, Reviewed, Received, Not approved, each with "by R. Osei · 29 Oct 2026, 10:14". Footnote: "The name and time are written into the stamp when you place it and don’t change later. Name from Settings › Your name." (resolved at creation, never by JavaScript; E5c.)
- **Custom:** "Add image…" (PNG, JPEG; a signature-like image here is just a picture, see Fill & Sign for signing), the user's saved stamps with Rename and Remove. Stored on this device only (desktop app data, web IndexedDB).
- **Attachment inspector** ("Attached file"): file card (name, size, type, date added) · Save file as… · Replace… · the line "lamina doesn’t open attached files. Save it, then open it with an app you trust." · Description · Icon (Paperclip, Push pin, Graph, Tag) · Icon colour (the four fill colours) · Author · Reply · Delete attachment. Command ids: `comment.place-stamp`, `stamps.add-custom`, `attachment.save`, `attachment.replace`, `annot.delete`.
- **Sound comments** (read only): listed in C04 with a Play button on desktop ("Play sound comment, 0:12"); web lists them with "Playing sound comments needs the desktop app". XFDF import skips them (C06).

**States.** Picker on Dynamic with keyboard focus on Approved (drawn); Custom empty: "No custom stamps yet" / Add image…; attachment too large (over the soft limit in `Limits`): notice "This file is 48 MB; the PDF will grow by about that much" with Attach anyway (proposed).

**Behaviour.** A stamp is one `AddAnnot` `Stamp` op and is selected after placing (resize keeps proportions). An attachment embeds the file bytes in the PDF (file specification, ISO 32000-2 §7.11) as one `AddAnnot` `FileAttachment` op; Save file as… writes the bytes unchanged.

**Accessibility.** Tabs are a tablist; each stamp is a button with its full text as the name. Attachment icons on the page are buttons named "Attached file north-90-day-list.xlsx".

**Platform differences.** Desktop uses the native save dialog for Save file as…; web downloads the file. Phone: Stamp and Attach file are under the annotate bar's More (not drawn).

**Artboards.** `C05-stamps.dc.html`, `C05-attachment.dc.html`.

**Todos.** [E5c](../../todos/ready/E5c-stamps-and-attachments.md).

## C06 Import and export comments

**Purpose.** Send comments to other apps and people (XFDF, FDF) and bring theirs back; export highlights and notes as Markdown for notes apps.

**Entry points.** Comments panel More › Export comments… / Import comments…; palette "Export comments…", "Import comments…", and "Export highlights as Markdown" (`⌥⌘E` on macOS, from the files; no Windows/Linux shortcut per §8.8); desktop File › Export and File › Import menus; dropping an `.xfdf` or `.fdf` file on the window opens Import.

**Layout.** Dialogs (§7.18) over the dimmed shell.

**Content and controls — Export** (artboard `C06-export`, 600 px). Subtitle "Makes a separate file. Q3-board-report.pdf doesn’t change." · **Format** as three radio cards: XFDF ("For Acrobat and most PDF apps. Keeps replies and status."), FDF ("An older format some apps still ask for."), Markdown ("Highlights and notes as text, for your notes app.") · **Comments** select: Highlights and notes (Markdown default), All comments, Shown in the panel · **Include** (Markdown only): Page numbers and headings, Author and date, Replies · **Preview** (Markdown only; a scrollable `pre` with a count, "11 items on 4 pages", showing only what the Comments choice includes: highlights, their quotes and notes) · a footer hint naming the file, "Saves as Q3-board-report – highlights.md" · Cancel · Copy (Markdown only, to the clipboard) · Save as… (primary). Command ids: `comments.export`, `comments.export-markdown`, `comments.copy-markdown`.

**Content and controls — Import** (artboard `C06-import`, 520 px), shown after a file is chosen: file card ("review-jpark.xfdf · XFDF · 9 comments · made for Q3-board-report.pdf") with Choose another… · a plain summary list: "6 new comments will be added, on pages 1, 3, 11 and 16", "2 are already in this file (same comment ID), with changes" · **For comments already here** radios: Merge (keep them and add new replies and status; default), Replace them with the imported version, Add the imported ones as separate copies (the E5d modes: merge, replace by `/NM`, add) · a warning notice for what can't be imported ("1 comment will be skipped. A sound comment on page 12: sound comments in XFDF files aren’t imported. The other 8 aren’t affected.") · "You can undo the import in one step." · Cancel · Import 8 comments. Command id: `comments.import`.

**States.** Import: the file was made for a different PDF (its `/F` or `/ID` differs): warning notice "This file was made for Q2-board-report.pdf. Comments may land in the wrong places." with Import anyway; comments for pages that don't exist are listed and skipped; a malformed file: error notice "lamina couldn’t read this file: it isn’t valid XFDF (line 41)" and the Import button disabled; duplicate IDs inside the file are reported (ADR 0005 §5). Export: no comments: the dialog's Save button is disabled with the reason "This file has no comments to export". After export, a toast "Comments exported" with Show file (desktop).

**Behaviour.** Export never changes the PDF. Import is one composite `ImportAnnotations` op (one undo step) and marks the file changed; nothing is fetched from the network, and the XML parser refuses DTDs (E5d). Markdown is generated from the same model the summary uses: one heading per page with its section title, quotes as block quotes, notes as plain paragraphs.

**Accessibility.** Radio cards are labels wrapping real radios (`fieldset`/`legend`); the preview is focusable so it can be scrolled with the keyboard; the warning notice is `role="status"`.

**Platform differences.** Web: Save as… downloads (the browser's save location); desktop uses the native save and open dialogs. Phone: Export is in More › Export comments and opens the share sheet for the file.

**Artboards.** `C06-export.dc.html`, `C06-import.dc.html`.

**Todos.** [E5d](../../todos/ready/E5d-xfdf-fdf.md).

## C07 Flatten and comment summary

**Purpose.** Flatten: make comments a permanent part of the page content. Summary: make a new PDF listing the comments beside their pages.

**Entry points.** Comments panel More › Flatten comments… / Comment summary…; palette "Flatten comments…", "Comment summary…"; desktop File › Print comment summary…; Print dialog's "Comments: Summary" option links here (C6c, proposed).

**Layout.** Flatten is an `alertdialog` (500 px); Summary a wide dialog (780 px) with options left and a live preview right that wraps under the options when narrow.

**Content and controls — Flatten** (artboard `C07-flatten`). Title "Flatten comments into the pages?" · body: what flattening means, and "Form fields and signature fields are never flattened." · **Which comments**: All comments (14), On this page (3), Shown in the Comments panel (14 here, since the panel shows all), Selected (n) when there is a selection; "Not counted: 2 hidden comments, which stay as they are." (`include_hidden: false`; the counts, like the panel's "14 comments", leave hidden ones out) · a signature note (approval signatures only): "**Signed file.** The signature by M. Adeyemi stays valid, but its details will show that page content changed after signing." · checkbox **Also remove the comments from the file’s history** with "Saving normally adds to the file, so earlier versions inside it still hold the comments. This writes a whole new file instead, which invalidates the signature by M. Adeyemi." · "You can undo this until you save." · Cancel · **Flatten 14 comments** (danger-filled). Command id: `comments.flatten`.

**History option ticked** (artboard `C07-flatten-history`). The save becomes a full rewrite, which an approval signature can't survive (ADR 0007 §3), so the dialog changes:

- The signature note becomes a danger notice (`role="alert"`): "**Saving will invalidate the signature.** Removing the comments from the file’s history writes a whole new file, so the signature by M. Adeyemi will no longer be valid. Untick the option below to keep it."
- The override checkbox "I understand the signature will no longer be valid" appears, unticked (drawn with the focus ring).
- The confirm button reads **Flatten and rewrite…** and stays disabled (`aria-disabled`, disabled tokens) until the override is ticked; then it is danger-filled.
- The footer line reads "You can undo this until you save. Saving rewrites the file."

**Content and controls — Summary** (artboard `C07-summary`). Layout: Page beside its comments / Comments only, as a list · Comments: All comments / Shown in the Comments panel · Include: Replies, Status, Pages without comments · Paper size: Same as the document / A4 / Letter · Preview with numbered markers on the page thumbnail matching numbered comments, "Summary page 2 of 7" · "Opens as a new, unsaved document. Q3-board-report.pdf doesn’t change." · Cancel · Print… · Create summary. Command ids: `comments.summary`, `comments.print-summary`.

**States.** Flatten on a **certified** file (any DocMDP level, since flattening is class `Content`): the note becomes a danger notice "**Flattening will invalidate the signature.** This file is certified by Northwind Housing Trust. Save a copy instead to keep the original signed." and the confirm button reads "Flatten in a copy…"; continuing in place needs the explicit override checkbox "I understand the signature will no longer be valid" (ADR 0007 §3). Comments without appearance streams get one generated first; if one can't be generated the dialog lists it and leaves it unflattened ("1 comment can’t be flattened: a sound comment"). Summary: no comments → Create disabled with the reason; building a long summary shows a determinate progress bar in the dialog.

**Behaviour.** Flatten is one `FlattenAnnots` op (one undo step): each appearance becomes page content and the annotation is removed. Save then appends (incremental) unless the history checkbox was ticked, which makes that save a full rewrite. The summary is built locally and opens as a new document tab (desktop: a new window) titled "Q3-board-report – comments.pdf", unsaved.

**Accessibility.** The flatten dialog uses `role="alertdialog"` with `aria-describedby` on its explanation; focus starts on Cancel (proposed for destructive dialogs), drawn here on the first radio for the review's focus-ring rule. The summary preview is `role="img"` with a text description; the numbered markers are repeated in the comment text.

**Platform differences.** Desktop opens the summary in a new window and Print… uses the native print dialog; web opens it in a new tab of the app and prints through the browser (R09's page-appearance warning applies).

**Artboards.** `C07-flatten.dc.html`, `C07-summary.dc.html`, `C07-flatten-history.dc.html`.

**Todos.** [E5e](../../todos/ready/E5e-flatten-and-summary.md).

## C08 Phone annotate

**Purpose.** Draw, highlight, erase and leave notes with a finger on a phone, and read and answer a comment.

**Entry points.** Annotate in the bottom bar (the phone's way into Comment mode, §6.1); Highlight and Comment in the selection action bar (F04) for text; tapping an annotation on the page opens its comment sheet.

**Layout — annotate** (artboard `C08-annotate-phone`). The top bar stays (no auto-hide while annotating): Back, title with "Annotating · 1 unsaved change", Undo (44 px, named after the step: "Undo pen stroke"). The bottom bar is replaced by an **annotate bar** (`role="toolbar"`, same 84 px grid): Pen, Highlighter, Eraser, Note, Done. Above it a 60 px **options strip** for the active tool: four line colours (26 px circles with the 1 px `control-border` edge in 44 px buttons; the selected one, Red on the board, gets the touch ring of §7.12: 2 px `surface` gap and 2 px `text`) and Thin / Medium / Thick widths (44 px toggles showing a 20 px line 1.5, 3 or 5 px thick; Medium selected with the `accent` border and tint). The board shows a red pen loop round "older than 90 days" with an arrow into the margin.

- **Scroll hint** (proposed): the page indicator pill (§6.2) shows "Use two fingers to scroll" instead of the page number for the first three annotate sessions, then goes back to page numbers. It sits 16 px above the options strip, as the pill sits 16 px above the bottom bar, and is `aria-live="polite"`. It is a state of the pill, which already floats over the page, not a new floating layer.

**Layout — comment sheet** (artboard `C08-comment-sheet-phone`). A bottom sheet at the medium detent (sheet top at 404 px, §6.2) over the reader shell: grabber, title "Highlight" with "Page 3 · 2 replies", More and Close (44 px); author, time and a status button (badge plus chevron, opens a status sheet); the quoted text (Newsreader italic with a 2 px `hairline` rule on its left); note; replies (12 px initials in 28 px avatars); a Reply field (16 px text, 44 px high) with Send pinned to the sheet's bottom edge. The annotation stays outlined on the page above the sheet.

**Controls.** `comment.pen` (the touch name of the Ink tool: the same `AddAnnot` Ink op as `comment.ink`, so the palette lists one "Draw ink" command and `comment.pen` is its alias on touch bars), `comment.highlighter` (freehand highlighter that snaps to text lines when drawn over text, proposed), `comment.eraser` (erases whole strokes; tap or drag across), `comment.note`, `comment.done`, `comment.reply`, `comment.set-status`.

**States.** Pen drawing (drawn); Highlighter with fill colours in the strip; Eraser with no options strip; Note placing (tap a spot, the comment sheet opens at the large detent with the keyboard). Leaving with Done keeps changes unsaved; Back with changes asks "Save changes to Q3-board-report.pdf?".

**Behaviour.** One finger draws, two fingers pan and pinch (the viewer owns gestures, plan §5.4); long-press does nothing in Pen mode. Outside annotate mode a pen scrolls and selects like a finger: Read mode never changes the file (design language §8.11), so there is no "Pen draws in Read mode" setting. Undo removes the last stroke.

**Accessibility.** All targets ≥ 44 px; the pen tools are toggle buttons with names; each drag has an alternative: Note by tap, sheet resize by the grabber button; the reply field is labelled. Focus ring drawn inset on the Pen button (§7.0).

**Platform differences.** iOS: no Apple Pencil-only distinction in the web app (plan open question 2); native apps after ADR 0025. Android: same.

**Artboards.** `C08-annotate-phone.dc.html`, `C08-comment-sheet-phone.dc.html`.

**Todos.** [E4f](../../todos/ready/E4f-annotation-tools-ui.md), [U3a](../../todos/ready/U3a-responsive-mobile-and-touch.md).

## C09 Tablet stylus

**Purpose.** Write and draw with a stylus at full page size while the hand rests on the screen.

**Entry points.** Comment mode on a tablet once a pen has been used; picking up the pen while in Comment mode selects the Pen tool (proposed).

**Which toolbar.** Design language §6.2: a tablet in Comment mode uses the Comment toolbar of C01 at touch size (56 px, 44 px toggles); once lamina sees a pen (`pointerType: "pen"`), the **tool shelf** (§7.35) replaces it for the rest of the session. This is the one tablet Comment toolbar. Reader B's [R17-tablet](reader-b.md#r17-tablet-reading) follows it (the shelf, with Highlighter pressed).

**Layout** (artboard `C09-stylus-tablet`, 1024 × 1366 portrait). The desktop shell with touch rules (§6.2): a 56 px top bar, the docked tool shelf as the secondary toolbar (60 px, wraps when narrow), the page at full width (860 px wide, rail collapsed), and a 44 px status bar with 13 px items. The shelf is the only thing between the top bar and the page; nothing floats over the page while drawing.

**Content and controls.** Top bar (the Editor variant of §7.1 at touch size, without zoom and Commands): Sidebar, file name with "1 unsaved change", a divider, Undo ("Undo ink stroke") and Redo (disabled), the mode switcher (Comment active; 56 px buttons with 10 px side padding), page 3 / 24, Find, View, Save, More; all targets 44 px. Tool shelf (`role="toolbar"`, "Pen tools"): Pen (pressed, drawn with the focus ring), Highlighter, Eraser, Lasso, Shapes, Text, Note as 48 px icon-over-label buttons │ four line colours in 44 px buttons (edges and selected ring as in C08; Blue selected) │ Thin / Medium (selected) / Thick │ "Draw with finger" switch (off). Status bar: the network item as a button, "0 network requests for this document · Offline-safe: nothing uploaded" (§7.8), "Pen draws, finger scrolls. A resting hand is ignored." in the middle (`aria-live="polite"`), "Page 3 of 24" right. Command ids: `comment.pen` (alias of `comment.ink`, see C08), `comment.highlighter`, `comment.eraser`, `comment.lasso` (proposed), `comment.shape`, `comment.text-box`, `comment.note`, `comment.draw-with-finger` (proposed). The board shows a pressure-varying blue loop round the highlighted sentence with an arrow into the margin, the pen at the end of the stroke, and a dashed palm shape (mock-up only) marking the ignored contact.

**Behaviour.** Palm rejection by pointer type: while "Draw with finger" is off, only `pointerType: "pen"` draws; touches pan and pinch, and large-contact touches during a pen stroke are ignored. Pressure varies the stroke width (stored per E4a). Hovering pens (Apple Pencil hover, S Pen) show a preview ring of colour and width. Lasso selects strokes to move, recolour or delete. The barrel button or double-tap on the pen toggles Pen/Eraser where the platform reports it (proposed).

**States.** Writing (drawn, with a faint palm shape showing the ignored contact for the mockup only); Draw with finger on: the status note changes to "Finger draws. Use two fingers to scroll."; no pen detected: the switch is on by default.

**Accessibility.** Shelf buttons ≥ 44 px with labels; the switch is a `role="switch"` with a visible label; every pen action has a non-drag alternative through Select and the inspector (C02).

**Platform differences.** iPadOS and Android tablets in the web app; landscape uses the desktop layout with the rail open. Desktop pen tablets (Wacom) behave like C02 with pressure.

**Artboards.** `C09-stylus-tablet.dc.html`.

**Todos.** [E4f](../../todos/ready/E4f-annotation-tools-ui.md).

## New components

The earlier proposals were merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29 (see its decisions table). New since then, from the final review:

- **Panel toggle in a secondary toolbar:** the Comments button at the right end of the Comment toolbar is a §7.24 toggle with the secondary-button edge (1 px `control-border`, `field` fill), so a toggle that opens a panel reads differently from the tool toggles. Pressed: `accent` border and tint, as for tools.
- **Icon-only tool toggle:** a §7.24 toggle at 28 × 28 with no label, `aria-label` and a tooltip naming the tool and its key. Used only to fit a mode toolbar on one row (Underline, Strike, Caret).
- **Width toggles (touch):** 44 px toggles holding a 20 px line at the stroke's thickness (1.5, 3, 5 px) drawn in `currentColor`; selected: `accent` border and tint.

## Proposed design-language additions

Merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29; see its decisions table.
