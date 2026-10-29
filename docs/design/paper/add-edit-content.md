# Paper: add and edit content

This area covers screens A01–A07 of the [screen inventory](screens.md#a-add-and-edit-content-e6e7):

- adding text, images, shapes and page marks to a PDF;
- editing what is already on the page (text, text blocks, images, vector paths);
- the warnings that keep editing honest: font substitution, text that can't be edited in place, replace-all failures, and tagged or PDF/A files whose claims an edit could break.

The work it designs is planned in [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) and [§E7](../../plans/full-editor.md#e7-editing-existing-content).

- **Canvas:** [lamina Paper — Add and edit content](https://claude.ai/artifact/5CecE9xXiw2RXoVde7xEsL) has one row per screen and a sticky note per row. The canvas is republished from the files after review; until then the files are the source of truth.
- **Look:** the [Paper design language](design-language.md). Every board copies the foundation Desktop board's window frame, top bar, rail, inspector and status bar. Dialogs sit over the full shell (top bar, toolbar, rail, page and status bar), dimmed by the scrim, `aria-hidden` and taken out of the tab order.
- **Platform:** all seven screens are **D** (desktop first). The web app uses the same layout without the window frame; the differences are listed per screen.

## Shared decisions for the area

**Top bar.** Every board uses the Editor top bar ([design language §7.1](design-language.md#71-top-bar)):

- the sidebar toggle, the file name, then the save state, then a divider and Undo and Redo (28 px icon buttons in a `History` group);
- the mode switcher, with Edit pressed;
- the page input, zoom −/%/+, Find, View (view settings, `aria-haspopup="dialog"`) and Commands (⌘K);
- Save, shown only when there are changes;
- More (`⋯`).

Undo and Redo name the step they undo ("Undo move image") and are disabled (`aria-disabled`, `disabled` colour) with no history. The save count goes up when an op is recorded, not while you type in a text box. So A01 and A04 show "No unsaved changes" mid-typing; their Undo inside the box is the text field's own undo.

**Edit mode toolbar.** One toolbar, identical on every Edit board (the [§8.1](design-language.md#81-modes-and-their-secondary-toolbars) Edit toolbar). Only the pressed tool and the context note change from board to board. While find and replace is open (A06) the find bar takes its place, so there is still one secondary toolbar:

```
Select · Edit text · Add text · Image · Shape ▾ · Link │ Page marks ▾ · Replace… │ Redact · Protect · Tools ▾
```

| Control | What it is | Command ids |
| --- | --- | --- |
| Select, Edit text, Add text, Image, Link | Tool toggles: a 16 px icon and a label, `aria-pressed` | `tool.select`, `tool.edit-text`, `tool.add-text`, `tool.image`, `tool.link` |
| Shape ▾ | Split tool button ([§7.24](design-language.md#724-secondary-toolbar-tool-toggle)). The main part is a toggle (`aria-pressed`) for the last-used shape, labelled "Shape" with that shape's icon. The 20 × 28 chevron part ("Choose shape", `aria-haspopup="menu"`) opens Line, Rectangle, Ellipse, Polygon | `tool.shape` (with a `kind` argument) |
| Page marks ▾ | Menu button: Watermark…, Header & footer…, Bates numbers…, Manage page marks… | `edit.add-watermark`, `edit.add-header-footer`, `edit.add-bates`, `edit.manage-page-marks` |
| Replace… | Opens the find bar with the Replace row; pressed style while open | `find.open-replace` |
| Redact · Protect | Tool buttons that stay visible on the toolbar (principle 3: every action has a visible home). Each replaces this toolbar with its own tool toolbar that ends in "Done" | `redact.open`, `security.protect` |
| Tools ▾ | Menu: Form design, Actions, Accessibility, Print production. Each replaces this toolbar with its own tool toolbar that ends in "Done" | `form-design.open`, `actions.open`, `a11y.check`, `print.open` |

A context note at the right end gives the active tool's hint, for example "Click to place a text box, or drag to set its width · Esc to finish". After placing an object the tool returns to Select, with the new object selected (as in [§8.2](design-language.md#82-selection--inspector)).

**Rail** ([§8.10](design-language.md#810-rail-tab-set-proposed)). Edit mode shows `Pages · Outline · Attachments`, with Attachments always shown in Edit mode, in a 220 px rail. Once the session has a step in its history, History joins in a "More tabs" menu button after the third tab, and the rail is 280 px. While find is open, `Results` takes the third visible place and Attachments moves into More tabs. On the boards: A01, A03 watermark, A04 and A05 text block have no history yet (220 px); the others show More tabs (280 px).

**Status bar.**

- **Left:** the network indicator.
- **Middle:** at most one fixed note, by priority:
  1. the signed-file note or a constraint note;
  2. the repaired-file notice;
  3. the history note, only once a delete, replace or recolour of original content is pending: "Earlier versions stay inside the file until you choose Save as optimised. To remove what text or images say for good, use Redact.";
  4. nothing.
- **Right:** the page label; on A07, also the claim badges.

Boards in this area use the history note on A05, A05-edit-points and A06-report.

**Signed files** (ADR 0007).

- **Approval signatures only (no certification):** these files are `AppendOnly`. Edits save incrementally after the signature and the signatures stay valid. There is no banner; the status bar shows "Signed file. Saving adds your changes after the signature; the signature stays valid."
- **Conflicts:** the constraint banner appears with the first edit that conflicts, names each signature, and offers "Save a copy" and "Apply anyway…". "Apply anyway…" opens an alertdialog listing the signers; confirming maps to `invalidate_signatures`. Conflicts are:
  - certified files where the change isn't allowed: all content edits in this area are a DocMDP violation at P1, P2 and P3;
  - edits to FieldMDP or `/Lock`ed field content;
  - actions that need a full rewrite.
- **Certified "no changes" (P1):** the banner shows on entering Edit mode.

**Shortcuts.** Single keys work only in Edit mode with the page focused, never switch mode, and can be turned off in Settings › Shortcuts.

| Command | macOS | Windows / Linux |
| --- | --- | --- |
| Select tool | V | V |
| Edit text tool | E | E |
| Add text tool | T | T |
| Nudge selection 1 pt / 10 pt | Arrow keys / Shift + arrows | same |
| Delete selection | ⌫ | Delete |
| Undo / redo | ⌘Z / ⇧⌘Z | Ctrl Z / Ctrl Shift Z |
| Cut, copy, paste | ⌘X, ⌘C, ⌘V (`edit.cut`, `edit.copy`, `edit.paste`) | Ctrl X, Ctrl C, Ctrl V |
| Find and replace | Desktop app ⌥⌘F; web: none (Chrome and Safari use ⌥⌘F) | Desktop app Ctrl H; web: none (Ctrl H opens browser history) |
| Edit points of a selected path | ↵ | Enter |

Image, Shape, Link, Page marks, Duplicate and the arrange commands have no shortcut; they are in the palette and menus only. `Mod D` would collide with bookmarking in the web app, and `⌘[` / `⌘]` with browser back and forward.

**Selection handles** (see [design language §7.32](design-language.md#732-page-overlays-proposed)):

- 8 px white squares with a 1.5 px `accent` border, at the corners and edge midpoints;
- a 10 px circular rotation handle 24 px above the top edge, on a 1 px stem;
- each visible square sits in a 24 × 24 hit area.

Handles are `aria-hidden`. Every handle action also has an inspector field (X, Y, W, H, Rotate, crop insets) and a keyboard path (arrow keys), which is the single-pointer, non-drag alternative (WCAG 2.5.7). Drawing tools also work by click: a click places a default-size object.

**Undo and save.**

- Every action here is one op in the op log ([ADR 0006](../../decisions/0006-op-log-and-undo.md)) and one undo step. A replace-all or a page mark on 24 pages is also one step.
- Nothing writes the file until Save ([§8.4](design-language.md#84-explicit-save)). Saves are incremental ([ADR 0007](../../decisions/0007-save-planner.md)), so earlier content stays in the file; the history note says so.

**Fonts on web and desktop.**

- **Desktop** lists bundled fonts, then fonts on the computer ("on this Mac" or "on this PC"), then "Add a font file…".
- **Web** lists bundled fonts and font files the user adds.
- A font whose `fsType` forbids embedding is listed but disabled, with the reason "This font’s licence doesn’t allow saving it into a PDF". A file added with such a font is refused with the coded error from E6c.

Everything else in the area is identical on web and desktop.

## A01 Edit mode: add text box

**Purpose.** Add new text anywhere on a page, styled from the inspector, and save it as real text with an embedded font subset.

**Entry points.** Edit mode → Add text tool (`tool.add-text`, `T`); palette "Add text"; the Edit menu.

**Layout.** The full shell, with:

- the top bar showing "No unsaved changes" while the first box is being typed, with Undo and Redo disabled and no Save;
- the Edit toolbar with Add text pressed and its context note;
- the rail (`Pages · Outline · Attachments`, Outline selected, 220 px);
- the canvas with page 3;
- the inspector "Text box";
- the status bar with no middle note.

**Content and controls.**

- **On the page:** a text box with a 1 px `accent` border and two side handles. Only the width is set by handles; the height follows the text. While you type, the box has the focus ring. A click places a box at the click point with the page's text width; a drag sets the width.
- **Inspector "Text box"** (`Page 3 · New text`):
  - **Font** (select): bundled, then system (desktop), then "Add a font file…". Help: "Bundled font. Saved into the file as a subset." `edit.set-font`
  - **Size (pt)** and **Line spacing**. `edit.set-font-size`, `edit.set-line-spacing`
  - **Style** (toggle group): Bold, Italic, Underline, Superscript, Subscript. `edit.toggle-bold`, `edit.toggle-italic`, `edit.toggle-underline`, `edit.toggle-superscript`, `edit.toggle-subscript`
  - **Alignment** (toggle group): left, centre, right. `edit.align-text`
  - **Colour:** Ink, Grey, Teal, Red, then "Custom…" (content colours, not annotation colours).
  - **Read by screen readers as** (tagged files only): Paragraph; Heading 3 (under “2. Repairs backlog”), the default heading level, one below the section's heading; Heading 1–6; Decoration (not read). Help says where the text joins the reading order ("…joins the reading order after “The board is asked to note…”"). `edit.set-structure-role`
  - **Actions:** "Insert symbol…" (`edit.insert-symbol`) and "Delete text box" (danger ghost; `edit.delete`).

**States.**

- **Tool armed:** text cursor over the page, context note shown, no inspector.
- **Typing** (drawn): the box is focused and the inspector is open. The save count changes when typing ends.
- **Box selected, not typing:** border and handles, no caret; Enter starts typing.
- **Empty box on Escape or click away:** removed, no op recorded.
- **Character the chosen font lacks:** lamina falls back down the chain and shows the A04 notice.
- **Synthetic styles:** if the chosen family has no bold or italic face, lamina draws one synthetically and the Style help says "No bold face in this font; lamina thickens the regular one".
- **Signed file:** as in Shared decisions. Approval-only files show the signed-file note; certified files show the banner with the first character typed.

**Behaviour.** `AddText` is recorded when typing ends (Escape, click away or a tool change) as one undo step; later style changes are separate steps. On save, the text becomes a Form XObject with a font subset and a ToUnicode map, so it stays searchable and copyable. In a tagged file it gets a structure element in reading order.

**Accessibility.**

- Focus order: toolbar → page → inspector.
- After placing a box, focus is in it. Escape returns focus to the page with the box selected.
- The box is announced as "New text box on page 3, edit text".
- Without a pointer: with the page focused, Enter on the Add text tool places a box at the top of the visible area, and arrow keys move it.

**Platform differences.** On the web there is no window frame and no system fonts. On desktop, system fonts are discovered at the edge (CoreText, DirectWrite, fontconfig). Phone and tablet are out of scope for this round.

**Artboards.** `A01-add-text.dc.html`

**Todos.** [E6c](../../todos/ready/E6c-add-text.md), [E6e](../../todos/ready/E6e-content-tools-ui.md)

## A02 Place image and shapes

**Purpose.** Put images, logos and shapes on a page as page content (not annotations), then move, resize, rotate, crop and fade them.

**Entry points.**

- Image tool (`tool.image`): opens the file picker.
- Drop an image file on a page, or paste an image (desktop: also from the OS clipboard).
- Shape split button (`tool.shape`).
- Palette: "Place image…", "Draw rectangle".

**Layout.** The full shell with Select pressed (the tool returns to Select after placing).

- **Left artboard:** the placed logo selected below the running header, with a snap guide on the right margin labelled in the top margin, and the inspector "Image". Top bar: "3 unsaved changes", Undo "Undo place image".
- **Right artboard:** the rectangle drawn around the West bar selected (eight handles and a rotation handle), with the inspector "Rectangle". Top bar: "4 unsaved changes", Undo "Undo resize rectangle".
- Both: Select pressed, rail at 280 px with More tabs (History), no status-bar middle note (added content is not a delete, replace or recolour of original content).

**Content and controls.**

- **On the page:**
  - The image has eight handles and a rotation handle. Handles are an overlay drawn above the page, so the rotation handle of an object placed just under the running header may cross the header rule (as on the left board); that is expected.
  - A dashed `accent` snap guide appears while dragging near the margins, the page centre and other objects' edges. Its label sits in the margin, never over text.
  - Shapes: a **click places a 72 × 72 pt shape** (a 72 pt line) at the click point, ready to edit in W and H; a drag draws it, and Shift keeps squares, circles and 45° lines.
  - Polygon: one point per click; Enter or a click on the first point closes it. Without a pointer, the inspector's Edit points (A05) adds points.
- **Inspector "Image"** (`Page 3 · Added by you`):
  - File line ("northwind-logo.png · 640 × 196 px · PNG with transparency") and **Replace…** (`edit.replace-image`).
  - **Position and size (pt):** X, Y, W, H; **Keep proportions** (pressed by default); **Rotate**. `edit.transform-object`
  - **Opacity:** slider plus value (focus ring drawn). `edit.set-opacity`
  - **Crop, inset from each edge (pt):** Left, Top, Right, Bottom, plus "Crop with handles", which swaps the resize handles for crop handles until Enter or Escape. `edit.crop-image`
  - **Arrange:** Bring to front, Bring forward, Send backward, Send to back, as a 2 × 2 group of secondary buttons. `edit.bring-to-front`, `edit.bring-forward`, `edit.send-backward`, `edit.send-to-back`
  - **Alternative text** and "Decoration, screen readers skip it" (tagged files).
  - **Actions:** Duplicate (`edit.duplicate`), Delete image (`edit.delete`).
- **Inspector "Rectangle"** (shape; same pattern for line, ellipse and polygon):
  - Position and size and Rotate.
  - **Fill:** No fill (chosen on the board) and the content colours.
  - **Stroke:** colour swatches with Custom…, Line (Solid, Dashed, Dotted, None) and Width (pt). The focus ring is drawn on Width.
  - Corner radius (rectangles only), Opacity, the four Arrange buttons, Duplicate, Delete shape.
  - Commands: `edit.set-fill`, `edit.set-stroke`.

**States.**

- **Unsupported file** (for example HEIC on the web): no object exists yet to show an error on, so a small dialog "lamina can’t place this file" names the formats it takes (JPEG, PNG; desktop also converts others it can decode), with OK.
- **Very large image:** placed at the page's text width, with the inspector note "Shown smaller than its full size; the full image is saved."
- **Picker cancelled:** nothing changes.
- **Signed file:** as in Shared decisions.

**Behaviour.**

- `AddContentObject` is recorded on place, `UpdateContentObject` for each finished change (a drag is one step), and `DeleteContentObject` on delete.
- Each object is its own Form XObject, so it can be changed or removed without touching the page's original content.
- The page re-renders after each finished change ([E6f](../../todos/ready/E6f-reload-budget.md)); during a drag only the overlay moves.

**Accessibility.**

- The image is announced by its alt text; the shape as "Rectangle you drew, selected".
- Arrow keys nudge the selection; every handle has an inspector field.
- Opacity is a native range input with its value shown as text.
- Focus rings: Opacity (left), stroke Width (right).

**Platform differences.** Desktop: paste from the OS clipboard and drag from Finder or Explorer. Web: picker, drop, and clipboard paste where the browser allows.

**Artboards.** `A02-image-and-shapes.dc.html`, `A02-shape.dc.html`

**Todos.** [E6b](../../todos/ready/E6b-place-images-and-shapes.md)

## A03 Watermark, header and footer dialog

**Purpose.** Stamp a watermark or background, a header and footer with page tokens, or Bates numbers onto a page range, with a live preview, as page content.

**Entry points.**

- Edit toolbar → Page marks ▾ → Watermark…, Header & footer…, Bates numbers… (`edit.add-watermark`, `edit.add-header-footer`, `edit.add-bates`).
- Palette: "Add watermark", "Add page numbers".
- "Manage page marks…" (`edit.manage-page-marks`) lists the marks lamina added, to edit or remove them.

**Layout.** One **Page marks** dialog, the 820 px wide dialog of [§7.18](design-language.md#718-dialog-proposed), over the full Edit-mode shell (Page marks ▾ shown open in the toolbar), dimmed and inert. The dialog has:

- a title, a subtitle ("Stamped onto the pages as content, not as comments") and "Close page marks";
- tabs: Watermark · Header & footer · Bates numbers;
- a form column and a preview column;
- a footer with a summary line, Cancel and the primary action.

**Content and controls.**

- **Watermark tab** (left artboard):
  - **Use:** Text or Image… (radios built as in [§7.19](design-language.md#719-form-controls-proposed-consistent-with-the-page-input-and-note-field)).
  - **Text** (focus ring drawn).
  - **Font** and **Size**.
  - **Colour**: Grey, Red, Teal, Ink.
  - **Opacity** and **Angle**.
  - **Layer**: In front of the content, or Behind the content (behind = a background).
  - **Pages:** all, or a range (page labels allowed).
  - **Readers can hide it**: writes an optional content group.
  - The preview draws the mark in its real colour and opacity, with a page stepper.
  - Footer: summary · Cancel · **Add watermark**.
- **Header & footer tab** (right artboard; it follows the left one, so the watermark just added shows on the page and in the preview, and Undo names it):
  - A 2 × 3 grid of slot inputs, each labelled by its row and column.
  - **Insert into the slot:** Page, Pages, Page label, Date, File name. Tokens are written in square brackets ("[Page] of [Pages]").
  - Size, Top and Bottom margin, Date format, and Pages from/to.
  - **Overlap notice** (warning): lamina looks for text already in the header and footer bands. Here: "**These pages already show a page number (“Page 3 of 24”).** Yours would add a second one. Clear the Right slot or remove the existing footer from the page first." The Right footer slot (focus ring drawn) is described by the notice.
  - **Preview:** new marks are drawn in the colour they will print, each inside a 1 px dashed `control-border` box. Text already on the page is drawn plainly. The note explains this, so "new" never depends on colour.
  - Footer: "Adds a header and footer to pages 2–24 (23 pages)." · Cancel · **Add header and footer**.
- **Bates numbers tab** (not drawn; same layout): Prefix, Start number, Digits (zero-padded), Suffix, Slot and Pages, with a preview of the first and last number ("NHT-000001 … NHT-000024"). In a batch (E14d) the count continues across files.

**States.**

- **Invalid range:** the Pages input turns `danger` with "Pages 1–30: this file has 24 pages." and the primary is disabled.
- **Empty text:** the primary is disabled, with the reason as help text.
- **Editing an existing mark:** title "Edit watermark", primary "Update watermark", plus "Remove watermark" (danger ghost).
- **Many pages:** a determinate progress bar in the footer ("Adding to pages… 62%"); still one undo step.
- **Approval-signed file:** no change.
- **Certified file:** the footer adds "Saving will invalidate the certification by R. Osei", in `warning`, matching the banner.

**Behaviour.**

- The primary records `AddWatermark`, `AddBackground`, `SetHeaderFooter` or `ApplyBates` as one op and closes the dialog. Focus returns to Page marks, and a toast says "Watermark added to 24 pages · Undo".
- Marks are Form XObjects, marked as artifacts in tagged files.
- Tokens are expanded at apply time.
- The removal ops only remove marks lamina added.

**Accessibility.**

- `role="dialog"`, `aria-modal`, labelled by its title.
- The tabs are a tablist with arrow-key movement.
- The preview is `role="img"`, described by its note; its stepper is a live region.
- Escape cancels.

**Platform differences.** Identical on web and desktop; desktop lists system fonts. Phone (later): the dialog becomes a large-detent sheet with the preview above the form.

**Artboards.** `A03-watermark.dc.html`, `A03-header-footer.dc.html`

**Todos.** [E6d](../../todos/ready/E6d-watermarks-headers-footers.md)

## A04 Edit existing text inline

**Purpose.** Edit a paragraph that is already in the PDF, in place. lamina keeps the embedded font where it has the glyphs and says plainly when it doesn't, or when the text can't be edited at all.

**Entry points.** Edit text tool (`tool.edit-text`, `E`), then click a paragraph; double-click text with the Select tool; Edit text in the A05 "Text block" inspector; palette "Edit text".

**Layout.**

- The full shell with Edit text pressed, the context note "Click a paragraph to edit it · Dashed outlines show editable text", and the inspector "Paragraph".
- Every detected run has a 1 px dashed `control-border` outline (3.65:1 on white).
- The paragraph being edited has the 2 px `accent` focus outline.
- The top bar shows "No unsaved changes" until editing ends (Undo and Redo disabled, no Save); the rail is the 220 px `Pages · Outline · Attachments`; the status bar has no middle note.

**Content and controls.**

- **On the page:** the paragraph is a text editor. A character drawn with another font has a 2 px dotted `warning` underline, in the editor only (never saved).
- **Inspector "Paragraph"**:
  - **Substitution notice** (warning, only when needed): "1 character uses another font. The font in this file has no “č”, and its full version isn’t available. lamina draws it in Source Serif 4, the closest bundled match, and marks it with a dotted underline here."
    - Choices: **Substitute only missing characters** (default) or **Set the paragraph in Source Serif 4**; plus **Choose another font…**.
    - `edit.substitute-missing-glyphs`, `edit.set-paragraph-font`
  - **Font in the file:** its name and coverage ("NorthwindSerif Regular · Embedded subset. Has every character in this paragraph except “č”.").
  - Size, Line spacing, Style, Alignment and Colour. The current colour is named "Ink (original)". If the font has no bold or italic face, Bold and Italic are drawn synthetically, and the Style help says so.
  - **Actions:** Done (`edit.finish-text`), Revert paragraph (`edit.revert-text`).

**Font resolution order** (E7e, E7f). The user sees it only at step 3 or 4.

1. The embedded subset has every glyph: re-encode with it.
2. The full font is available (embedded in full, or the same font on the computer): add a new subset.
3. Use a metrics-matched substitute for the missing characters only.
4. The user picks a font.

**States.**

- **Original font is enough:** no notice.
- **Substitution:** drawn, left.
- **Can't edit in place** (right artboard): the chart labels are outlined with the 1 px `accent` selection outline (the only focus ring is on the first action), and the inspector "Chart labels" says "This text can’t be edited in place. Its letters are drawn as shapes with no record of which characters they are, so lamina can’t retype them in the same font." It offers:
  - **Select to move or delete**: switches to Select with the text object selected. Moving, resizing, rotating or deleting the whole object is allowed (E7d A.07); its letters can't be changed.
  - **Delete and add new text**: one step; opens an A01 box in its place.
  - **Redact…**
  - "Details" names the cause ("Type 3 font without a text map · 8 labels").
  - The notice uses the neutral notice style (`surface` with a `hairline` edge).
  - Other causes have their own sentence: scanned text (suggests OCR), form fields (suggests Fill & Sign), annotations (suggests Comment).
- **Reverted:** the paragraph returns to the file's original; the op is dropped.

**Behaviour.**

- Typing updates the overlay at once.
- `EditText` is recorded when editing ends (Done, Escape, click away), one step per paragraph session. Style changes are `SetTextStyle` steps.
- Following glyphs on the line are repositioned. Lines reflow only when alignment or spacing changes.
- In tagged files the structure element is kept and `/ActualText` is updated (E7i).

**Accessibility.**

- The editor is a multi-line `textbox` described by the substitution notice's sentence, so the warning is announced when it appears. Only that sentence is the `role="status"` live region; the choices below it are not inside it, so changing them doesn't re-announce the notice.
- With the Edit text tool, Tab cycles editable runs in reading order, Enter starts editing and Escape ends it.
- The inspector states everything that the dashed outlines and the dotted underline show.

**Platform differences.** Desktop can reach step 2 with fonts on the computer. The web can only use embedded, bundled or added fonts, so substitution is more common there, and the notice says "isn’t available in the browser".

**Artboards.** `A04-inline-edit.dc.html`, `A04-not-editable.dc.html`

**Todos.** [E7e](../../todos/ready/E7e-text-runs-and-inline-edit.md), [E7f](../../todos/ready/E7f-font-handling-and-styling.md)

## A05 Select and transform existing objects

**Purpose.** Select images, vector paths and text blocks that were already in the file, then move, resize, rotate, restyle, reorder, clip, unmask or delete them.

**Entry points.**

- Select tool (`tool.select`, `V`), then click an object.
- Shift-click adds to the selection.
- Double-click edits text (A04) or opens a group.
- Right-click or Shift+F10 opens the context menu.

**Layout.** The full shell with Select pressed, in three artboards:

- **Left:** the chart's four bars selected as one group, with eight handles and a rotation handle. The context menu is open above the selection, clear of it and of the rotation handle. Top bar "2 unsaved changes", Undo "Undo recolour 4 shapes"; rail 280 px with More tabs (History).
- **Middle:** a paragraph selected as a **text block**, with nothing changed yet ("No unsaved changes", 220 px rail, no middle note).
- **Right:** **edit points** of the West bar, after a point was moved ("3 unsaved changes", Undo "Undo move point").

**Content and controls.**

- **Context menu** (left), the [§7.26](design-language.md#726-menu-proposed) menu: 32 px items, 14 px labels indented to the icon column, shortcut chips, the palette's active-row style on Bring forward (focus ring drawn), disabled items in the `disabled` colour with the reason as a tooltip, Delete last in `danger`. Items: Cut ⌘X · Copy ⌘C · Duplicate │ Bring to front · Bring forward · Send backward · Send to back │ Edit points ↵ · Release clipping · Release mask (disabled: these paths have no mask) · Replace image… (disabled for paths) │ Delete shapes ⌫.
  - It opens next to the pointer, flips to stay inside the canvas, and never covers the selection when there is room.
  - Commands: `edit.cut`, `edit.copy`, `edit.duplicate`, `edit.bring-to-front`, `edit.bring-forward`, `edit.send-backward`, `edit.send-to-back`, `edit.edit-points`, `edit.release-clip`, `edit.release-mask`, `edit.replace-image`, `edit.delete`.
- **Inspector "4 shapes":**
  - Position and size (X, Y, W, H) and Rotate.
  - **Fill** swatches: the original colour first, named "Slate (original)", with Teal chosen.
  - **Stroke** and **Width** (Width is disabled while Stroke is None, with the help "Set a stroke first.").
  - **Shape and clipping:** Edit points and Release clipping, with "Clipped to the chart area by the original file."
  - Actions: Revert to original (`edit.revert-object`), Delete shapes.
- **Inspector "Text block"** (middle):
  - A single click with Select selects a text block as a whole object (E7d A.07): a 1 px `accent` outline, eight handles and a rotation handle. The inspector shows a quote of its text, then X, Y, W, H and Rotate (focus ring on X).
  - Help: "A side handle re-flows the lines in the same font; a corner handle scales the text."
  - Arrange (four buttons), **Edit text** (or double-click), Revert to original (disabled until something changed), Delete text block.
  - Text whose letters can't be edited (A04, right) is still a text block here: moving and deleting it as a whole is allowed; editing inside its glyphs is not.
- **Inspector "Edit points"** (right):
  - On the page, the path being edited has a 1 px dashed `accent` outline and its anchor points are 8 px squares; the selected one (top right of the West bar) is filled `accent`. Smooth points also show two 6 px round control handles (not drawn: the bar has only corners).
  - The inspector "Edit points" (`West bar · point 2 of 4 · part of the original page`) shows the selected point's X (focus ring) and Y, Point type (Corner or Smooth, a segmented radio group), Previous point, Next point, Add point after, Done and Delete point.
  - The toolbar's context note says "Editing points · Tab moves between points · Enter or Esc to finish".
  - Tab and Shift-Tab move between points, and arrow keys move the selected one. These make every point edit possible without dragging.
- **Images**, inspector as A02, plus:
  - **Apply to:** This page only or All 24 pages that use it. Copy-on-write is the default ("This page only").
  - For an image with a soft mask or mask, a **Mask** row: "Uses a transparency mask" with **Release mask** (`edit.release-mask`) and **Invert mask** (`edit.invert-mask`).
- **Status bar:** the history note on the left and right boards, because a recolour or path edit of original content is pending.

**States.**

- **Nothing selected:** hovering an object shows a 1 px `accent` outline.
- **Shared image:** described above.
- **Object lamina can't change safely** (an inline image inside text, a shading, content inside a Type 3 glyph): outlined, with no handles. The inspector says why and offers Delete only when deleting is safe.
- **Locked by a signature field's appearance:** a read-only inspector naming the signature.

**Behaviour.**

- Each finished change is one `EditContent` op holding one `ContentEdit` (Transform, Delete, ReplaceImage, Reorder, Clip, EditPath, SetClip, SetMask).
- A change to a multiple selection is one op.
- Revert to original drops this object's edits.

**Accessibility.**

- The context menu is `role="menu"` with arrow keys; Escape returns focus to the selection. The focus ring is drawn on the active item.
- With Select, Tab moves between objects in content order.
- Selections are announced by kind and count ("4 shapes selected, part of the original page").

**Platform differences.** Windows and Linux show Ctrl X, Ctrl C and Delete in the menu. The menu is lamina's own on web and desktop, so labels and enabled states come from the command registry.

**Artboards.** `A05-select-transform.dc.html`, `A05-text-block.dc.html`, `A05-edit-points.dc.html`

**Todos.** [E7d](../../todos/ready/E7d-select-transform-existing.md), [E7h](../../todos/ready/E7h-vector-clipping-masks.md)

## A06 Search and replace across the document

**Purpose.** Replace text across the whole document, with a preview, a per-match choice, and a report of matches that could not be replaced.

**Entry points.** Edit toolbar → Replace… (`find.open-replace`); ⌥⌘F (macOS desktop app) or Ctrl H (Windows/Linux desktop app); palette "Find and replace"; or the "Replace" disclosure in the find bar of an editable file.

**Layout.**

- The **find bar from [R05](reader-a.md#r05-find-in-document)** replaces the Edit toolbar while it is open, so there is still one secondary toolbar ([§8.1](design-language.md#81-modes-and-their-secondary-toolbars)). Close brings the Edit toolbar back. The top bar's Find button shows pressed (`aria-expanded="true"`).
- First row, as R05: a 300 px field with the search icon and the count inside it ("1 of 6"), Previous and Next, a divider, Match case and Whole words, the **Replace** disclosure (`aria-expanded="true"`, `aria-controls` the second row), "Searched all 24 pages" and Close.
- Second row ([§7.30](design-language.md#730-find-bar-and-results-proposed)): the Replace with field (300 px, Replace icon inside, focus ring drawn), Replace, Skip, **Replace 5 checked** (primary) and "One undo step for all of them".
- The rail is 280 px with `Pages · Outline · Results` and More tabs (Attachments; History too once the session has a step).
- There is no inspector, and no status-bar middle note until something is replaced.

**Content and controls.**

- **Bar:**
  - Commands: `find.next`, `find.previous`, `find.match-case`, `find.whole-words`, `find.open-replace`, `edit.replace-one`, `edit.skip-match`, `edit.replace-checked`, `find.close`.
  - Focus ring on Replace with.
- **Results tab:**
  - A "Replace all matches" tri-state checkbox (drawn mixed: `accent` box with an 8 × 2 white dash) and "5 of 6 checked", then "6 matches on 5 pages".
  - Matches are grouped by page headings, 12/600 `text-3` as R05: "Page 3 · Repairs backlog", "Page 7 · Rent collection" (2), "Page 9 · Rent collection", "Page 11 · Surveyor capacity", "Page 16 · Risk register". Each match is a checkbox row whose context shows the old words struck through in `muted` and the new words 600 in `accent`.
  - Unchecked rows say "Skipped: stays as it is".
  - The current match has the selected-row style.
- **On the page:** matches use the `find-match` and `find-current` tokens ([§2.6](design-language.md#26-find-marks-proposed)): other matches `#F6E3BC` with a 1 px `#8A5300` bottom edge, and the current match `#EDBF6A` with a 2 px `accent` outline (drawn on page 3).

**States.**

- **No matches:** the count says "No matches" and the Replace buttons are disabled.
- **Report after replacing** (right artboard):
  - The count reads "2 left"; Replace, Skip and the primary (now "Replace checked") are disabled, and the row's note says "Nothing left to replace".
  - A warning notice says "4 of 5 replaced. 1 match couldn’t be replaced."
  - Outcomes, each with a badge: **Not replaced** (with its reason and "Go to page 9", focus ring drawn), **Skipped by you** ("Go to page 11"), and **Replaced · 4** ("Pages 3, 7 (2), 16", with the new wording). Each link is `inline-flex` with a 24 px minimum height.
  - A toast "4 matches replaced · Undo" (Undo with a 24 px target), the top bar at "1 unsaved change" with Undo "Undo replace 4 matches", and the history note in the status bar.
- **Reasons a match can't be replaced:**
  - hidden OCR text over a scanned image;
  - text that can't be edited in place (the A04 reasons);
  - a form field or annotation;
  - no acceptable font, with "Allow substitutes and try again".

**Behaviour.**

- One `ReplaceText` op covers the checked matches; it is applied match by match through the A04 editing path.
- A failure doesn't stop the other matches. Undo reverts them all.
- Editing the Find field rebuilds the results and clears the report.

**Accessibility.**

- The bar is a `search` landmark and the count is a live region.
- Results are real checkboxes, built as in [§7.19](design-language.md#719-form-controls-proposed-consistent-with-the-page-input-and-note-field): a native input visually hidden inside its label, with a drawn box. The tri-state one is the same native checkbox with its `indeterminate` property set (the mixed state can't be written in markup, so the board draws the dash), described by "5 of 6 checked".
- The report notice is `role="status"`.
- Enter in Find goes to the next match; Enter in Replace with replaces this one.

**Platform differences.** Only the shortcut differs. Phone (later): the bar and the results become a sheet.

**Artboards.** `A06-replace.dc.html`, `A06-report.dc.html`

**Todos.** [E7g](../../todos/ready/E7g-search-and-replace.md)

## A07 Structure and conformance warning

**Purpose.** Keep a tagged file's structure valid while editing, and tell the user, before saving, when an edit would break a PDF/A or PDF/UA claim the file makes.

**Entry points.** Automatic: entering Edit mode on a tagged or PDF/A file; any edit that creates an issue; Save. The claim badges in the status bar, and the banner's Details, open the Accessibility tool's check panel (Tools ▾ → Accessibility, X06/X07).

**Layout (left artboard).** The Edit-mode shell with:

- **A warning strip** ([§7.29](design-language.md#729-strip-proposed), `warning-tint`) under the toolbar, with the Structure icon: "**This file would no longer meet PDF/UA-1.** The image you added on page 3 has no alternative text. The tags are otherwise up to date." · Details (ghost) · Add alternative text (secondary).
- The rail as everywhere in Edit mode: `Pages · Outline · Attachments` with More tabs (History). Reading order and tags are not a rail tab; they live in the Accessibility tool (Tools ▾ → Accessibility, X06/X07).
- The added image selected on the page (handles, no badge on the page: the strip and the inspector say what is wrong).
- **The inspector "Image"** (`Tagged as Figure`):
  - **Alternative text** in the error state: `danger` border, `aria-invalid`, the error "Needed to keep the PDF/UA claim.", and the focus ring.
  - The Decoration checkbox.
  - **This file’s claims**: PDF/A-2b "Still holds", PDF/UA-1 "1 issue".
- **The status bar:** the claim badges (one button, named "PDF/A-2b, PDF/UA-1 · 1 issue: open the conformance check", so its name starts with its visible text) beside the page label.

**Save check dialog (right artboard).** "Check before saving" lists each claim with a badge and one line:

- PDF/A-2b: Still holds.
- PDF/UA-1: 1 issue, with "Go to the image".
- Tags: Updated, "2 added on page 3 (a paragraph and a figure), in reading order".

Its explanation: "If you save now, lamina removes the PDF/UA-1 claim from the file, so the file doesn’t promise what it no longer meets. Fix the issue before saving to keep the claim." Actions:

- **Save without the claim** (secondary, focus ring drawn);
- **Fix it first** (primary): closes the dialog, selects the image and focuses Alternative text;
- "Close check before saving" (the close button's name): keep editing without saving.

The dialog is 560 px wide (the form width of [§7.18](design-language.md#718-dialog-proposed)) over the A07 shell, dimmed and inert.

The same dialog serves the PDF/A case from the Edit session area (E1c, "Save without the PDF/A claim?").

**States.**

- **On entering Edit mode** (not drawn): a neutral banner, "This file claims PDF/A-2b and PDF/UA-1. lamina keeps the tags up to date as you edit and checks both claims when you save." with Dismiss.
- **Issue found:** drawn. The banner comes with the edit that causes it and goes when the issue is fixed or the edit is undone.
- **All checks pass at save:** no dialog, and a toast "Saved · PDF/A-2b and PDF/UA-1 still hold".
- **PDF/A failure:** the same dialog with the PDF/A row in warning; it also offers "Save a copy instead".
- **Untagged file:** none of this appears, and A01's "Read by screen readers as" is hidden.

**Behaviour.**

- Structure maintenance runs inside each op: MCIDs, ParentTree, `/ActualText` and `/Alt`. Deleting a tagged run removes its element.
- Quick checks run after each op; the full profile check runs on Save.
- **Proposed:** removing a broken claim (`pdfuaid:` or `pdfaid:` in XMP) happens only through this dialog, as part of the save, never silently. It is consistent with the XMP editor (O04), where `pdfaid:` is read-only, "Kept by lamina". E7i itself only says "re-check and warn", so this is a decision for E7b/E7i (see Open questions).

**Accessibility.** The strip is `role="status"`. The dialog is a labelled modal described by its first paragraph. Badges pair colour with words and icons.

**Platform differences.** Identical on web and desktop.

**Artboards.** `A07-tagged-edit.dc.html`, `A07-save-check.dc.html`

**Todos.** [E7i](../../todos/ready/E7i-structure-and-conformance-maintenance.md)

## Open questions

1. **Broken conformance claims at save** (for [E7b](../../todos/ready/E7b-editing-existing-content-feature-plan.md) and [E7i](../../todos/ready/E7i-structure-and-conformance-maintenance.md)). The proposal is that lamina never saves a claim it has just found false; the only choices are to fix the issue first or to save without the claim. Should an expert option "save and keep the claim" exist? Should a removed claim come back automatically once a later save passes the check?
2. **Synthetic bold and italic** (E7f): should Paper show a badge on the page when a synthetic style is used, or is the inspector help enough?
3. **Replace row buttons** (for the [R05](reader-a.md#r05-find-in-document) owner). This doc and [design language §7.30](design-language.md#730-find-bar-and-results-proposed) give the Replace row as Replace · Skip · Replace n checked, because each match can be unchecked in the Results tab. reader-a.md's R05 table still says "Replace, Replace all"; it should follow §7.30.

## Proposed design-language additions

Merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29; see its decisions table.
