# Paper: add and edit content

Screens A01–A07 of the [screen inventory](screens.md#a-add-and-edit-content-e6e7): adding text, images, shapes and page marks to a PDF, and editing what is already on the page (text, images, vector paths), with the warnings that keep editing honest: font substitution, text that can't be edited in place, replace-all failures, and tagged or PDF/A files whose claims an edit could break. The work it designs is planned in [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) and [§E7](../../plans/full-editor.md#e7-editing-existing-content).

- **Canvas:** [lamina Paper — Add and edit content](https://claude.ai/artifact/5CecE9xXiw2RXoVde7xEsL) (one row per screen, a sticky note per row).
- **Look:** the [Paper design language](design-language.md); every board copies the Desktop foundation board's window frame, top bar, rail, inspector and status bar.
- **Platform:** all seven screens are **D** (desktop first). The web app uses the same layout without the window frame; differences are listed per screen.

## Shared decisions for the area

**Edit mode toolbar (proposed, extends [design language §8.1](design-language.md#81-modes-and-their-secondary-toolbars)).** One secondary toolbar for the Edit mode, three groups separated by hairline dividers:

| Group | Controls (tool toggles unless noted) | Command ids |
| --- | --- | --- |
| Tools | Select · Edit text · Add text · Image · Shape ▾ (menu: Line, Rectangle, Ellipse, Polygon) · Link | `edit.select`, `edit.edit-text`, `edit.add-text`, `edit.place-image`, `edit.shape.line` / `.rectangle` / `.ellipse` / `.polygon`, `edit.link` |
| Whole document | Page marks ▾ (menu: Watermark…, Header & footer…, Bates numbers…, Manage page marks…) · Replace… (actions, not toggles) | `page-marks.watermark`, `page-marks.header-footer`, `page-marks.bates`, `page-marks.manage`, `edit.replace` |
| Separate tools | Redact · Protect (open their own toolbars, as the design language says) | `redact.open`, `protect.open` |

A context note at the right end says what the active tool does ("Click to place a text box, or drag to set its width · Esc to finish"). After placing an object the tool returns to **Select**, with the new object selected (as "creating an annotation selects it", [§8.2](design-language.md#82-selection--inspector)).

**Shortcuts (proposed; single keys only while the page has focus and can be turned off, WCAG 2.1.4):**

| Command | macOS | Windows / Linux |
| --- | --- | --- |
| Select tool | V | V |
| Edit text tool | E | E |
| Add text tool | T | T |
| Nudge selection 1 pt / 10 pt | Arrow keys / Shift + arrows | same |
| Delete selection | ⌫ | Delete |
| Undo / redo | ⌘Z / ⇧⌘Z | Ctrl Z / Ctrl Shift Z |
| Cut, copy, paste | ⌘X, ⌘C, ⌘V | Ctrl X, Ctrl C, Ctrl V |
| Find and replace | ⌥⌘F | Desktop app: Ctrl H. Web: none (Ctrl H opens browser history) |
| Edit points of a selected path | ↵ | Enter |

Image, Shape, Link, Page marks, Duplicate and the arrange commands have no shortcut (palette and menus only; `Mod D` collides with bookmarking in the web app, and `⌘[`/`⌘]` with browser back/forward).

**Selection handles (new component, see below).** 8 px white squares with a 1.5 px `accent` border at the corners and edge midpoints, plus a 10 px circular rotation handle 24 px above the top edge on a 1 px stem. The visible square sits in a 24 × 24 hit area. Handles are `aria-hidden`; every handle action has an inspector field (X, Y, W, H, Rotate) and a keyboard path (arrow keys), which is the single-pointer and non-drag alternative (WCAG 2.5.7).

**Undo and save.** Every action on these screens is one op in the op log ([ADR 0006](../../decisions/0006-op-log-and-undo.md)) and one undo step; a replace-all or a page mark on 24 pages is one step too. Nothing writes the file until Save ([§8.4](design-language.md#84-explicit-save)); saves are incremental ([ADR 0007](../../decisions/0007-save-planner.md)). Because an incremental save keeps the earlier bytes, the status bar says so on screens that change or delete existing content ("Saving keeps the earlier version inside the file. To remove content for good, use Redact."). On a signed or certified file, the first content edit raises the signature banner from [§7.13](design-language.md#713-save-note-and-save-banner-signed-file) (content changes are a DocMDP violation at every certification level), before the save, not at it.

**Fonts on web and desktop.** Desktop lists bundled fonts, fonts on the computer (marked "on this Mac" / "on this PC", only those whose licence allows embedding) and "Add a font file…". The web app lists bundled fonts and font files the user adds. Everything else in the area is identical on both.

## A01 Edit mode: add text box

**Purpose.** Add new text anywhere on a page, styled from the inspector, saved as real text with an embedded font subset.

**Entry points.** Edit mode → Add text tool (`edit.add-text`, `T`); palette "Add text"; the More menu's Edit submenu.

**Layout.** Window frame, top bar (Edit active, "1 unsaved change", Save), Edit toolbar with Add text pressed and the context note, left rail (Outline), canvas with page 3, inspector "Text box", status bar.

**Content and controls.**

- **On the page:** a text box with a 1 px `accent` border and two side handles (width only; height follows the text). While typing it has the focus ring. Click places a box at the click point with the page's text width; drag sets the width.
- **Inspector "Text box"** (`Page 3 · New text`):
  - **Font** (select): bundled fonts first, then system fonts (desktop), then "Add a font file…". Help: "Bundled font. Saved into the file as a subset." `text.set-font`
  - **Size (pt)** and **Line spacing** (number inputs). `text.set-size`, `text.set-line-spacing`
  - **Style** (toggle group): Bold, Italic, Underline, Superscript, Subscript. `text.bold`, `text.italic`, `text.underline`, `text.superscript`, `text.subscript`
  - **Alignment** (toggle group): left, centre, right. `text.align-left` / `-centre` / `-right`
  - **Colour** swatches: Ink, Grey, Teal, Red, then "Custom…". These are content colours (see New components), not annotation colours.
  - **Read by screen readers as** (only for tagged files): Paragraph, Heading, Decoration (not read). Help says where the text joins the reading order: "The new text is read after “The board is asked to note…”."
  - **Actions:** "Insert symbol…" (opens a symbol picker; `text.insert-symbol`), "Delete text box" (danger ghost; `edit.delete`).

**States.**

- Tool armed, nothing placed: cursor is a text cursor over the page; context note visible; no inspector.
- Typing (drawn): box focused, inspector open.
- Box selected but not typing: border + handles, no caret; Enter starts typing.
- Empty box on Escape or click away: removed, no op recorded.
- Character the chosen font lacks: lamina falls back to the next font in the chain and shows the same notice as A04 in the inspector.
- Signed file: the signature banner appears with the first character typed.

**Behaviour.** The op `AddText` is recorded when typing ends (Escape, click away, or switching tool), as one undo step; later style changes are separate steps. Saving builds a Form XObject with a subset of the font and a ToUnicode map, so the text stays searchable and copyable. In a tagged file the text gets a structure element and joins the reading order where it sits on the page.

**Accessibility.** Focus order: toolbar → page → inspector. After placing, focus is in the box; Escape returns focus to the page with the box selected; Tab from the page reaches the inspector. Screen readers hear "New text box on page 3, edit text". Placing without a pointer: with the page focused, Enter on the Add text tool places a box at the top of the visible page area; arrow keys move it.

**Platform differences.** Web: no window frame; font list without system fonts. macOS/Windows/Linux: identical apart from shortcut chips and system font discovery (CoreText, DirectWrite, fontconfig at the edge). Phone and tablet: out of scope for this round.

**Artboards.** `A01-add-text.dc.html`

**Todos.** [E6c](../../todos/ready/E6c-add-text.md), [E6e](../../todos/ready/E6e-content-tools-ui.md)

## A02 Place image and shapes

**Purpose.** Put images, logos and shapes on a page as page content (not annotations), then move, resize, rotate, crop and fade them.

**Entry points.** Image tool (`edit.place-image`: opens the file picker); drop an image file on a page; paste an image (desktop also from the OS clipboard); Shape ▾ menu (`edit.shape.*`); palette "Place image…", "Draw rectangle".

**Layout.** Same shell as A01 with Select pressed (the tool returns to Select after placing), the placed logo selected on page 3, a snap guide, a previously drawn rectangle around the West bar, and the inspector "Image".

**Content and controls.**

- **On the page:** the image with eight handles and a rotation handle. A dashed `accent` snap guide with a small label ("Right margin") appears while dragging near margins, page centre and other objects' edges. Shapes draw by drag (Shift keeps squares, circles and 45° lines); a polygon adds a point per click and closes on Enter or a click on the first point.
- **Inspector "Image"** (`Page 3 · Added by you`):
  - File line: "northwind-logo.png · 640 × 196 px · PNG with transparency" and **Replace…** (`object.replace-image`).
  - **Position and size (pt):** X, Y, W, H inputs; **Keep proportions** toggle (pressed by default for images); **Rotate** input. `object.move`, `object.resize`, `object.rotate`
  - **Opacity:** slider plus value (85%). `object.set-opacity`
  - **Arrange:** Crop… (`object.crop`: crop handles replace the resize handles until Enter or Escape), Bring forward, Send backward (`object.bring-forward`, `object.send-backward`).
  - **Alternative text** and "Decoration, screen readers skip it" (tagged files).
  - **Actions:** Duplicate (`object.duplicate`), Delete image (`edit.delete`).
- **Inspector "Shape"** (not drawn): Position and size, Rotate, Fill (content colours + None + Custom), Stroke (None, Solid, Dashed), Width, Opacity, Arrange, Delete shape.

**States.**

- Unsupported file (for example HEIC on the web): no object exists yet to show an inline error on, so a small dialog says "lamina can’t place this file" and names the formats it takes (JPEG, PNG; desktop also converts others it can decode), with OK.
- Very large image: placed at the page's text width, with a note in the inspector "Shown smaller than its full size; the full image is saved."
- Picker cancelled: nothing changes.
- Signed file: signature banner on placing.

**Behaviour.** `AddContentObject` on place; `UpdateContentObject` for each finished move, resize, rotate, crop or opacity change (a drag is one step, not one per frame); `DeleteContentObject` on delete. Each object is its own Form XObject so it can be changed or removed later without touching the page's original content. The page re-renders after each finished change (the touched-page reload in [E6f](../../todos/ready/E6f-reload-budget.md)); during a drag only the overlay moves.

**Accessibility.** The image on the page is announced as "Placed image: Northwind Housing Trust logo" (its alt text). Arrow keys nudge, Shift + arrows by 10 pt; every handle action has an inspector field. Opacity is a native range input with its value shown as text. The focus ring is drawn on the Opacity slider.

**Platform differences.** Desktop: paste from the OS clipboard and drag from Finder/Explorer. Web: picker, drop and clipboard paste where the browser allows it.

**Artboards.** `A02-image-and-shapes.dc.html`

**Todos.** [E6b](../../todos/ready/E6b-place-images-and-shapes.md)

## A03 Watermark, header and footer dialog

**Purpose.** Stamp a watermark or background, a header and footer with page tokens, or Bates numbers on a page range, with a live preview, as page content.

**Entry points.** Edit toolbar → Page marks ▾ → Watermark… / Header & footer… / Bates numbers… (`page-marks.watermark`, `page-marks.header-footer`, `page-marks.bates`); palette "Add watermark", "Add page numbers". "Manage page marks…" (`page-marks.manage`) lists marks lamina added, to edit or remove them.

**Layout.** One **Page marks** dialog (wide form dialog, see New components) over the dimmed Edit-mode shell: title and subtitle ("Stamped onto the pages as content, not as comments"), close button, tabs (Watermark · Header & footer · Bates numbers), a form column, a preview column, and a footer with a summary line and Cancel + the primary action.

**Content and controls.**

- **Watermark tab** (left artboard):
  - **Use:** Text / Image… (radio).
  - **Text** input ("Draft · for board review"; focus ring drawn here).
  - **Font**, **Size** (Fit to page, 48 pt, 72 pt, custom), **Colour** (Grey, Red, Teal, Ink), **Opacity** slider (20%), **Angle** (Diagonal rising, Horizontal, Diagonal falling), **Layer** (In front of the content / Behind the content; behind = a background).
  - **Pages:** All 24 pages / Pages [range] (accepts "1–3, 7", page labels too).
  - **Readers can hide it (put it on its own layer)**: writes an optional content group.
  - Preview: a small page with the mark, a page stepper ("Page 3 of 24"), and the note "Marked as decoration in this tagged file, so screen readers skip it."
  - Footer: "Adds one watermark to 24 pages. Change or remove it later from Page marks." · Cancel · **Add watermark**.
- **Header & footer tab** (right artboard):
  - A 2 × 3 grid of slots (Header/Footer × Left/Centre/Right), each a text input labelled by its row and column.
  - **Insert into the slot:** Page, Pages, Page label, Date, File name. Each inserts a token shown in square brackets ("[Page] of [Pages]") at the caret of the last focused slot.
  - **Size (pt)**, **Top margin**, **Bottom margin**, **Date** format, **Pages** From/to.
  - **Overlap notice** (warning): lamina looks for existing text in the header and footer bands and says so ("These pages already have a footer. “Page 3 of 24” sits where yours would go."). The preview shows new marks in `accent` and existing ones in grey.
  - Footer: "Adds a header and footer to pages 2–24 (23 pages)." · Cancel · **Add header and footer**.
- **Bates numbers tab** (not drawn; same layout): Prefix, Start number, Digits (zero-padded), Suffix, Slot (one of the six), Pages; a preview of the first and last number ("NHT-000001 … NHT-000024"). In a batch (E14d) the same settings run across files, continuing the count.

**States.**

- Invalid page range: the Pages input turns `danger` with "Pages 1–30: this file has 24 pages." and the primary button is disabled.
- Empty text: primary disabled with the reason as help text.
- Editing an existing mark (from Manage page marks): title "Edit watermark", primary "Update watermark", plus a ghost "Remove watermark".
- Applying to many pages: the primary shows a determinate progress bar in the footer ("Adding to pages… 62%") and stays a single undo step.
- Signed file: the footer line adds "Saving will invalidate the signature" in `warning`, matching the banner.

**Behaviour.** Primary records `AddWatermark` / `AddBackground` / `SetHeaderFooter` / `ApplyBates` as one op and closes the dialog; focus returns to the Page marks button; a toast confirms ("Watermark added to 24 pages · Undo"). Marks are Form XObjects, marked as artifacts in tagged files; tokens are expanded per page at apply time (page label, number, total, date from the dialog, not the clock at save). Removal ops only remove marks lamina added.

**Accessibility.** `role="dialog"`, `aria-modal`, labelled by its title; tabs are a tablist with arrow-key movement; the preview is `role="img"` labelled "Preview" and described by its note; the preview stepper announces the page (`aria-live="polite"`). Escape cancels. Swatches carry names.

**Platform differences.** Identical on web and desktop (desktop lists system fonts in Font). Phone (later): the dialog becomes a large-detent sheet with the preview above the form.

**Artboards.** `A03-watermark.dc.html`, `A03-header-footer.dc.html`

**Todos.** [E6d](../../todos/ready/E6d-watermarks-headers-footers.md)

## A04 Edit existing text inline

**Purpose.** Edit a paragraph that is already in the PDF, in place, keeping the embedded font where it has the glyphs, and saying plainly when it doesn't or when the text can't be edited at all.

**Entry points.** Edit text tool (`edit.edit-text`, `E`), then click a paragraph; double-click text with the Select tool; palette "Edit text".

**Layout.** Same shell; Edit text pressed; context note "Click a paragraph to edit it · Dashed outlines show editable text". Every detected text run on the page has a 1 px dashed `control-border` outline (3.65:1 on white); the paragraph being edited has the 2 px `accent` focus outline and a caret. Inspector "Paragraph".

**Content and controls.**

- **On the page:** the paragraph is a text editor. The character drawn with another font has a 2 px dotted `warning` underline (in the editor only, never saved).
- **Inspector "Paragraph"** (`Page 3 · Repairs backlog · 1 of 3`):
  - **Substitution notice** (warning, only when needed): "1 character uses another font. The font in this file has no “č”, and its full version isn’t available. lamina draws it in Source Serif 4, the closest bundled match, and marks it with a dotted underline here." Choices: **Substitute only missing characters** (default) / **Set the paragraph in Source Serif 4**; link-button **Choose another font…**. `text.substitute-missing`, `text.set-paragraph-font`
  - **Font in the file:** name and "Embedded subset · has the other characters you typed".
  - Size, Line spacing, Style toggles, Alignment, Colour (as A01; changing them re-lays out the edited lines).
  - **Actions:** Done (`edit.finish`), Revert paragraph (`text.revert-run`).
- **Status bar:** "Editing existing text is best-effort. Check the page before you save."

**Font resolution order** (from E7e/E7f, shown to the user only when it reaches step 3 or 4): 1. the embedded subset has every glyph → re-encode with it; 2. the full font is available (embedded in full, or the same font on the computer) → add a new subset; 3. a metrics-matched substitute for missing characters only; 4. the user picks a font.

**States.**

- Editing with the original font (no notice): inspector without the warning block.
- Substitution (drawn, left).
- **Can't edit in place** (right artboard): the clicked text is outlined with the focus outline and the inspector "Chart labels" says "This text can’t be edited in place. Its letters are drawn as shapes with no record of which characters they are, so lamina can’t retype them in the same font." It offers: **Select as shapes** (switches to Select with them selected), **Delete and add new text** (one step: delete the run, open an A01 text box in its place), **Redact…** (to remove what it says for good). A "Details" line names the cause ("Type 3 font without a text map · 8 labels"). Other causes with their own sentence: text inside an image (scanned page; suggests OCR), text in a form field (suggests Fill & Sign), text in an annotation (suggests Comment mode).
- Reverted: the paragraph returns to the file's original bytes; the op is dropped.

**Behaviour.** Typing updates the overlay at once; `EditText` is recorded when editing ends (Done, Escape, click away), one step per paragraph session; style changes are `SetTextStyle` steps. Following glyphs on the same line are repositioned; lines don't reflow across the paragraph unless alignment or spacing changes. In tagged files the paragraph keeps its structure element and its `/ActualText` is updated (E7i).

**Accessibility.** The paragraph editor is a `textbox` (multi-line) named "Paragraph 1, editing" and described by the substitution notice, so screen readers hear the warning when it appears. Tab cycles editable runs on the page in reading order (Edit text tool), Enter starts editing, Escape ends it. The dashed outlines and dotted underline are not the only signal: the inspector states both.

**Platform differences.** Desktop can resolve step 2 with fonts on the computer; the web can only use embedded, bundled or user-added fonts, so substitution is more common there and the notice says "isn’t available in the browser".

**Artboards.** `A04-inline-edit.dc.html`, `A04-not-editable.dc.html`

**Todos.** [E7e](../../todos/ready/E7e-text-runs-and-inline-edit.md), [E7f](../../todos/ready/E7f-font-handling-and-styling.md)

## A05 Select and transform existing objects

**Purpose.** Select images and vector paths that were already in the file and move, resize, rotate, restyle, reorder, clip or delete them.

**Entry points.** Select tool (`edit.select`, `V`), click an object; Shift-click adds to the selection; double-click selects inside a group; right-click or Shift+F10 opens the context menu.

**Layout.** Same shell; Select pressed; the chart's four bars selected as one group with eight handles; a context menu open beside them (menus may float); inspector "4 shapes".

**Content and controls.**

- **Context menu** (new component): Cut ⌘X · Copy ⌘C · Duplicate │ Bring to front · Bring forward · Send backward · Send to back │ Edit points ↵ · Release clipping · Replace image… (disabled for paths) │ Delete shapes ⌫. Command ids `object.cut`, `object.copy`, `object.duplicate`, `object.bring-to-front`, `object.bring-forward`, `object.send-backward`, `object.send-to-back`, `path.edit-points`, `object.release-clip`, `object.replace-image`, `edit.delete`.
- **Inspector "4 shapes"** (`Page 3 · Part of the original page`):
  - Position and size (pt): X, Y, W, H.
  - **Fill** swatches with the original colour first and labelled "(original)", Teal selected, Custom….
  - **Stroke** (None, Solid, Dashed) and **Width** (disabled while Stroke is None).
  - **Shape and clipping:** Edit points (Bézier point handles, E7h), Release clipping, with the help "Clipped to the chart area by the original file."
  - **Actions:** Revert to original (`object.revert`), Delete shapes.
- For an **image** the inspector matches A02's, plus, when the image is used on several pages, **Apply to:** This page only / All 24 pages that use it (copy-on-write is the default: "This page only").
- **Status bar:** "Saving keeps the earlier version inside the file. To remove content for good, use Redact."

**States.**

- Nothing selected: hovering an object shows a 1 px `accent` outline; the context note explains the clicks.
- Shared image (described above).
- Object lamina can't change safely (inline image inside a text block, shading, content in a Type 3 glyph): selected with the outline but no handles; the inspector says why and offers Delete only if deletion is safe.
- Locked by the file (content inside a signed field's appearance): read-only inspector naming the signature.
- Edit points (E7h): anchor points as 8 px squares and control points as 6 px circles; Enter/Escape leave.

**Behaviour.** Each finished change is one `EditContent` op with one `ContentEdit` (Transform, Delete, ReplaceImage, Reorder, Clip, EditPath, SetClip, SetMask); multi-selection edits are one op. Revert to original drops this object's edits. The page re-renders after each change.

**Accessibility.** The focus ring is drawn on the active context-menu item (`role="menu"`, arrow keys, Escape closes and returns focus to the selection). Tab moves between objects on the page in content order with the Select tool; arrow keys nudge; all handle actions have inspector fields. Selected objects are announced by kind and count ("4 shapes selected, part of the original page").

**Platform differences.** Windows/Linux show Ctrl X, Ctrl C, Delete in the menu; the menu is the same component on web and desktop (the desktop app does not use the OS context menu, so labels and enabled states come from the command registry).

**Artboards.** `A05-select-transform.dc.html`

**Todos.** [E7d](../../todos/ready/E7d-select-transform-existing.md), [E7h](../../todos/ready/E7h-vector-clipping-masks.md)

## A06 Search and replace across the document

**Purpose.** Replace text across the whole document with a preview, a per-match choice, and a report of matches that could not be replaced.

**Entry points.** Edit toolbar → Replace… (`edit.replace`); ⌥⌘F (desktop Ctrl H); palette "Find and replace"; from the find bar (R05) a "Replace" disclosure when the file is editable.

**Layout.** While it is open, the **find-and-replace bar** takes the secondary toolbar's place (one toolbar at a time, [§8.1](design-language.md#81-modes-and-their-secondary-toolbars)); closing it brings the Edit tools back. The left rail switches to a **Results** tab (300 px; the rail is resizable). No inspector.

**Content and controls.**

- **Bar, row 1:** Find input · status "Match 1 of 6 · on 5 pages" · Previous / Next match · Match case, Whole words (toggles) · Close. `find.next`, `find.previous`, `find.match-case`, `find.whole-words`
- **Bar, row 2:** Replace with input (focus ring drawn) · **Replace** (this match, then go to next; `edit.replace-one`) · **Skip** (uncheck and go to next) · **Replace 5 checked** (primary; `edit.replace-checked`) · "One undo step for all of them."
- **Results tab:** "Replace all matches" checkbox (mixed when some are unchecked) with "5 of 6"; matches grouped by outline section with page numbers; each match is a checkbox row showing the context with the old words struck through and the new words in `accent`. Unchecked rows say "Skipped: stays as it is". The current match has the selected-row style. Clicking a row goes to the match.
- **On the page:** the current match in `selection-tint` with the 2 px `accent` outline.

**States.**

- No matches: status "No matches"; Replace buttons disabled.
- **Report after replacing** (right artboard): the bar shows "2 matches left · on 2 pages" and disabled Replace buttons; the Results tab shows a warning notice "4 of 5 replaced. 1 match couldn’t be replaced", then each outcome with a badge: **Not replaced** (warning) with its reason and "Go to page 9", **Skipped by you** (neutral), **Replaced · 4** (positive) with the new wording and "All four kept the font in the file; no substitutes were needed." A toast "4 matches replaced · Undo".
- Reasons a match can't be replaced, each with its own sentence: hidden OCR text over a scanned image (changing it wouldn't change what the page shows); text that can't be edited in place (A04 reasons); text in a form field or annotation (suggest Fill & Sign or Comment); the replacement needs a font and none is acceptable (offer "Allow substitutes and try again").
- Some replacements used substitutes: the Replaced group lists them with the A04 dotted underline note.

**Behaviour.** One `ReplaceText` op over the checked matches, applied with the A04 editing path match by match; failures don't stop the others and are listed; Undo reverts all of them. The find input stays editable; changing it rebuilds the results and clears the report.

**Accessibility.** The bar is a `search` landmark; the match status is a live region; results are real checkboxes with the match context as label; the report notice is `role="status"`. Keyboard: Enter in Find = next match; Enter in Replace with = Replace; ⌥↵ / Alt Enter = Replace checked (proposed).

**Platform differences.** Shortcut only (see the shortcut table). Phone (later): the bar becomes a sheet; results as a list in it.

**Artboards.** `A06-replace.dc.html`, `A06-report.dc.html`

**Todos.** [E7g](../../todos/ready/E7g-search-and-replace.md)

## A07 Structure and conformance warning

**Purpose.** Keep a tagged file's structure valid while editing, and tell the user, before saving, when an edit would break a PDF/A or PDF/UA claim the file makes.

**Entry points.** Automatic: entering Edit mode on a tagged or PDF/A file; any edit that creates an issue; Save. The status-bar claim badges and the banner's Details open the check panel.

**Layout (left artboard).** Edit-mode shell with:

- A **warning banner** under the Edit toolbar (the [§7.13](design-language.md#713-save-note-and-save-banner-signed-file) banner pattern with the structure icon): "**Saving would break the PDF/UA claim.** The image you added on page 3 has no alternative text. The tags are otherwise up to date." · Add alternative text · Details.
- Left rail **Tags** tab (proposed): the reading order of the current page as a tree (Section › Heading 2, Figure, Paragraph ×4, Figure; Decoration last), the new items marked "· new", the faulty one selected with a warning icon and "No alt text".
- Inspector "Image" with **Alternative text** in the error state (`danger` border, `aria-invalid`, error "Needed to keep the PDF/UA claim.", focus ring) and the Decoration checkbox; **This file’s claims** list with badges (PDF/A-2b "Still holds", PDF/UA-1 "1 issue").
- Status bar: claim badges ("PDF/A-2b", "PDF/UA-1 · 1 issue") as one button that opens the check panel.

**Save check dialog (right artboard).** On Save while a claim fails: "Save with one accessibility issue?" lists each claim with a badge and one line (PDF/A-2b Still holds; PDF/UA-1 1 issue + "Go to the image"; Tags Updated: "2 added on page 3 (a paragraph and a figure), in reading order"). It explains: "If you save now, lamina removes the PDF/UA-1 claim from the file, so it doesn’t promise what it no longer meets. Fix the issue and save again to put the claim back." Actions: **Save without the claim** (secondary, focus ring drawn) and **Fix it first** (primary: closes, selects the image, focuses Alternative text). Close (X) = keep editing without saving.

**States.**

- **Info, on entering Edit mode** (not drawn): a neutral (`surface`) banner, "This file claims PDF/A-2b and PDF/UA-1. lamina keeps the tags up to date as you edit and checks both claims when you save." with Dismiss; it doesn't come back for this file in this session.
- Issue found (drawn). The banner appears with the edit that causes it and disappears when the issue is fixed or the edit undone.
- All checks pass at save: no dialog; the save completes and a toast says "Saved · PDF/A-2b and PDF/UA-1 still hold".
- A PDF/A failure (for example a font that can't be embedded): same dialog, the PDF/A row in warning; "Save without the claim" removes the PDF/A identification; for PDF/A the dialog also offers "Save a copy instead".
- Untagged file: none of this appears; A01's "Read by screen readers as" field is hidden.

**Behaviour.** Structure maintenance runs inside each op (MCIDs, ParentTree, `/ActualText`, `/Alt`; deleting a tagged run removes its element). Quick checks run after each op; the full PDF/A and PDF/UA profile check runs on Save. Removing a claim is part of the save, not a separate undo step. Proposed: lamina never saves a claim it has just found false; whether "save and keep the claim" should exist for expert users is an open question for the E7i PR.

**Accessibility.** Banner `role="status"`; the dialog is a labelled modal described by its first paragraph; the Tags tree uses `role="tree"` with levels; badges pair colour with words and icons.

**Platform differences.** Identical on web and desktop.

**Artboards.** `A07-tagged-edit.dc.html`, `A07-save-check.dc.html`

**Todos.** [E7i](../../todos/ready/E7i-structure-and-conformance-maintenance.md)

## New components

Proposed for the design language; built only from its tokens.

| Component | Spec | Used in |
| --- | --- | --- |
| **Selection handles** | 8 × 8 white squares, 1.5 px `accent` border (7.58:1 on white), at corners and edge midpoints, 4 px outside a 1 px `accent` outline; 24 × 24 hit area. Rotation handle: 10 px white circle, same border, on a 1 px × 19 px stem 24–30 px above the top edge. Text boxes show only the two side handles. `aria-hidden`. | A01, A02, A05, A07 |
| **Snap guide** | 1 px dashed `accent` line across the page, with a label chip (11 px white on `accent`, radius 3, padding 1 × 5). Shown only during a drag. | A02 |
| **Editable-run outline** | 1 px dashed `control-border`, offset 3 px, on every editable text run while the Edit text tool is active; the run being edited gets the 2 px `accent` focus outline. | A04 |
| **Substituted-glyph mark** | 2 px dotted `warning` underline, offset 3 px, in the editor only (never saved). | A04, A06 |
| **Toggle group** | Joined buttons 30 × 28 in one `control-border` box (radius 4), `hairline` dividers, pressed = `accent-tint` fill + `accent` glyph; `role="group"` with a visible label, each button `aria-pressed` and `aria-label`. | A01, A04 |
| **Content colour swatches** | Same circles as §7.12, for page content: Ink `#262421`, Grey `#6E6A64`, Teal `#1F5C63`, Red `#9B2335`, then "Custom…". Separate from the four annotation colours, which keep their meaning. The original colour of an existing object is listed first with "(original)" in its name. | A01, A02, A03, A04, A05 |
| **Wide form dialog** | §7.18 dialog at `max-width: 760px` with a header (title + 12 px subtitle + close), a tablist, a body that wraps into a form column (`flex: 1 1 320px`) and a preview column (`flex: 0 1 280px`), and a footer row (summary 12 `muted` left; actions right) over a `hairline`. | A03 |
| **Live preview** | A 232 × 300 page drawn in thumbnail style on a `gutter` well (radius 4, padding 16), with the new marks in their real colours (teal for new header/footer text, existing text in grey), a page stepper (two 26 px icon buttons around "Page 3 of 24", live region), `role="img"` with a describing note. | A03 |
| **Context menu** | `surface`, `chip-border` edge, radius 6, padding 4, menu shadow (`0 4px 14px rgba(60,48,30,0.14)`), 13 px. Items 30 high, padding `0 10px`, radius 4, shortcut 11 px `muted` right-aligned; separators 1 px `hairline` with 4 × 6 margins; active item = palette active row (`accent-tint` + inset `accent` ring); disabled = `disabled` text + `aria-disabled`; destructive item in `danger`. `role="menu"` / `menuitem`. | A05 |
| **Find-and-replace bar** | Two rows in a `surface` strip (padding 10 × 16, gap 8) under the top bar, replacing the secondary toolbar while open; labels 92 px wide, inputs 280 px; `role="search"`. | A06 |
| **Result row with change preview** | A checkbox row (padding 8, radius 4, 13/1.45) whose label shows the context with the old words struck through in `muted` and the new words 600 in `accent`; grouped under Newsreader 15 section headings with page numbers. | A06 |
| **Outcome badges** | §7.23 badges used as report states: "Replaced · n" positive with the check icon, "Not replaced" warning with the warning icon, "Skipped by you" neutral. Each sits above the match and a one-sentence reason. | A06 |
| **Structure tree rows** | Rows 13 px, padding 5 × 8, tag name in a 64 px `muted` column then the content excerpt (ellipsis); levels indent 14 px; selected = `accent-tint` + inset `accent` ring; a problem shows the warning icon and its short name in `warning` 600. `role="tree"` / `treeitem`. | A07 |
| **Claim badges** | §7.23 badges naming a standard ("PDF/A-2b", "PDF/UA-1 · 1 issue") in the status bar, as one button that opens the check panel. | A07 |

New icons (24 grid, 1.5 stroke, add to §5 when accepted):

| Name | Markup |
| --- | --- |
| Select (pointer) | `<path d="M6.5 4.5l11 6.5-5 1.5-2.5 5z"></path>` |
| Edit text | `<path d="M4 7.5V5.5h10v2M9 5.5V18.5M7 18.5h4"></path><path d="M18 8.5v11M16.5 8.5h3M16.5 19.5h3"></path>` |
| Add text | `<path d="M5 7V5h14v2M12 5v14M9.5 19h5"></path>` |
| Image | `<rect x="4" y="5" width="16" height="14" rx="1.5"></rect><circle cx="9" cy="10" r="1.5"></circle><path d="M4.5 17l5-5 4 4 2.5-2.5 3.5 3.5"></path>` |
| Link | `<path d="M10 14a4 4 0 0 0 5.7 0l2.8-2.8a4 4 0 0 0-5.7-5.7l-1.3 1.3"></path><path d="M14 10a4 4 0 0 0-5.7 0l-2.8 2.8a4 4 0 0 0 5.7 5.7l1.3-1.3"></path>` (also "Keep proportions") |
| Page marks | `<path d="M6 3.5h8.5L18 7v13.5H6z"></path><path d="M9 16.5l6-6"></path>` |
| Replace | `<path d="M5 9h12l-3-3M19 15H7l3 3"></path>` |
| Redact | `<rect x="4" y="9.5" width="16" height="5"></rect><path d="M4 5.5h9M4 18.5h12"></path>` |
| Protect (lock) | `<rect x="5.5" y="10.5" width="13" height="9" rx="1.5"></rect><path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5"></path>` |
| Align left / centre / right | `<path d="M5 6h14M5 10h9M5 14h14M5 18h9"></path>` / `<path d="M5 6h14M7.5 10h9M5 14h14M7.5 18h9"></path>` / `<path d="M5 6h14M10 10h9M5 14h14M10 18h9"></path>` |
| Text not editable | Edit text's T plus `<path d="M15 14l5 5M20 14l-5 5"></path>` |
| Structure (tags) | `<path d="M5 5h6v4H5zM13 15h6v4h-6zM5 15h6v4H5z"></path><path d="M8 9v6M8 12h8v3"></path>` |
| Chevron up / right | `<path d="M7 14l5-5 5 5"></path>` / `<path d="M9 5l7 7-7 7"></path>` |
