# Paper: Forms

Screens P01–P07 of the [screen inventory](screens.md): filling interactive forms in Fill & Sign mode, validation and calculated fields, form data (import, export, reset, flatten), filling on a phone, form design mode, the tab-order and accessible-label editor, and the review of automatically detected fields. Everything follows the [Paper design language](design-language.md); new parts are listed under [New components](#new-components).

- **Canvas:** [lamina Paper — Forms](https://claude.ai/artifact/Jat39ahU8QNgrb7TLi3mZR) (private until the owner shares it).
- **Plan:** [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and [§E9](../../plans/full-editor.md#e9-form-authoring); layout rules from the [UI/UX plan](../../plans/ui-ux.md) §4–§5.
- **Sample file:** `repair-request-form.pdf`, “Northwind Housing Trust — Repair request form”, 2 pages, 27 fields (12 on page 1, 15 on page 2), 9 required, 1 signature field. P07 uses a flat, older version of the same form, `repair-request-2019.pdf`. All names, references and addresses are fictional (`repairs.northwind-housing.example` uses the reserved `.example` domain).

Where forms live in the shell:

- **Filling** is the Fill & Sign mode (top-bar mode switcher). Its secondary toolbar is the one proposed in design language §8.1, plus **Highlight fields** and a **Form data** menu.
- **Designing** is a tool of Edit mode, like Redact and Protect: Edit › Form design swaps in its own secondary toolbar (“Form design” · field tools · Tab order · Detect fields · … · Done). The mode switcher keeps Edit selected.
- In both, the left rail gains a **Fields** tab (and, in design, **Tab order**, **Calculations** and **Suggestions** tabs as needed) next to Pages.

Command ids below use the registry’s `area.verb-object` form ([U1b](../../todos/ready/U1b-command-registry-and-palette.md)). Every command is in the palette; only those listed with a shortcut have one.

## P01 Fill & Sign mode

**Purpose.** Fill an existing interactive (AcroForm) form in place, field by field, with every field type: text (single and multi-line), check box, radio group, dropdown and list box, date and formatted text, push button and signature field.

**Entry points.** The Fill & Sign mode button; the palette command “Fill & Sign mode” (`mode.fill-sign`); opening a file with fields shows a status-bar item “Interactive form · 27 fields”, which switches to the mode when clicked. Clicking a field in Read mode also switches to Fill & Sign and focuses the field.

**Layout.** Web shell (inverted L). Top bar with Fill & Sign selected and the save state (“5 unsaved changes”). Secondary toolbar: Previous field · Next field · Highlight fields (toggle) │ Text · Check · Date (tools for placing free text, ticks and dates on flat forms) │ Add a signature image · Sign with a certificate · Form data ▾ · … · context note “4 of 9 required fields filled”. Left rail, **Fields** tab. Canvas: the page with fields drawn as DOM overlays. No inspector while filling: the field itself is the control. Status bar: network indicator · “Interactive form · 27 fields · Tab follows the form’s row order” · page label.

**Content and controls.**

| Control | Does | Command id |
| --- | --- | --- |
| Field overlays | Real `<input>`, `<textarea>`, `<button aria-haspopup="listbox">`, radio and check box elements placed over each widget, synced with zoom and rotation. Typing is coalesced into one undo step per field visit | `form.set-value` (implicit) |
| Previous field / Next field | Move focus in tab order, across pages (the page scrolls to the field) | `form.previous-field` (Shift Tab), `form.next-field` (Tab) |
| Next required field | Palette only: jump to the next empty required field | `form.next-required-field` |
| Highlight fields | Toggles the field tint; on by default for files with fields | `form.highlight-fields` |
| Text · Check · Date | Place a free-text, tick or date annotation on a page without fields (a flat form) | `fill.text`, `fill.check`, `fill.date` |
| Add a signature image · Sign with a certificate | Open the signing flows (Sign area, G01–G02) | `sign.add-image`, `sign.certificate` |
| Form data ▾ | Opens the P03 menu | `form.data-menu` |
| Fields tab | One row per field in tab order: tab-stop number, label (from the accessible label, else the field name), state (✓ filled, “Required”, error). Grouped by page; other pages collapse to “Show page 2 fields · 2 required”. Clicking a row focuses the field | `rail.fields` |
| Push buttons | Run the file’s non-script actions: go to a page, reset form (asks first), show/hide fields, open a link (asks first, naming the address), submit (P01 submit confirmation) | — |

The drawn state: page 1 with the first six fields filled, the **Type of repair** dropdown focused and open (a listbox menu under the field, “Damp and mould” active), required fields marked with a `danger` asterisk, two check boxes (one ticked).

**States.**

- **Empty form:** every field tinted; the context note reads “0 of 9 required fields filled”.
- **All required filled:** the note turns into “9 of 9 required fields filled”; nothing else changes (no celebration).
- **Field locked by a signature** (FieldMDP `/Lock` or a signature’s field-lock dictionary): the field draws as read-only (P02 style) and its Fields-tab row says “Locked by R. Osei’s signature”; trying to type shows a toast “This field is locked by a signature”. The signing name comes from the locking signature ([E8e](../../todos/ready/E8e-field-widgets-ui.md)).
- **Signed file that allows form filling** (DocMDP P2/P3): the status bar shows the signed-file note from design language §7.13, “Signed file. Saving adds your changes after the signature; the signature stays valid.”
- **XFA form:** a notice strip under the toolbar, “This form uses XFA, which lamina can’t fill. You can read it and fill the PDF fields it also contains, if any.” The Fields tab lists only the AcroForm fields.
- **No fields:** the Fields tab shows the empty state “No fillable fields” / “Use Text, Check and Date to fill a flat form, or design fields in Edit › Form design.”

**Behaviour.** Tab and Shift Tab follow the file’s `/Tabs` order (rows, columns or structure) and cross page boundaries. Enter in a single-line field moves to the next field; Space toggles check boxes; arrow keys move within a radio group; Alt ↓ (⌥↓) opens a dropdown, typing selects by first letters. Each field visit is one undo step (`SetFieldValue`). Values are stored with generated appearance streams so other apps show them ([E8d](../../todos/ready/E8d-field-appearances.md)). Saving is incremental; nothing is written until Save.

**Accessibility.** Each overlay is a native control whose accessible name is the field’s accessible label (`/TU`), falling back to the field name; required fields carry `aria-required="true"`; radios sit in a `fieldset` with a `legend`; the open dropdown is a `listbox` with `aria-selected` options. Focus ring: 2 px `accent`, offset 1 (drawn on the dropdown). The tint is decoration: every field also has a 1 px `control-border` edge (3.84 on white). The Fields tab is a `nav` of links with `aria-current` on the focused field.

**Platform differences.** Web and desktop are identical (drawn: web). Desktop adds the native window frame and ⌘ chips; Windows/Linux use Ctrl chips. Tablet uses the desktop shell with 44 px field hit areas (the overlay grows its hit area, not its drawing) and the bottom action bar instead of the dropdown menu. Phone: P04.

**Artboards.** `P01-fill-and-sign.dc.html` (tweak: Highlight fields on/off); `P01-submit-confirm.dc.html`.

### Submit confirmation

A push button with a SubmitForm action never sends silently; submit is off by default and confirmed per send ([E8e](../../todos/ready/E8e-field-widgets-ui.md), principle 7). The dialog “Send this form to another computer?” names the button, the destination host (bold) and path, the format, how many fields and which (with “Show all 19 fields and values”), and what is **not** sent (the PDF, comments, other files). A trust note says sending makes “1 network request” that the status bar will list, and that lamina asks again every time. Actions: “Don’t send” (focused by default, so Enter never sends by accident) and “Send” (primary). After sending, a toast “Sent to repairs.northwind-housing.example” and the network indicator counts the request with its host (design language §7.8). A failed send shows the reason inline in the dialog and keeps it open. `role="dialog"`, `aria-modal`, labelled by the title and described by the first paragraph. On the phone it becomes a bottom sheet at the large detent with full-width buttons.

**Todos:** [E8e](../../todos/ready/E8e-field-widgets-ui.md).

## P02 Validation, formatting and calculated fields

**Purpose.** Show the file’s own rules while filling: formatted input (dates, numbers, currency), validation errors, calculated totals, read-only fields, and which of the form’s scripts lamina handled.

**Entry points.** Happens while filling in Fill & Sign mode; the Fields tab and the toolbar note lead to fields that need attention.

**Layout.** As P01, on page 2. Under the secondary toolbar, a **script report notice** (warning tint) when the form has scripts. The context note reads “8 of 9 required fields filled · 1 field needs attention” (the second part in `danger`).

**Content and controls.**

- **Formatted input:** a date field formats on leaving it (“29/10/26” becomes “29/10/2026”); number fields right-align with tabular figures; currency shows “£18.40”. Formats come from recognised Acrobat AF* patterns and run natively, without running the script ([E8f](../../todos/ready/E8f-declarative-behaviour.md)).
- **Validation error** (drawn on “Earliest visit date”, value “31/09/2026”, focused): 2 px `danger` border plus the focus ring, and a **field message** directly under the field: warning glyph and “September has 30 days. Enter a real date, like 03/11/2026.” The message says what to do, never “Invalid”. The Fields tab row shows “Check date” in `danger` 600.
- **Calculated field** (“Total”, £98.40 = hours × rate + materials): dashed `control-border` edge, white fill, a Σ glyph at left, value 600 right-aligned, and the caption “Calculated: hours × rate + materials”. It is an `<output>` linked to its inputs; it recalculates in `/CO` order whenever an input changes, inside the same undo step as the change.
- **Read-only field** (“Job number”, “Hourly rate”): dashed edge, white fill, lock glyph, `aria-readonly="true"`. Tab still stops on it so screen-reader users can read it; typing does nothing and announces “read-only”.
- **Script report notice:** “**This form has scripts.** lamina runs the 3 it recognises (a total and two date formats) without running code, and skipped 1. Check the ‘Visit reference’ field by hand.” · Details · dismiss. Details opens a panel listing each script, the field it belongs to, and Handled / Skipped. The skipped field’s Fields-tab row says “Script skipped” in `warning`. What happens to unrecognised scripts is ADR 0021’s decision ([E8a](../../todos/ready/E8a-adr-0021-javascript-policy.md)); the screen only reports it.

**States.** No scripts: no notice. All scripts handled: the notice is not shown; the status bar says “Scripts: 3 handled”. Several errors: the note counts them (“3 fields need attention”) and the palette command “Next field that needs attention” (`form.next-invalid-field`) walks them. Hide actions: hidden fields disappear from the page and from the Fields tab (shown again when their condition changes).

**Behaviour.** Validation runs when focus leaves the field (not on each keystroke); the field keeps the typed value so it can be fixed, and the value is still stored (PDF validation rejects on commit; lamina keeps the text but flags it, so nothing typed is lost). Save is never blocked by validation; a form submit with invalid fields lists them in the submit dialog and disables Send. Undo restores the previous value and recalculates.

**Accessibility.** Error fields have `aria-invalid="true"` and `aria-describedby` pointing at the message, which is `role="alert"` the first time it appears. Calculated and read-only are said in words (caption, “read-only”), not shown by the dashed edge alone. Contrast: `danger` on `danger-tint` 6.31; message text `text` on tint ≥ 13.5.

**Platform differences.** Identical on web and desktop (drawn: web). Phone: the field message sits in the form bar (P04) under the field label, and the field’s border turns `danger`.

**Artboards.** `P02-validation.dc.html` (tweak: script notice on/off).

**Todos:** [E8f](../../todos/ready/E8f-declarative-behaviour.md).

## P03 Form data

**Purpose.** Move answers in and out of the form: import values (JSON, XFDF, FDF), export them, reset the form, and flatten it.

**Entry points.** Fill & Sign toolbar › **Form data ▾**; the palette (“Import form data…”, “Export form data…”, “Reset form…”, “Flatten form…”); desktop menu bar File › Form data. No shortcuts (low-frequency commands).

**Layout.** Desktop window (drawn), Fill & Sign mode, rail Fields tab showing a **summary** (Fields 27 · Filled 19 · Required left 0 · Signature fields 1). The menu drops from the toolbar button; dialogs sit over the dimmed window below the title bar.

**Content and controls.**

| Item | Does | Command id |
| --- | --- | --- |
| Import form data… | Native open dialog (JSON, XFDF, FDF). Values for matching field names are set in one undo step; a toast reports “19 fields filled · 2 names not in this form · Details” with Undo | `form.import-data` |
| Export form data… | Export dialog (below) | `form.export-data` |
| Reset form… | Alert dialog “Reset all 27 fields?” / “Every answer goes back to the form’s default. You can undo this.” with Cancel and “Reset form” (danger filled) | `form.reset` |
| Flatten form… | Flatten dialog (below) | `form.flatten` |
| Menu footer | “Import accepts JSON, XFDF and FDF. Values stay on this device.” | — |

**Export dialog** (“Export form data”, subtitle “Saves the answers as a separate file. The PDF doesn’t change.”): Format as large radio rows with one-line explanations (JSON, selected · XFDF · FDF); Fields: “Filled fields only · 19” or “All fields, including empty ones · 27”; File name (prefilled, extension follows the format); help “Signature values are never exported. Choose where to save it next.” Buttons Cancel and “Export…” (opens the native save dialog on desktop, a download on web).

**Flatten dialog** (`alertdialog`, “Flatten the form?”): explains the effect in plain words; scope radio (“Every field · 26” or “Only the fields selected in the Fields tab”); a note “**The signature field stays a field.** lamina never flattens signature fields, so the form can still be signed” ([E8g](../../todos/ready/E8g-form-data-and-flatten.md)); “You can undo this until you close the file. Nothing is written until you save.” Buttons Cancel (focused) and “Flatten 26 fields” (danger filled).

**States.**

- **Signed file** (tweak `signed`): flattening is a content change, so under a certification signature the dialog adds the warning notice “**Saving will invalidate the signature.** This file is certified by Northwind Housing Trust to allow form filling only. Flattening changes page content. Save a copy instead to keep the original signed.” After flattening, the save banner (design language §7.13) appears until undone or saved as a copy. With approval signatures only, no warning: the save is append-only.
- **Field locked by a signature:** excluded from the count; the scope line says “26 fields · 1 locked field is left as it is”.
- **Import with no matching names:** the toast becomes a notice in the Fields tab: “None of the 21 names in repair-data.xfdf match this form.” Nothing changes.
- **Import of a malformed file:** dialog error “This file isn’t valid XFDF (line 14: unclosed field element).” No partial import.

**Behaviour.** Import is one `ImportFormData` op (class FormFill: allowed on signed files that permit filling). Flatten is one `FlattenForm` op (class Content): appearances are stamped into page content and the fields removed; undo restores them. Export and reset change nothing on disk until Save (export writes only the new data file).

**Accessibility.** The menu is `role="menu"` with `menuitem`s, opened by the button with `aria-haspopup="menu"` and `aria-expanded`; arrow keys move, Escape closes and returns focus. Focus ring drawn on the first menu item (inset, as in phone bars). Dialogs as design language §7.18; destructive confirms are `alertdialog` with Cancel focused.

**Platform differences.** Desktop (drawn) and web are the same; web has no File menu, and Export downloads the file. Phone: Form data is in the More sheet; import uses the system file picker, export the share sheet.

**Artboards.** `P03-form-data.dc.html`, `P03-export-dialog.dc.html`, `P03-flatten-dialog.dc.html` (tweak: signed file).

**Todos:** [E8g](../../todos/ready/E8g-form-data-and-flatten.md).

## P04 Phone: fill a form field by field

**Purpose.** Fill a form on a phone without pinching: one field at a time, the page zoomed to it, with Previous / Next / Done always reachable above the keyboard.

**Entry points.** Tapping a field on the page; More › Fill form (starts at the first empty required field).

**Layout.** Phone shell. Top bar: Back, title “Repair request” over “5 of 9 required filled · 6 unsaved”, More. The page zooms so the focused field’s text is 16 px (no iOS input zoom) and scrolls it into view above the **form bar**. The form bar replaces the bottom bar and sits on top of the system keyboard (drawn as a labelled placeholder: the real keyboard is the OS’s). For choice fields (dropdown, list box, radio group) there is no keyboard: a **choice sheet** opens at the medium detent instead.

**Content and controls.**

- **Form bar** (`role="toolbar"`, “Form navigation”): field label (15/600), position “9 of 27”, hint line (“Required · Day, month, year, like 14/08/2026”, or the error message in `danger`), then Previous field (44, chevron up), Next field (44, chevron down), “Next required” (ghost), Done (primary). Done closes the keyboard and returns the bottom bar.
- **Text field:** the overlay input on the page, 44 px high, `inputmode` from the field’s format (numeric for dates and numbers, email, tel).
- **Choice sheet** (“Type of repair”, “Required · field 7 of 27”): a `radiogroup` of 48 px rows, the selected row tinted with a check; a footer with Previous and Next (Next is primary and also commits the choice). Close keeps the value.
- **Check boxes and radios** on the page toggle with a tap; the form bar still shows (without a keyboard) so Next works.

**States.** Error: hint line in `danger`, field border `danger`. Read-only: the form bar says “Read-only” and Next skips nothing (screen readers still reach it). Last field: Next becomes “Review” and opens the Fields sheet (the Fields tab as a sheet, P01). Signature field: Next lands on it and the bar offers “Sign” (opens G01).

**Behaviour.** Next/Previous follow tab order; swiping the page keeps the field focused until the user taps elsewhere. Autocorrect and autocapitalise off for fields with a format. Undo is in More (and shake-to-undo on iOS). Each field visit is one undo step, as on desktop.

**Accessibility.** All targets ≥ 44 px; the form bar is announced as a toolbar; focus moves to the field, not the bar, so VoiceOver reads the label, value and hint (`aria-describedby` on the hint). The choice sheet traps focus while open and returns it to the field. Focus ring drawn on the date input and the selected sheet row (inset).

**Platform differences.** Phone (drawn). Tablet uses the desktop shell (P01) with the form bar docked above the keyboard in landscape. Android uses the same bar; the back gesture closes the sheet first, then leaves the field.

**Artboards.** `P04-phone-fill.dc.html`, `P04-phone-choice.dc.html`.

**Todos:** [E8e](../../todos/ready/E8e-field-widgets-ui.md).

## P05 Form design mode

**Purpose.** Create, place, resize and configure fields of every type (text, check box, radio group, dropdown, list box, push button, signature, date), with their names, accessible labels, options, flags, format, validation and calculation.

**Entry points.** Edit mode › Form design (button in Edit’s toolbar); palette “Form design” (`form-design.open`); from the Fields-tab empty state on a file without fields. No shortcut.

**Layout.** Desktop window (drawn). Edit selected in the mode switcher. Secondary toolbar: “Form design” (Newsreader 15) · Select · Text field · Check box · Radio group · Dropdown · List box · Button · Signature · Date │ Tab order · Detect fields · … · “27 fields · Snap to page grid” · Done. Rail tabs: Pages · **Fields** (field names in monospace with their type; selection syncs with the page) · **Calculations** (calculation order, see below). Canvas: every field drawn in **design style**: dashed `accent` edge, field tint, the field name in monospace inside. Inspector: the selected field. Status bar: hint “Arrow keys move the field 1 pt · Shift adds 10 · Alt resizes” (⌥ on macOS).

**Content and controls.**

| Control | Does | Command id |
| --- | --- | --- |
| Field tools | Pick a tool, then drag on the page to draw the field, or click to place it at its default size. Each placement selects the new field | `form-design.add-text-field`, `…-check-box`, `…-radio-group`, `…-dropdown`, `…-list-box`, `…-button`, `…-signature`, `…-date` |
| Select | Click a field to select; Shift-click adds; drag moves; 8 handles resize; a size chip (“218 × 20 pt”) shows while selected; snaps to the page grid and to other fields’ edges | `form-design.select` |
| Inspector: Field name | Unique fully qualified name; a clash shows “Another field is called repair_type” and keeps the old name | `form-design.rename-field` |
| Inspector: Accessible label | The `/TU` text: screen readers say it and other apps show it as the tooltip | — |
| Inspector: Options | (Dropdown, list box) rows with Move up and Remove; Add option; “Let people type their own answer” (combo edit flag) | — |
| Inspector: When filling in | Required · Read-only · Hidden when printed | — |
| Inspector: Validation, Format, Calculation | Selects for the native declarative settings ([E8f](../../todos/ready/E8f-declarative-behaviour.md)): Format (number, percent, date, phone, postcode), Validation (range, one of the options), Calculation (sum, average, product, smallest, largest of chosen fields). Text fields show all three; the drawn dropdown shows Validation only | — |
| Inspector: Position (pt) | X, Y, W, H number inputs: the keyboard alternative to dragging | `form-design.move-field` |
| Duplicate · Delete field | Copy with a new unique name; delete (undoable) | `form-design.duplicate-field`, `form-design.delete-field` |
| Signature field (when selected) | Adds “Lock these fields when signed” (FieldMDP `/Lock`) and seed values ([E9b](../../todos/ready/E9b-field-creation-ops.md)) | — |
| Calculations tab | Ordered list of calculated fields with Move up/down: the `/CO` calculation order | `form-design.calculation-order` |
| Done | Back to Edit’s default toolbar | `form-design.close` |

**States.** No fields yet: the Fields tab empty state offers “Detect fields” (P07). Several selected: the inspector shows shared properties (flags, size) and “3 fields”. Signed file: entering Form design shows the save banner pre-emptively, because any field definition change is a DocMDP violation under certification (“Saving will invalidate the signature…”, Save a copy); with approval signatures only, locked fields and signature fields cannot be selected and say why.

**Behaviour.** Every change is an op (`CreateField`, `UpdateFieldDef`, `MoveWidget`, `RenameField`, `DeleteField`; class Document) and one undo step; dragging is one step on release. Saving is incremental. Switching to Fill & Sign shows the form as it will behave.

**Accessibility.** Fields on the page are `button`s with names like “repair_type, dropdown, selected”; the Fields tab is a `listbox` synced with the page. Arrow keys move, Shift+arrows by 10 pt, Alt+arrows (⌥) resize; every drag has the Position inputs as an alternative (WCAG 2.5.7). Focus ring drawn on the Accessible label input; the selected field shows the 2 px `accent` outline (selection, design language §8.2).

**Platform differences.** Desktop (drawn) and web identical. Not offered on the phone (“Form design needs a larger screen” in More, opening on a tablet or computer); tablet supports it with 44 px handles and the Position inputs.

**Artboards.** `P05-form-design.dc.html`.

**Todos:** [E9c](../../todos/ready/E9c-form-design-mode.md), [E9b](../../todos/ready/E9b-field-creation-ops.md).

## P06 Tab order and accessible labels

**Purpose.** See and change the order Tab follows, and make sure every field has a label screen readers announce.

**Entry points.** Form design toolbar › **Tab order** (toggle); the palette “Edit tab order” (`form-design.tab-order`); the toolbar note “1 field has no accessible label” (jumps to the first one).

**Layout.** Desktop window (drawn), Edit › Form design with Tab order pressed (field tools are paused, said in the toolbar). Rail tab **Tab order**: an “Order on page 1” select (Rows, left to right · Columns, top to bottom · Document structure · Custom) with a one-line explanation, then the ordered list of tab stops. Canvas: each field shows a numbered **tab-stop badge** at its top-left; a field without a label also shows a “No label” warning badge. Inspector: the selected field’s label and tab stop.

**Content and controls.**

- **Order select** sets `/Tabs` for the page (`/R`, `/C`, `/S`); any manual move switches it to Custom (`SetTabOrder`, class Document).
- **Tab-stop list** (`ol`): drag handle, number, label, warning glyph for missing labels; the selected row adds Move up and Move down buttons (Move down disabled on the last stop). Keyboard: ⌥↑ / ⌥↓ (Alt ↑ / Alt ↓) move the selected stop. Clicking a row selects the field on the page.
- **Clicking badges in sequence** on the page sets a custom order quickly (“click fields in the order Tab should follow”); Escape stops.
- **Inspector:** warning notice “No accessible label. Screen readers say only ‘check box, not checked’.”; Accessible label (textarea); “Use nearby text” fills it from the closest text on the page; a preview “Screen readers will say: ‘You may enter with a key if I’m not at home, check box, not checked, 12 of 27’”; Tab stop number input (“of 12 on this page”), the non-drag alternative; “Also tag the field for screen readers (this file is tagged)”, which writes the Form structure element ([E15c](../../todos/ready/E15c-tagging-tools.md) completes form tagging); “Next field without a label”.

**States.** All fields labelled: the toolbar note disappears and the list shows no warnings. Untagged file: the tagging checkbox is replaced by “This file isn’t tagged. The label is still saved for screen readers that read tooltips.” Document structure order on an untagged file is disabled with that reason.

**Behaviour.** Each move or label change is one undo step. Turning Tab order off returns to field tools; badges hide.

**Accessibility.** Badges are decoration (`aria-hidden`); the order is conveyed by the list. The list exposes position (`aria-current` on the selected stop); Move buttons are labelled “Move up” and “Move down”. Focus ring drawn on the label textarea. The warning uses an icon and words, not colour alone.

**Platform differences.** Desktop (drawn) and web identical (web: Alt ↑ / Alt ↓). Not on the phone.

**Artboards.** `P06-tab-order.dc.html`.

**Todos:** [E9c](../../todos/ready/E9c-form-design-mode.md).

## P07 Automatic field detection review

**Purpose.** Turn a flat (printed or scanned-and-OCRed) form into a fillable one: lamina suggests fields from ruling lines, boxes, underscores and label text, and the user accepts, edits or rejects each one. No suggestion is applied without user action ([E9d](../../todos/ready/E9d-automatic-field-detection.md)).

**Entry points.** Form design toolbar › **Detect fields**; the Fields-tab empty state on a file without fields; palette “Detect form fields” (`form-design.detect-fields`).

**Layout.** Desktop window (drawn) on `repair-request-2019.pdf`. Form design toolbar with Detect fields pressed: Accept all on this page (secondary) · Reject all on this page · … · “Page 1: 12 suggestions · 4 accepted · 1 rejected” · Done. A notice strip: “**This file has no fillable fields.** lamina looked for lines, boxes and labels on this device and suggests fields. Nothing is added until you accept it.” Rail tab **Suggestions**: one row per suggestion (label, type, Accept ✓ and Reject ✕ buttons; accepted and rejected rows show a badge instead). Canvas: pending suggestions outlined with a 2 px dashed `accent` outline and a type tag (“Date”, “Text, several lines”); accepted ones become design-style fields with an “Accepted · Text” tag in `positive`; rejected ones lose their outline. Inspector: the selected suggestion.

**Content and controls.**

- **Inspector** (“Suggested date field”, “Page 1 · Suggestion 5 of 12”): “Why lamina suggests it” (“A ruled line after the label ‘Date first noticed’, which reads as a date.”); Field type select; Field name (from the label); Accessible label (from the label, “Taken from the label beside the line.”); type-specific settings (Date format); Accept, Reject, Next.
- **Accept** creates the field (`CreateField`, one undo step) with the shown settings and selects the next suggestion. **Accept all on this page** is one undo step. Reject only hides the suggestion (no file change); Rejected rows can be restored from the row’s menu.
- Accepted fields can be edited like any field (P05) once detection is closed.

**States.** Running: an indeterminate progress bar in the notice (“Looking for fields on 2 pages…”), the page stays readable. Nothing found: empty state “No fields found” / “Draw fields with the field tools instead.” File already has fields: Detect fields still runs but skips areas covered by existing fields, and says so. Detection never uses the network; the status bar says “Detection ran on this device”.

**Behaviour.** Suggestions are not part of the document or the undo history until accepted; leaving Form design discards pending ones after a confirmation if any remain (“12 suggestions not reviewed. Leave them?”). No confidence numbers are shown; the reason line explains each suggestion instead.

**Accessibility.** Each row’s buttons are labelled “Accept Room”, “Reject Room”; accepted/rejected states use words in badges. The selected suggestion has `aria-current`. Focus ring drawn on the inspector’s Accept button. Everything is keyboard reachable: ↑ ↓ in the list, Enter accepts, Delete rejects (page or list focus only, never in inputs).

**Platform differences.** Desktop (drawn) and web identical. Not on the phone.

**Artboards.** `P07-field-detection.dc.html`.

**Todos:** [E9d](../../todos/ready/E9d-automatic-field-detection.md).

## New components

Proposed for the design language (§7) when these boards are reviewed.

**Token.** `field-tint` **(proposed)** = `accent` 8% over white, `#EDF2F3`: the fill of fillable fields while Highlight fields is on. It is decoration; the field’s boundary is its 1 px `control-border` edge (3.84 on white). Page ink on it ≥ 14.

| Component | Spec |
| --- | --- |
| **Field overlay (fill)** | Native control over the widget, square corners (it belongs to the page), 1 px `control-border`, `field-tint` fill, page ink text in Source Sans 3 at the widget’s size (13 px at 110%). Focused: white fill + 2 px `accent` ring, offset 1. Required: `danger` asterisk after the printed label, `aria-required`. Check box 14 px square, radio 14 px circle with an 8 px ink dot |
| **Read-only field** | 1 px dashed `control-border`, white fill, 14 px lock glyph `text-3`, text `text-2`, `aria-readonly` |
| **Calculated field** | As read-only plus a Σ glyph at left, value 600 right-aligned, a 11 px caption naming the formula; an `<output>` |
| **Field error** | 2 px `danger` border; a field message under the field |
| **Field message** | Page overlay (layer 1, like highlights) directly under the field: `danger-tint`, 1 px `danger` edge, radius 4, padding `6px 8px`, 14 px warning glyph, 12 px text. `role="alert"`, referenced by `aria-describedby` |
| **Fields tab** | Rail tab: overline per page (“Page 1 · 12 fields”), rows 28 high (number 12 `muted` tabular · label 13 · state: ✓ `positive`, “Required” `danger`, “Read-only”/“Calculated” `muted`, “Script skipped” `warning`). Current: `accent-tint` + `accent` 600, `aria-current`. Summary variant: label/value rows |
| **Menu** | From a toolbar button: `surface`, `chip-border` edge, radius 6, padding 4, floating shadow, z 30. Items 32 high, padding `0 10px`, 16 px icon `text-3` + 14 px label; separators 1 px `hairline`; optional 12 `muted` footer. Active item `accent-tint`. `role="menu"` |
| **Listbox popup** | The dropdown field’s open list: as Menu, items 28 high, active `accent-tint` + inset 1 px `accent` ring |
| **Form bar (phone)** | `surface`, `hairline` top, sheet shadow, padding `8px 8px 8px 16px`: label 15/600 + position 13 `muted` tabular; hint 13 `text-3` (error: `danger`); row of 44 px Previous/Next (secondary, radius 6), ghost “Next required”, primary Done. Sits on the keyboard (`visualViewport`), replaces the bottom bar |
| **Choice sheet (phone)** | Bottom sheet (§7.16) at medium detent: title + “Required · field 7 of 27”, a radiogroup of 48 px rows (selected: `accent-tint`, `accent` 600, check icon), footer with Previous (secondary) and Next (primary), 44 high |
| **Design-mode field** | 1 px dashed `accent` edge, `field-tint` fill, field name in monospace 11 px `accent`. Selected: solid `accent` edge + 2 px `accent` outline, offset 2, and 8 handles (7 px white squares, 1 px `accent`). Size chip: `text` background, white 11 px tabular, radius 3, under the field |
| **Tab-stop badge** | 20 px pill, `accent` fill, white 11/600 tabular number, at the field’s top-left (−11, −11). `aria-hidden` |
| **Suggestion outline** | Pending: 2 px dashed `accent` outline, offset 2, with a type tag (20 high, `accent` fill, white 11/600) at the top-right. Current: solid outline. Accepted: becomes a design-mode field with a `positive-tint` / `positive` tag “Accepted · Text” |
| **Option list editor** | Inspector list inside a `control-border` box, rows 30 high with `hairline` separators, Move up and Remove 24 px icon buttons per row, “Add option” ghost link below |
| **Toolbar title** | In tool toolbars that replace a mode’s own (Form design): Newsreader 15/500 label at the left, Done (secondary) at the right end |

**Icons (proposed)**, same grid and stroke as design language §5:

| Name | Markup |
| --- | --- |
| Forward (chevron right) | `<path d="M9 5l7 7-7 7"></path>` |
| Chevron up | `<path d="M7 14l5-5 5 5"></path>` |
| Text tool (T) | `<path d="M6 6.5h12M12 6.5v12"></path>` |
| Text field | `<rect x="3.5" y="7" width="17" height="10" rx="1"></rect><path d="M7 10v4"></path>` |
| Check box | `<rect x="4.5" y="4.5" width="15" height="15" rx="1.5"></rect><path d="M8 12.5l3 3 5-6"></path>` |
| Radio group | `<circle cx="12" cy="12" r="7.5"></circle><circle cx="12" cy="12" r="3"></circle>` |
| Dropdown | `<rect x="3.5" y="7" width="17" height="10" rx="1"></rect><path d="M13 11l2 2 2-2"></path>` |
| List box | `<rect x="4.5" y="4.5" width="15" height="15" rx="1"></rect><path d="M8 9h8M8 12h8M8 15h5"></path>` |
| Push button | `<path d="M8 7.5h8a4.5 4.5 0 0 1 0 9H8a4.5 4.5 0 0 1 0-9z"></path>` |
| Date | `<rect x="4.5" y="5.5" width="15" height="14" rx="1.5"></rect><path d="M4.5 9.5h15M8.5 3.5v4M15.5 3.5v4"></path>` |
| Select (pointer) | `<path d="M6 4l12 7-5.5 1.5L10 18z"></path>` |
| Lock | `<rect x="5.5" y="10.5" width="13" height="9" rx="1.5"></rect><path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5"></path>` |
| Calculated (Σ) | `<path d="M17 5.5H7l5 6.5-5 6.5h10"></path>` |
| Import | `<path d="M12 4.5V15M8 11l4 4 4-4"></path><path d="M5.5 19.5h13"></path>` |
| Reset | `<path d="M5 8.5A7.5 7.5 0 1 1 4.5 13"></path><path d="M5 4v4.5h4.5"></path>` |
| Flatten (layers) | `<path d="M12 4.5l8 4-8 4-8-4z"></path><path d="M4 15.5l8 4 8-4"></path>` |
| Tab order | `<path d="M10 7h10M10 12h10M10 17h10M5 5.5v3M4 17h2"></path>` |
| Detect fields | `<path d="M4.5 9V4.5H9M15 4.5h4.5V9M19.5 15v4.5H15M9 19.5H4.5V15"></path><path d="M8 12h8"></path>` |
| Drag handle | six `<circle r="0.9">` at (9,7) (15,7) (9,12) (15,12) (9,17) (15,17) |

Export reuses Share; signature fields reuse Signed file.

**Shortcuts introduced.** None new beyond native form keys (Tab, Shift Tab, Space, arrows, Alt ↓ / ⌥↓ to open a dropdown) and ⌥↑ / ⌥↓ (Alt ↑ / Alt ↓) to move a tab stop while the tab-order list has focus. Form design, detection, import, export, reset and flatten are palette and menu only.
