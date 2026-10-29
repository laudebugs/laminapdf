# Paper: Forms

Screens P01–P07 of the [screen inventory](screens.md): filling interactive forms in Fill & Sign mode, validation and calculated fields, form data (import, export, reset, flatten), filling on a phone, form design, the tab-order and accessible-label editor, and the review of automatically detected fields. Everything follows the [Paper design language](design-language.md); what this area adds is listed under [Proposed design-language additions](#proposed-design-language-additions).

- **Canvas:** [lamina Paper — Forms](https://claude.ai/artifact/Jat39ahU8QNgrb7TLi3mZR) (private until the owner shares it). The files on disk are newer than the published canvas until it is republished.
- **Plan:** [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and [§E9](../../plans/full-editor.md#e9-form-authoring); layout rules from the [UI/UX plan](../../plans/ui-ux.md) §4–§5; saving signed files per [ADR 0007](../../decisions/0007-save-planner.md).
- **Sample file:** `repair-request-form.pdf`, “Northwind Housing Trust — Repair request form”, 2 pages, 27 fields (12 on page 1, 15 on page 2), 9 required, 1 signature field. P07 uses a flat, older version of the same form, `repair-request-2019.pdf`. All names, references and addresses are fictional (`repairs.northwind-housing.example` uses the reserved `.example` domain).

Where forms live in the shell:

- **Filling** is the Fill & Sign mode. Its secondary toolbar (drawn on P01, canonical for the mode, identical on P01–P03): Previous field · Next field · Highlight fields │ Text · Check · Date │ Add a signature image · Sign with a certificate · Form data ▾ · `…` · context note “4 of 9 required fields filled”.
- **Designing** is the **Form design tool** of Edit mode: like Redact and Protect, it replaces Edit’s toolbar while active with a tool toolbar (“Form design” · field tools · Tab order · Detect fields · … · Done). The mode switcher keeps Edit selected.
- **Rail tabs** follow the cross-area rule: Pages · Outline, then contextual tabs. `repair-request-form.pdf` has a signature field and is in Fill & Sign or Form design, so its contextual tabs are Signatures and Fields: four tabs, so the rail is 280 px and shows three tabs plus a “More tabs” menu button. In Fill & Sign and Form design the mode’s own tab (Fields) is promoted to the third slot and Signatures moves into More tabs. `repair-request-2019.pdf` (P07) has no signature field: Pages · Outline · Fields at 220 px. The Fields tab changes its content with the tool: field list (filling and design), tab stops (Tab order), suggestions (Detect fields).
- **Every board** uses the full top bar: file name and save state, mode switcher, page n / 2, zoom − 110% +, find, Commands (⌘K desktop, Ctrl K web), Save, More.

Command ids follow the registry’s `area.verb-object` form ([U1b](../../todos/ready/U1b-command-registry-and-palette.md)). Every command is in the palette; only those listed with a shortcut have one.

## P01 Fill & Sign mode

**Purpose.** Fill an existing interactive (AcroForm) form in place, field by field, with every field type: text (single and multi-line), check box, radio group, dropdown and list box, date and formatted text, push button and signature field.

**Entry points.** The Fill & Sign mode button; the palette “Fill & Sign mode” (`mode.switch-fill-sign`); the status-bar item “Interactive form · 27 fields”, which switches to the mode. Clicking a field in Read mode does not fill it (Read never edits): it shows the hint “Fill in this form in Fill & Sign” with a button that switches mode and focuses the field.

**Layout.** Web shell (inverted L). Top bar with Fill & Sign selected and “5 unsaved changes”. The Fill & Sign toolbar. Rail (280 px) with the **Fields** tab. Canvas: the page with fields drawn as DOM overlays. No inspector while filling: the field itself is the control. Status bar: network indicator · “Interactive form · 27 fields · Tab follows the form’s row order” · page label.

**Content and controls.**

| Control | Does | Command id |
| --- | --- | --- |
| Field overlays | Native `<input>`, `<textarea>`, `<button aria-haspopup="listbox">`, radio and check box elements placed over each widget, synced with zoom and rotation. Typing is coalesced into one undo step per field visit | `form.set-value` (implicit) |
| Previous field / Next field | Move focus in tab order, across pages (the page scrolls to the field) | `form.focus-previous-field` (Shift Tab), `form.focus-next-field` (Tab) |
| Next required field | Palette only: jump to the next empty required field | `form.focus-next-required-field` |
| Highlight fields | Toggles the field tint; on by default for files with fields | `form.toggle-field-highlight` |
| Text · Check · Date | Place a free-text, tick or date annotation on a page without fields (a flat form) | `fill.add-text`, `fill.add-check`, `fill.add-date` |
| Add a signature image · Sign with a certificate | Open the signing flows (Sign area, G01–G02) | `sign.add-image`, `sign.with-certificate` |
| Form data ▾ | Opens the P03 menu | `form.open-data-menu` |
| `…` | Overflow: the toolbar’s commands that don’t fit, plus “Next required field” | `form.open-overflow` |
| Fields tab | One row per field in tab order: tab-stop number, label (accessible label, else field name), state (✓ filled, “Required”, error). Grouped by page; other pages collapse to “Show page 2 fields · 2 required”. Clicking a row focuses the field | `rail.show-fields` |
| More tabs | Menu with the tabs that don’t fit (Signatures) | `rail.open-more-tabs` |
| Push buttons | Run the file’s non-script actions: go to a page, reset form (asks first), show/hide fields, open a link (asks first, naming the address), submit (below) | — |

The drawn state: page 1 with six fields filled, the **Type of repair** dropdown focused and open (a listbox popup under the field, “Damp and mould” active), required fields marked with a `danger` asterisk, two check boxes (one ticked).

**States.**

- **Empty form:** every field tinted; the note reads “0 of 9 required fields filled”.
- **All required filled:** “9 of 9 required fields filled”; nothing else changes.
- **Certified “no changes” file (DocMDP P1)** (tweak `certifiedNoChanges`): nothing may change, so the constraint banner shows on entering the mode (the one exception to “banner on first edit”, ADR 0007 §3): “**Saving any answer will invalidate 1 signature.** Northwind Housing Trust certified this file to allow no changes, so the fields stay read-only. Save a copy to fill it in and keep the original signed.” with **Save a copy** and **Apply anyway…**. Fields draw read-only (dashed edge, no tint) and the dropdown can’t open. Apply anyway… opens an alertdialog listing each affected signature by signer; confirming (`invalidate_signatures`) makes the fields editable and the banner stays until saved or undone.
- **Certified for form filling (P2/P3) or approval signatures:** filling is allowed; the status bar shows “Signed file. Saving adds your changes after the signature; the signature stays valid.” (design language §7.13).
- **Field locked by a signature** (FieldMDP `/Lock` or a signature’s field-lock dictionary): the field draws read-only and its Fields-tab row says “Locked by R. Osei’s signature”; typing in it shows a toast “This field is locked by R. Osei’s signature” ([E8e](../../todos/ready/E8e-field-widgets-ui.md)).
- **XFA form:** a notice strip under the toolbar, “This form uses XFA, which lamina can’t fill. You can read it and fill the PDF fields it also contains, if any.” The Fields tab lists only AcroForm fields.
- **No fields:** the Fields tab empty state “No fillable fields” / “Use Text, Check and Date to fill a flat form, or design fields with Edit › Form design.”

**Behaviour.** Tab and Shift Tab follow the file’s `/Tabs` order (rows, columns or structure) and cross page boundaries. **Leaving the form:** Tab on the last field of the document (Shift Tab on the first) moves focus out of the page to the status bar (backwards: to the Fill & Sign toolbar), never wrapping, so there is no keyboard trap (WCAG 2.1.2); Escape in a field returns focus to the page itself (the page region, where single-key shortcuts and scrolling work), and a second Escape leaves the mode’s current tool. Region cycling: F6 / Shift F6 on desktop; on the web, where F6 belongs to the browser, Ctrl F6 / Ctrl Shift F6 **(proposed)** and the palette “Move to next region” (`shell.focus-next-region`). Enter in a single-line field moves to the next field; Space toggles check boxes; arrow keys move within a radio group; Alt ↓ (⌥↓) opens a dropdown, typing selects by first letters. Each field visit is one undo step (`SetFieldValue`). Values are stored with generated appearance streams ([E8d](../../todos/ready/E8d-field-appearances.md)). Saving is incremental; nothing is written until Save.

**Accessibility.** Each overlay is a native control named by the field’s accessible label (`/TU`), falling back to the field name; required fields carry `aria-required="true"`; radios sit in a `fieldset` with a `legend`; the open dropdown is a `listbox` with `aria-selected` options. Check boxes and radios are visually hidden native inputs inside their `<label>`, next to a drawn box; the focus ring is drawn on the whole label with `label:has(input:focus-visible)` (2 px `accent`, offset 2), so keyboard focus is always visible. The ring on the drawn board is on the dropdown. The tint is decoration: every field also has a 1 px `control-border` edge (3.40 on `field-tint`). The Fields tab is a `nav` of links with `aria-current` on the focused field.

**Platform differences.** Web and desktop identical (drawn: web). Desktop adds the window frame and ⌘ chips. Tablet uses the desktop shell with 44 px field hit areas (the overlay grows its hit area, not its drawing) and the bottom action bar instead of the listbox popup. Phone: P04.

**Artboards.** `P01-fill-and-sign.dc.html` (tweaks: Highlight fields, certified no-changes file); `P01-submit-confirm.dc.html` (tweak: send error none / http / blocked).

### Submit confirmation

A push button with a SubmitForm action never sends silently; submit is off by default and confirmed per send ([E8e](../../todos/ready/E8e-field-widgets-ui.md), principle 7). The dialog “Send this form to another computer?” sits over the dimmed shell (page 2, 19 unsaved changes) and names the button, the destination (scheme muted, host bold, path), the format, how many fields and which (“Show all 19 fields and values”), and what is **not** sent (the PDF, comments, other files). A trust note says sending makes “1 network request” that the status bar will list, and that lamina asks every time. Actions: “Don’t send” (focused by default, so Enter never sends by accident) and “Send”. After sending, a toast “Sent to repairs.northwind-housing.example” and the network indicator counts the request with its host (design language §7.8).

Failures (tweak `sendError`) appear inside the dialog as a `danger` notice with `role="alert"`, and the dialog offers “Export form data…” as the way out:

- **`http:` address** (any platform; on the web the page’s content security policy blocks it anyway): “**lamina can’t send to an http:// address.** It isn’t encrypted, so anyone on the network could read the answers. Ask Northwind for an https:// address, or export the form data and send the file yourself.” Send is removed.
- **Blocked by the server (web):** a cross-origin POST from the web app fails unless the server allows it (CORS): “**Not sent.** The server at repairs.northwind-housing.example didn’t accept a send from a web page. Use the lamina desktop app, or export the form data and send the file yourself. Nothing was uploaded.” Send stays (to retry).
- **Desktop:** sends through the app’s HTTP plugin, whose scope only allows `https:` hosts confirmed in this dialog; confirming adds this host for this one request. A network failure says “Not sent. lamina couldn’t reach repairs.northwind-housing.example (no connection).”

`role="dialog"`, `aria-modal`, labelled by the title, described by the first paragraph; the close button is “Close Send this form”. On the phone it is a bottom sheet at the large detent with full-width buttons.

**Todos:** [E8e](../../todos/ready/E8e-field-widgets-ui.md).

## P02 Validation, formatting and calculated fields

**Purpose.** Show the file’s own rules while filling: formatted input (dates, numbers, currency), validation errors, calculated totals, read-only fields, and which of the form’s scripts lamina handled.

**Entry points.** Happens while filling in Fill & Sign; the Fields tab and the toolbar note lead to fields that need attention.

**Layout.** As P01, on page 2, with the same toolbar and rail. Under the toolbar, a **script report notice** (warning tint) when the form has scripts. The note reads “8 of 9 required fields filled · 1 field needs attention” (the second part in `danger`).

**Content and controls.**

- **Formatted input:** a date field formats on leaving it (“29/10/26” becomes “29/10/2026”); number fields right-align with tabular figures; currency shows “£18.40”. Formats come from recognised Acrobat AF* patterns and run natively, without running the script ([E8f](../../todos/ready/E8f-declarative-behaviour.md)).
- **Validation error** (drawn on “Earliest visit date”, typed “31/09/2026”, focused): 2 px `danger` border plus the focus ring, and a **field message** under the field: warning glyph and “September has 30 days. Enter a real date, like 03/11/2026.” The Fields-tab row shows “Check date” in `danger` 600.
- **Calculated field** (“Total”, £98.40 = hours × rate + materials): dashed `control-border` edge, white fill, a Σ glyph, value 600 right-aligned, and the 12 px caption “Calculated: hours × rate + materials”. An `<output>` linked to its inputs; recalculated in `/CO` order whenever an input changes, inside the same undo step as the change.
- **Read-only field** (“Job number”, “Hourly rate”): dashed edge, white fill, lock glyph, `aria-readonly="true"`. Tab still stops on it so screen-reader users can read it.
- **Script report notice:** “**This form has scripts.** lamina runs the 3 it recognises (a total and two date formats) without running code, and skipped 1. Check the ‘Visit reference’ field by hand.” · Details · Dismiss notice. Details opens a panel listing each script, its field and Handled / Skipped. What happens to unrecognised scripts is ADR 0021’s decision ([E8a](../../todos/ready/E8a-adr-0021-javascript-policy.md)); the screen only reports it.

**States.** No scripts: no notice. All handled: no notice; the status bar says “Scripts: 3 handled”. Several errors: the note counts them (“3 fields need attention”) and the palette “Next field that needs attention” (`form.focus-next-invalid-field`) walks them. Hide actions: hidden fields disappear from the page and the Fields tab until their condition changes.

**Behaviour.** Validation runs when focus leaves the field. An invalid entry is **not committed**: the field’s value (`/V`) keeps its last valid value, while the overlay keeps the typed text so the person can fix it; the field shows the error until it is fixed or the person presses Escape (which restores the last valid value). Leaving the file or saving with uncommitted text asks “1 answer isn’t valid and won’t be saved: Earliest visit date” with “Go to field” and “Save without it”. Whether a pending invalid entry should survive closing the file is an open question for the forms feature plan ([E8b](../../todos/ready/E8b-form-filling-feature-plan.md)). A submit with invalid fields lists them in the submit dialog and disables Send. Undo restores the previous value and recalculates.

**Accessibility.** Error fields have `aria-invalid="true"` and `aria-describedby` pointing at the message, which is `role="alert"` the first time it appears. Calculated and read-only are said in words, not shown by the dashed edge alone. Contrast: `danger` on `danger-tint` 6.31; message text on the tint ≥ 13.5. Check boxes use the label focus ring (P01).

**Platform differences.** Identical on web and desktop (drawn: web). Phone: the message sits in the form bar (P04) under the field label, and the field’s border turns `danger`.

**Artboards.** `P02-validation.dc.html` (tweak: script notice on/off).

**Todos:** [E8f](../../todos/ready/E8f-declarative-behaviour.md).

## P03 Form data

**Purpose.** Move answers in and out of the form: import values (JSON, XFDF, FDF), export them, reset the form, and flatten it.

**Entry points.** Fill & Sign toolbar › **Form data ▾**; the palette; desktop menu bar File › Form data. No shortcuts.

**Layout.** Desktop window (drawn), Fill & Sign mode with the full top bar and toolbar, rail Fields tab showing a **summary** (Fields 27 · Filled 19 · Required left 0 · Signature fields 1). The menu drops from the toolbar button; the export and flatten dialogs sit over the same real shell, dimmed.

**Content and controls.**

| Item | Does | Command id |
| --- | --- | --- |
| Import form data… | Native open dialog (JSON, XFDF, FDF). Values for matching field names are set in one undo step; a toast “19 fields filled · 2 names not in this form · Details” with Undo | `form.import-data` |
| Export form data… | Export dialog (below) | `form.export-data` |
| Reset form… | Alert dialog “Reset all 27 fields?” / “Every answer goes back to the form’s default. You can undo this.” with Cancel and “Reset form” (danger filled) | `form.reset-fields` |
| Flatten form… | Flatten dialog (below) | `form.flatten-fields` |
| Menu note | Below the menu (outside `role="menu"`, linked by `aria-describedby`): “Import accepts JSON, XFDF and FDF. Values stay on this device.” | — |

**Export dialog** (“Export form data”, subtitle “Saves the answers as a separate file. The PDF doesn’t change.”): Format as radio rows with one-line explanations (JSON, selected · XFDF · FDF); Fields: “Filled fields only · 19” or “All fields, including empty ones · 27”; File name (prefilled, extension follows the format); help “Signature values are never exported. Choose where to save it next.” Buttons Cancel and “Export…” (native save dialog on desktop, a download on web). Close button “Close Export form data”.

**Flatten dialog** (`alertdialog`, “Flatten the form?”): the effect in plain words; scope radio: “Every field · 26” (selected) and “Only the fields selected in the Fields tab”, disabled while nothing is selected, with the reason under it (“No fields are selected. Select fields in the Fields tab to flatten only those.”); a note “**The signature field stays a field.** lamina never flattens signature fields, so the form can still be signed” ([E8g](../../todos/ready/E8g-form-data-and-flatten.md)); “You can undo this until you close the file. Nothing is written until you save.” Buttons Cancel (focused) and “Flatten 26 fields” (danger filled).

**Signed files and locked fields** (ADR 0007; [E8g](../../todos/ready/E8g-form-data-and-flatten.md) verdicts):

| Action | Approval signatures only | Certified P2/P3 (filling allowed) | Certified P1 (no changes) | Fields locked by a signature |
| --- | --- | --- | --- | --- |
| Import (FormFill) | Allowed, append-only save | Allowed | Blocked: the P1 banner is already showing; import fills nothing until Apply anyway… is confirmed | Locked fields are skipped; the toast names them: “2 fields locked by R. Osei’s signature weren’t changed” |
| Reset (FormFill) | Allowed | Allowed | As import | The reset dialog says “27 fields · 2 locked by R. Osei’s signature stay as they are” and leaves them |
| Flatten (Content) | Allowed, append-only save | Invalidates the certification: the dialog shows the warning (tweak `signed`), and after flattening the constraint banner appears with Save a copy / Apply anyway… | As P2/P3 | Locked fields are excluded: “26 fields · 1 locked field is left as it is”; flattening only them is refused |

Other states: import with no matching names shows a notice in the Fields tab (“None of the 21 names in repair-data.xfdf match this form.”) and changes nothing; a malformed file shows a dialog error (“This file isn’t valid XFDF (line 14: unclosed field element).”) and imports nothing.

**Behaviour.** Import is one `ImportFormData` op (class FormFill). Flatten is one `FlattenForm` op (class Content): appearances are stamped into page content and the fields removed; undo restores them. Export writes only the new data file; reset and flatten change nothing on disk until Save.

**Accessibility.** The menu is `role="menu"` with `menuitem`s and `role="separator"`, opened by a button with `aria-haspopup="menu"` and `aria-expanded`; arrow keys move, Escape closes and returns focus. Focus ring drawn on the first item (inset). Dialogs as design language §7.18; destructive confirms are `alertdialog` with Cancel focused. Radio rows use the label focus ring (P01).

**Platform differences.** Desktop (drawn) and web are the same; web has no File menu and Export downloads the file. Phone: Form data is in the More sheet; import uses the system file picker, export the share sheet.

**Artboards.** `P03-form-data.dc.html`, `P03-export-dialog.dc.html`, `P03-flatten-dialog.dc.html` (tweak: signed file).

**Todos:** [E8g](../../todos/ready/E8g-form-data-and-flatten.md).

## P04 Phone: fill a form field by field

**Purpose.** Fill a form on a phone without pinching: one field at a time, the page zoomed to it, with Previous / Next / Done always reachable above the keyboard.

**Entry points.** Tapping a field on the page; More › Fill in form (`form.start-filling`, starts at the first empty required field).

**Layout.** Phone shell. Top bar: Back, title “Repair request” over “5 of 9 required filled · 6 unsaved”, More. The page zooms so the focused field’s text is 16 px (no iOS input zoom) and scrolls it into view above the **form bar**, which replaces the bottom bar and rides on the system keyboard (drawn as a labelled placeholder: the real keyboard is the OS’s; the home indicator sits over it). For choice fields (dropdown, list box, radio group) there is no keyboard: a **choice sheet** opens at the medium detent (top 404 px).

**Content and controls.**

- **Form bar** (`role="toolbar"`, “Form navigation”): field label (15/600), position “9 of 27”, hint line (“Required · Day, month, year, like 14/08/2026”, or the error message in `danger`), then Previous field (`form.focus-previous-field`, 44 px), Next field (`form.focus-next-field`, 44 px), “Next required” (`form.focus-next-required-field`, ghost), Done (`form.finish-filling`, primary). Done closes the keyboard and returns the bottom bar.
- **Text field:** the overlay input on the page, 44 px high, `inputmode` from the field’s format.
- **Choice sheet** (“Type of repair”, “Required · field 7 of 27”): a modal sheet (`aria-modal="true"`) with a `radiogroup` of 48 px rows (selected: tint, check icon); a footer with Previous and Next (Next is primary and also commits the choice). The close button “Close Type of repair” keeps the value.
- **Check boxes and radios** on the page toggle with a tap; the form bar still shows (without a keyboard) so Next works.

**States.** Error: hint line in `danger`, field border `danger`. Read-only: the form bar says “Read-only”. Last field: Next becomes “Review” and opens the Fields sheet. Signature field: the bar offers “Sign” (opens G01). Certified P1 file: the banner text appears as a notice at the top of the page and fields don’t take focus until Apply anyway… (in More).

**Behaviour.** Next/Previous follow tab order; Done or the system back gesture leaves field-by-field filling. Autocorrect and autocapitalise off for formatted fields. Undo is in More (and shake-to-undo on iOS). Each field visit is one undo step.

**Accessibility.** All targets ≥ 44 px; focus moves to the field, not the bar, so VoiceOver reads label, value and hint (`aria-describedby` on the hint). The choice sheet traps focus while open and returns it to the field. Focus ring drawn on the date input and the selected sheet row (inset).

**Platform differences.** Phone (drawn). Tablet uses the desktop shell (P01) with the form bar docked above the keyboard. Android: the back gesture closes the sheet first, then leaves the field.

**Artboards.** `P04-phone-fill.dc.html`, `P04-phone-choice.dc.html`.

**Todos:** [E8e](../../todos/ready/E8e-field-widgets-ui.md).

## P05 Form design mode

**Purpose.** Create, place, resize and configure fields of every type (text, check box, radio group, dropdown, list box, push button, signature, and text fields with a date format), with names, accessible labels, options, flags, format, validation and calculation.

**Entry points.** Edit toolbar › Form design; palette “Form design” (`form-design.open-tool`); the Fields-tab empty state on a file without fields. No shortcut.

**Layout.** Desktop window (drawn), full top bar with Edit selected. Tool toolbar: “Form design” · Select · Text field · Check box · Radio group · Dropdown · List box · Button · Signature · Date │ Tab order · Detect fields · … · Snap to grid (toggle) · “27 fields” · Done. Rail (280 px) Fields tab: field names in `mono` with their type; selection syncs with the page. Canvas: every field in **design style** (dashed `accent` edge, field tint, name in `mono`). Inspector: the selected field. Status bar: “Arrow keys move the field 1 pt · ⇧ adds 10 · ⌥ resizes” (Windows/Linux: Shift, Alt).

**Content and controls.**

| Control | Does | Command id |
| --- | --- | --- |
| Field tools | Pick a tool, then drag on the page to draw the field, or click to place it at its default size; the new field is selected | `form-design.add-text-field`, `form-design.add-check-box`, `form-design.add-radio-group`, `form-design.add-dropdown`, `form-design.add-list-box`, `form-design.add-button`, `form-design.add-signature-field`, `form-design.add-date-field` |
| Date | PDF has no date field type: it creates a **text field with a date format** (AFDate), shown as its own tool because people look for it ([E9b](../../todos/ready/E9b-field-creation-ops.md)) | `form-design.add-date-field` |
| Select | Click selects; Shift-click adds; drag moves; 8 handles resize; a size chip (“218 × 20 pt”) shows while selected | `form-design.select-fields` |
| Snap to grid | Toggle (`aria-pressed`): snap to the page grid and other fields’ edges | `form-design.toggle-snap` |
| Inspector: Field name | Unique fully qualified name; a clash shows “Another field is called repair_type” and keeps the old name | `form-design.rename-field` |
| Inspector: Accessible label | The `/TU` text: screen readers say it and other apps show it as the tooltip | `form-design.set-field-label` |
| Inspector: Options | (Dropdown, list box) rows with Move up and Remove; Add option; “Let people type their own answer” | `form-design.add-option`, `form-design.move-option-up`, `form-design.remove-option` |
| Inspector: When filling in | Required · Read-only · Hidden when printed | — |
| Inspector: Validation, Format, Calculation | The native declarative settings ([E8f](../../todos/ready/E8f-declarative-behaviour.md)): Format (number, percent, date, phone, postcode), Validation (range, one of the options), Calculation (sum, average, product, smallest, largest of chosen fields). Text fields show all three; the drawn dropdown shows Validation only | — |
| Inspector: Position (pt) | X, Y, W, H inputs: the keyboard alternative to dragging | `form-design.move-field` |
| Duplicate · Delete field | Copy with a new unique name; delete (undoable) | `form-design.duplicate-field`, `form-design.delete-field` |
| Signature field (when selected) | “Lock these fields when signed” (FieldMDP `/Lock`) and seed values | — |
| Calculation order… | Palette and `…`: a dialog listing calculated fields with Move up / Move down (the `/CO` order) | `form-design.edit-calculation-order` |
| Done | Back to Edit’s toolbar | `form-design.close-tool` |

**States.** No fields yet: the Fields tab empty state offers Detect fields (P07). Several selected: the inspector shows shared properties and “3 fields”. **Signed files** (ADR 0007 §3, cross-area rule): entering Form design changes nothing, so no banner; certified files show the status-bar note “Certified file: changing fields will invalidate the signature”. The constraint banner appears with the **first field change**, names the signature, and offers Save a copy and Apply anyway… (alertdialog listing signatures by signer). Certified P1 files show the banner on entering, as in P01. With approval signatures only, locked fields and signature fields can’t be selected and say why. This follows [E9c](../../todos/ready/E9c-form-design-mode.md) and [E9b](../../todos/ready/E9b-field-creation-ops.md) (save after the override); note that [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) still says authoring on a signed file “is blocked unless signatures are removed”, which the todos have superseded and the plan should be updated to match.

**Behaviour.** Every change is an op (`CreateField`, `UpdateFieldDef`, `MoveWidget`, `RenameField`, `DeleteField`; class Document) and one undo step; a drag is one step on release. Saving is incremental. Switching to Fill & Sign shows the form as it will behave.

**Accessibility.** Fields on the page are `button`s named like “repair_type, dropdown, selected”; the Fields tab is a `listbox` synced with the page. Arrow keys move, Shift+arrows by 10 pt, Alt/⌥+arrows resize; the Position inputs are the alternative to every drag (WCAG 2.5.7). Focus ring drawn on the Accessible label input.

**Platform differences.** Desktop (drawn) and web identical. Not offered on the phone (“Form design needs a larger screen” in More); tablet supports it with 44 px handles and the Position inputs.

**Artboards.** `P05-form-design.dc.html`.

**Todos:** [E9c](../../todos/ready/E9c-form-design-mode.md), [E9b](../../todos/ready/E9b-field-creation-ops.md).

## P06 Tab order and accessible labels

**Purpose.** See and change the order Tab follows, and make sure every field has a label screen readers announce.

**Entry points.** Form design toolbar › **Tab order** (toggle, `form-design.toggle-tab-order`); the toolbar button “1 field has no accessible label” (`form-design.focus-next-unlabelled-field`), which jumps to the first one.

**Layout.** Desktop window (drawn), Form design with Tab order pressed (field tools paused, said in the toolbar). The rail’s Fields tab shows the tab-order view: an “Order on page 1” select (Rows, left to right · Columns, top to bottom · Document structure · Custom) with a one-line explanation, then the ordered list of tab stops. Canvas: a numbered **tab-stop badge** at each field’s top-left; a field without a label also shows a “No label” badge. Inspector: the selected field’s label and tab stop.

**Content and controls.**

- **Order select** sets `/Tabs` for the page (`/R`, `/C`, `/S`); any manual move switches it to Custom (`SetTabOrder`, class Document). Command `form-design.set-tab-order`.
- **Tab-stop list** (`ol`): drag handle, number, label, warning glyph for missing labels; the selected row adds Move up (`form-design.move-tab-stop-up`) and Move down (`form-design.move-tab-stop-down`, disabled on the last stop). Keyboard: ⌥↑ / ⌥↓ (Alt ↑ / Alt ↓) while the list has focus.
- **Clicking badges in sequence** on the page sets a custom order (“click fields in the order Tab should follow”); Escape stops.
- **Inspector:** warning notice “No accessible label. Screen readers say only ‘check box, not checked’.”; Accessible label (textarea); “Use nearby text” (`form-design.use-nearby-text`) fills it from the closest text; preview “Screen readers will say: ‘You may enter with a key if I’m not at home, check box, not checked’”; Tab stop number input (“of 12 on this page”), the non-drag alternative; “Also tag the field for screen readers (this file is tagged)”, which writes the Form structure element ([E15c](../../todos/ready/E15c-tagging-tools.md) completes form tagging); “Next field without a label”.

**States.** All fields labelled: the toolbar button disappears and the list shows no warnings. Untagged file: the tagging checkbox is replaced by “This file isn’t tagged. The label is still saved for screen readers that read tooltips.”; Document structure order is disabled with that reason.

**Behaviour.** Each move or label change is one undo step. Turning Tab order off returns to field tools; badges hide.

**Accessibility.** Badges are decoration (`aria-hidden`); the order is conveyed by the list, with `aria-current` on the selected stop. Focus ring drawn on the label textarea. The warning uses an icon and words, not colour alone.

**Platform differences.** Desktop (drawn) and web identical. Not on the phone.

**Artboards.** `P06-tab-order.dc.html`.

**Todos:** [E9c](../../todos/ready/E9c-form-design-mode.md).

## P07 Automatic field detection review

**Purpose.** Turn a flat (printed, or scanned and OCRed) form into a fillable one: lamina suggests fields from ruling lines, boxes, underscores and label text, and the user accepts, edits or rejects each. No suggestion is applied without user action ([E9d](../../todos/ready/E9d-automatic-field-detection.md)).

**Entry points.** Form design toolbar › **Detect fields** (`form-design.detect-fields`); the Fields-tab empty state on a file without fields.

**Layout.** Desktop window (drawn) on `repair-request-2019.pdf`, full top bar. Tool toolbar: “Form design” · “Field tools are paused while you review suggestions” │ Tab order · Detect fields (pressed) │ Accept all on this page · Reject all on this page · … · “Page 1: 12 suggestions · 4 accepted · 1 rejected” · Done. Notice strip: “**Suggested fields for this flat form.** lamina looked for lines, boxes and labels on this device. Nothing is added until you accept a suggestion.” Rail (220 px, Pages · Outline · Fields) with the Fields tab in suggestions view: one row per suggestion (label, type, Accept ✓ and Reject ✕; accepted and rejected rows show a badge), and a grouping hint “‘Contact: Phone’ and ‘Contact: Letter’ look like one choice” with **Group as one radio group**. Canvas: every pending suggestion has a 2 px dashed `accent` outline and a type tag (“Date”, “Signature”, “Check boxes · 2”, “Text, several lines”); accepted ones become design-style fields tagged “Accepted · Text”; rejected ones lose their outline. Inspector: the selected suggestion.

**Content and controls.**

- **Inspector** (“Suggested date field”, “Page 1 · Suggestion 5 of 12”): “Why lamina suggests it” (“A ruled line after the label ‘Date first noticed’, which reads as a date.”); Field type (Text, Date, Check box, Radio group, Dropdown, Signature); Field name; Accessible label (“Taken from the label beside the line.”); type-specific settings (Date format); Accept, Reject, Next.
- **Accept** (`form-design.accept-suggestion`) creates the field (`CreateField`, one undo step) with the shown settings and selects the next suggestion. **Accept all on this page** (`form-design.accept-page-suggestions`) is one undo step. **Reject** (`form-design.reject-suggestion`) only hides the suggestion; rejected rows can be restored from the row’s menu.
- **Group as one radio group** (`form-design.group-as-radio-group`): merges the selected check-box suggestions into one radio-group suggestion (one option per box, option names from their labels); a dropdown can be chosen instead in Field type.

**States.** Running: an indeterminate progress bar in the notice (“Looking for fields on 2 pages…”). Nothing found: empty state “No fields found” / “Draw fields with the field tools instead.” File already has fields: detection skips areas covered by existing fields and says so. Detection never uses the network; the status bar says “Detection ran on this device”.

**Behaviour.** Suggestions are not in the document or the undo history until accepted; leaving with pending suggestions asks “7 suggestions not reviewed. Leave them?”. No confidence numbers are shown; the reason line explains each suggestion.

**Accessibility.** Row buttons are named “Accept Room”, “Reject Room”; states use words in badges; the selected suggestion has `aria-current`. Focus ring drawn on the inspector’s Accept button. ↑ ↓ in the list, Enter accepts, Delete rejects (list or page focus only, never in inputs; single-key shortcuts follow Settings › Shortcuts).

**Platform differences.** Desktop (drawn) and web identical. Not on the phone.

**Artboards.** `P07-field-detection.dc.html`.

**Todos:** [E9d](../../todos/ready/E9d-automatic-field-detection.md).

## Proposed design-language additions

To be merged centrally into the design language.

### Tokens

- `field-tint` **(proposed)** = `accent` 8% over white, `#EDF2F3`: the fill of fillable fields while Highlight fields is on. Decoration only; the boundary is the 1 px `control-border` edge (3.40 on `field-tint`, 3.84 on white). Page ink `#262421` on it 13.71.
- Monospace for field names uses the cross-area `mono` token (`ui-monospace, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace`).

### §8.1 toolbars

- **Fill & Sign:** Previous field · Next field · **Highlight fields** (toggle) │ Text · Check · Date │ Add a signature image · Sign with a certificate · **Form data ▾** · `…` · context note. Every button has a 16 px icon plus label.
- **Form design** (a tool of Edit, replacing Edit’s toolbar): “Form design” title · Select · Text field · Check box · Radio group · Dropdown · List box · Button · Signature · Date │ Tab order · Detect fields · … · Snap to grid · field count · Done. While Tab order or Detect fields is on, the field tools are replaced by a muted “Field tools are paused…” note.
- **Rail:** in Fill & Sign and Form design, the Fields tab is promoted to the third slot; other contextual tabs go into More tabs.

### Components

| Component | Spec |
| --- | --- |
| **Field overlay (fill)** | Native control over the widget, square corners (it belongs to the page), 1 px `control-border`, `field-tint` fill, page ink in Source Sans 3 at the widget’s size (13 px at 110%). Focused: white fill + 2 px `accent` ring, offset 1. Required: `danger` asterisk after the printed label, `aria-required`. Check box 14 px square, radio 14 px circle with an 8 px ink dot, both visually hidden native inputs inside their `<label>`; focus ring on the label via `label:has(input:focus-visible)` |
| **Read-only field** | 1 px dashed `control-border`, white fill, 14 px lock glyph `text-3`, text `text-2`, `aria-readonly` |
| **Calculated field** | As read-only plus a Σ glyph, value 600 right-aligned, a 12 px caption naming the formula; an `<output>` |
| **Field error** | 2 px `danger` border; a field message under the field |
| **Field message** | Page overlay (layer 1, like highlights) under the field: `danger-tint`, 1 px `danger` edge, radius 4, padding `6px 8px`, 14 px warning glyph, 12 px text. `role="alert"`, referenced by `aria-describedby` |
| **Fields tab** | Rail tab: overline per page (“Page 1 · 12 fields”), rows 28 high (number 12 `muted` tabular · label 13 · state: ✓ `positive`, “Required” `danger`, “Read-only”/“Calculated” `muted`, “Script skipped” `warning`). Current: `accent-tint` + `accent` 600, `aria-current`. Summary variant: label/value rows. Tab-order and suggestions views as P06 and P07 |
| **More tabs button** | 26 px icon button (chevron down) at the right end of the rail tab row, outside the `tablist`, `aria-haspopup="menu"` |
| **Menu** | From a toolbar button: `surface`, `chip-border` edge, radius 6, padding 4, floating shadow, z 30. Items 32 high, padding `0 10px`, 16 px icon `text-3` + 14 px label; separators 1 px `hairline`; an optional 12 `muted` note below the items, outside `role="menu"` and linked with `aria-describedby`. Active item `accent-tint` |
| **Listbox popup** | A dropdown field’s open list: as Menu, items 28 high, active `accent-tint` + inset 1 px `accent` ring, `role="listbox"` |
| **Form bar (phone)** | `surface`, `hairline` top, sheet shadow, padding `8px 8px 8px 16px`: label 15/600 + position 13 `muted` tabular; hint 13 `text-3` (error: `danger`); 44 px Previous/Next (secondary, radius 6), ghost “Next required”, primary Done. Rides on the keyboard (`visualViewport`), replaces the bottom bar |
| **Choice sheet (phone)** | Modal bottom sheet (§7.16) at the medium detent: title + “Required · field 7 of 27”, a radiogroup of 48 px rows (selected: `accent-tint`, `accent` 600, check icon), a footer with Previous (secondary) and Next (primary), 44 high |
| **Design-mode field** | 1 px dashed `accent` edge, `field-tint` fill, field name in `mono` 11 px `accent`. Selected: solid `accent` edge + 2 px `accent` outline, offset 2, and 8 handles (7 px white squares, 1 px `accent`). Size chip: `text` background, white 11 px tabular, radius 3, under the field |
| **Tab-stop badge** | 20 px pill, `accent` fill, white 11/600 tabular number, at the field’s top-left (−11, −11). `aria-hidden` |
| **Suggestion outline** | Pending: 2 px dashed `accent` outline, offset 2, with a type tag (20 high, `accent` fill, white 11/600) above it; current: solid outline; accepted: becomes a design-mode field with a `positive-tint` / `positive` “Accepted · Text” tag |
| **Option list editor** | Inspector list in a `control-border` box, rows 30 high with `hairline` separators, Move up and Remove 24 px icon buttons per row, “Add option” ghost below |
| **Tool toolbar title** | Newsreader 15/500 label at the left of a tool toolbar, Done (secondary) at the right end |

### Icons

Same grid and stroke as design language §5.

| Name | Markup |
| --- | --- |
| Forward (chevron right) | `<path d="M9 5l7 7-7 7"></path>` |
| Chevron up | `<path d="M7 14l5-5 5 5"></path>` |
| Text tool (T) | `<path d="M6 6.5h12M12 6.5v12"></path>` |
| Certificate | `<circle cx="12" cy="9.5" r="5"></circle><path d="M9 14l-1.5 6 4.5-2 4.5 2-1.5-6"></path>` |
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

### Shortcuts

No new letter shortcuts. Native form keys (Tab, Shift Tab, Space, arrows, Alt ↓ / ⌥↓ to open a dropdown); ⌥↑ / ⌥↓ (Alt ↑ / Alt ↓) to move a tab stop while the tab-order list has focus; Enter and Delete on a focused suggestion (single-key rules apply); web region cycling Ctrl F6 / Ctrl Shift F6 **(proposed)** because the browser owns F6. Form design, detection, import, export, reset and flatten are palette and menu only.
