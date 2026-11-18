# E8: form filling and data

**Status:** Draft

This plan fixes the ops of the eighth editor round, how the save planner gates them, the tests and fixtures, and the twelve pull requests that build the round. The round lets people fill the forms they already have. Every AcroForm field type takes a value with an appearance that pdf.js, PDFium and Acrobat draw the same way. Tab follows the form's order, required and read-only fields behave as marked, and Acrobat's formatting, validation and calculation helpers run natively, without JavaScript. Answers move in and out as FDF, XFDF or JSON, a library call fills a form from data, and a form can be reset, flattened or submitted. An XFA form is named plainly, with what Lamina PDF can and can't do with it.

Form filling is the second most requested editor feature in the Hacker News research (776 comments), and XFA-only forms are its most cited dead end (121 comments; [R3](../../research/hacker-news/pdf-users-on-hacker-news.md#r3-finish-forms-and-signatures-and-name-xfa-files-plainly)). The round has two hard parts. Appearances must look right in every viewer, which is where fillers usually fail ([06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map), rank 3). And most real forms carry scripts, which [ADR 0021](../../decisions/0021-pdf-javascript-policy.md) never runs: the round recognises Acrobat's helper calls as data and reports the rest.

- **Round:** [full-editor §E8](../full-editor.md#e8-form-filling-and-data), phase 5, milestone M8 Forms and signing
- **Feature-plan todo:** [E8b](../../todos/done/E8b-form-filling-feature-plan.md)
- **Follows:** ADR [0021](../../decisions/0021-pdf-javascript-policy.md) (scripts never run; helper calls run natively), [0005](../../decisions/0005-document-model.md) (the overlay and `LaminaPdfId`s), [0006](../../decisions/0006-op-log-and-undo.md) (ops and undo), [0007](../../decisions/0007-save-planner.md) (the save planner, DocMDP and FieldMDP), [0010](../../decisions/0010-platform-adapter.md) (capability flags), [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (privacy), [0018](../../decisions/0018-test-strategy.md) (test tiers), [0026](../../decisions/0026-error-and-diagnostic-contract.md) (error codes)
- **Builds on:** [E4](e4-markup-and-drawing-annotations.md) (the annotation model, E4e's text layout and `/DA` handling, the viewer's annotation layer from E4i and Comment mode from E4f as the UI pattern), [E5](e5-review-stamps-interchange.md) (E5d's `laminapdf-interchange` and E5e's flattening), [E2](e2-page-organisation.md) (E2e's content append API and E2r's field-level page checks)
- **Design:** [P01 Fill & Sign mode](../../design/paper/forms.md#p01-fill--sign-mode), [P02 Validation, formatting and calculated fields](../../design/paper/forms.md#p02-validation-formatting-and-calculated-fields), [P03 Form data](../../design/paper/forms.md#p03-form-data), [P04 Phone: fill a form field by field](../../design/paper/forms.md#p04-phone-fill-a-form-field-by-field)

## 1. Where things stand

The library already knows where widgets are, but nothing reads a field's value or writes one.

- **Fields.** `FieldId` exists as a typed ID with no view behind it. `laminapdf-model`'s `widgets.rs` lists a page's widgets with the field each belongs to, and computes fully qualified names with E2's `<partial>#<n>` rule (ISO 32000-2 §12.7.4.2). Nothing walks `/AcroForm /Fields`, resolves inherited `/FT`, `/Ff`, `/V`, `/DA` or `/Q`, or reads `/Opt`.
- **Signatures and locks.** The save planner finds signature fields, `/Lock` dictionaries and FieldMDP transforms (ADR 0007). Its FieldMDP check for form changes is coarse: "op classes don't name fields yet", so any form change counts against every lock. E2r already judges page ops field by field (`Restriction::FieldMdp`, `Restriction::SignedWidget`).
- **Appearances.** `laminapdf-build/src/annot/appearance` draws notes, markup, ink, shapes and FreeText; FreeText parses `/DA` and lays text out with `laminapdf-text`. No builder draws a field value, a check box state or a list box.
- **Flattening.** E5e's `FlattenAnnots` refuses widgets (`flatten-widget`) and skips them when it picks candidates, leaving forms to this round. Its appearance and matrix functions live in `laminapdf-model`, so another flattener can share them.
- **Scripts and XFA.** `doc.disclosure()` (T2c) lists every action with JavaScript sizes, and records whether the form has `/XFA` (`disclosure/forms.rs`). Nothing tells static XFA from dynamic, and nothing recognises a helper call.
- **Interchange.** [E5d](../../todos/ready/E5d-xfdf-fdf.md), still ready, builds `laminapdf-interchange` with XFDF and FDF readers and writers for annotations, and leaves form fields to this round.
- **The viewer.** `@laminapdf/viewer`'s annotation layer puts a button over each annotation in an editing mode; PDFium draws every widget from its saved appearance. `@laminapdf/ui`'s tool list names a future `formFill` mode.
- **The matrix.** Every row in scope has a row in `docs/capabilities/manifest.toml` with `round = "E8"`. E.17 targets `read`; the others targeted `round-trip` until this plan (§2).

## 2. The operations

The round implements 16 rows of the [feature inventory](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms). E.15, signature fields, is E9's.

| ID | What it means | Ops or function (§2.1–§2.4) | Target | Todos (core and `pdf` · web · desktop) |
| --- | --- | --- | --- | --- |
| E.01 | Text fields (single and multi-line) | `SetFieldValue` | write | E8c, E8d · E8e · E8m |
| E.02 | Check boxes and radio buttons | `SetFieldValue` | write | E8c, E8d · E8e · E8m |
| E.03 | Dropdowns and list boxes | `SetFieldValue` | write | E8c, E8d · E8e · E8m |
| E.04 | Push buttons | `form_actions`, the action runner | read | E8i · E8i · E8m |
| E.05 | Date and formatted input | `field_behaviour`, `SetFieldValue` | round-trip | E8f · E8j · E8m |
| E.06 | Required and read-only | `fields` | round-trip | E8c · E8e · E8m |
| E.07 | Input validation | `field_behaviour`, `validate` | read | E8f · E8j · E8m |
| E.08 | Automatic calculations | `field_behaviour`, `calculate` | read | E8f · E8j · E8m |
| E.09 | Tab order | `tab_order` | read | E8c · E8e · E8m |
| E.10 | Conditional behaviour | `SetFieldHidden`, the action runner | round-trip | E8f · E8i · E8m |
| E.11 | Import and export form data | `import_form_data`, `export_form_data` | round-trip | E8g · E8k · E8m |
| E.12 | Populate fields programmatically | `fill_form` | round-trip | E8g · n/a · n/a |
| E.13 | Reset form fields | `reset_fields` | round-trip | E8g · E8k · E8m |
| E.14 | Flatten forms | `FlattenForm` | round-trip | E8g · E8k · E8m |
| E.16 | Submit to external endpoints | `submit_request`, `Platform.submitForm` | write | E8i · E8i · E8m |
| E.17 | XFA forms (cross-cutting) | `xfa`, `RemoveXfa` | read | E8h · E8h · E8m |

**Why some targets are below `round-trip`.** Seven rows (E.01–E.04, E.07–E.09) name a field type or setting that E9 lets people create. A row reaches `round-trip` once, when it can be both filled and authored, as B.01 and B.02 did across E6 and E7 ([E7 plan §1](e7-editing-existing-content.md#1-where-things-stand)).

Recommendation: this plan lowers those rows' targets to what filling reaches: `write` where filling changes the field, `read` where filling only follows it (a push button's action, a validation rule, a calculation, the tab order). [E9a](../../todos/ready/E9a-form-authoring-feature-plan.md) moves them to `round = "E9"` at `round-trip`, as E7b did for B.01 and B.02. A surface may pass its row's target: E8f proves E.07 and E.08 at `round-trip` in core, since a calculated value survives a save.

E.16 targets `write`: a submit sends data and changes nothing in the file, so it has nothing to round-trip. E.17 stays at `read`: the XFA packets are exposed whole, and Lamina PDF doesn't lay out XFA (§2.4).

### 2.1 Ops

Ops name fields by `FieldId` and carry every value they write (ADR 0006 §1). `SetFieldValue` regenerates the appearances of the widgets it changes (§2.2), and no op needs a full rewrite. Choosing a value inside an op would be an ambient input, so the façade resolves every "all fields" or "the form's default" before it builds the op.

| Op | Payload | Class | `requires_rewrite` | Todo |
| --- | --- | --- | --- | --- |
| `SetFieldValue` | `{ field: FieldId, value: FieldValue }`. `FieldValue` is `Text(String)`, `Button(Option<String>)` (an on-state name, `None` for Off), `Choice(Vec<String>)` (export values, several for a multi-select list) or `Clear` (removes `/V`) | `FormFill` | false | E8c, appearances E8d |
| `SetFieldHidden` | `{ field: FieldId, hidden: bool }`: sets or clears the Hidden flag (ISO 32000-2 §12.5.3) on every widget of a terminal field, what a Hide action does to a field (§12.6.4) | `Annotation` (§5) | false | E8f |
| `RemoveXfa` | `{}`: removes `/XFA` from the interactive form dictionary, and the catalog's `/NeedsRendering`, so every viewer shows the AcroForm fields | `Document` | false | E8h |
| `FlattenForm` | `{ fields: Vec<FieldId>, intent: View \| Print }`: draws each listed field's widgets into its page and removes the fields | `Content` | false | E8g |

**`SetFieldValue` checks the value against the field.** A text value longer than `/MaxLen` fails with `field-value-invalid`. So does a choice not in `/Opt` (unless the combo box has the Edit flag), an on-state name no widget has, or a value of the wrong kind. A signature field fails with `field-is-signature`: signing is E10's. For a radio group, the op sets the parent's `/V` and each kid's `/AS`, and follows RadiosInUnison; for a check box with several widgets, every widget's `/AS` (§12.7.5.2). A button field with `/Opt` takes the export value and maps it to the on-state index (04 §3.2). A multi-select list keeps `/I` in step with `/V` (§12.7.5.4). Setting a text value drops `/RV`, whose rich text would no longer match.

**The ReadOnly flag binds the interface, not the library.** ISO 32000-2 §12.7.4.1 says the user may not change a read-only field. The reader refuses typing into one, while `SetFieldValue` accepts it, because calculated fields are usually read-only and calculation writes them. A library caller that wants the interface's rule reads `Field::read_only`.

**Reset, import and fill are transactions of `SetFieldValue`.** E5 made an annotation import a transaction of ordinary ops rather than one composite op ([E5 plan §10](e5-review-stamps-interchange.md#10-decisions-on-the-open-questions)). This round does the same: `reset_fields`, `import_form_data` and `fill_form` each build one transaction of `SetFieldValue` ops, one undo step. This replaces the `ResetFields` op that E8c named and the `ImportFormData` op that E8g and P03 named, and keeps one way to change a value on the wire. Reset takes each field's `/DV`, or `Clear` where it has none (§12.7.6.3).

**`FlattenForm`.** For each widget of each listed field, the op picks the normal appearance through `/AS` and draws it into its page with E5e's functions (`wrap_existing`, `add_xobject`, the §12.5.5 matrix). It then removes the widget from `/Annots` and the field from the form's tree, along with its `/CO` entry. These rules apply:

- A signature field is never flattened. `FlattenForm` refuses one in `fields` with `field-is-signature`, as E8g specified, and the façade's "all" lists only non-signature terminal fields. Signature fields stay in `/AcroForm /Fields`, and `/SigFlags` stays.
- A widget without an appearance, or with a stale one while `/NeedAppearances` is true, gets one from E8d's builders first.
- Widgets hidden for the chosen intent are removed without being drawn, as E5e skips hidden annotations.
- When no field is left but signature fields, `/NeedAppearances`, `/CO` and `/XFA` go too. When fields are left, `/XFA` still goes: Acrobat would otherwise show the XFA form, whose fields no longer exist, and the op reports `xfa-removed`.
- In a tagged file, each drawn appearance is marked as an artifact and the field's `Form` structure element is removed, as E5e does for annotations; on a file that claims PDF/UA-1 the save planner raises E5e's `flatten-untagged` (§3).
- `FlattenAnnots` keeps refusing widgets (`flatten-widget`). Its error page gains a pointer to `flatten_form`, and the reader's Flatten comments dialog keeps leaving fields alone.

**No op for an action.** Pressing a button runs its action in the interface (§2.3); the action's effect is whatever op it leads to (`SetFieldValue` for a reset, `SetFieldHidden` for Hide). Nothing in a document starts an op by itself.

### 2.2 Fields, appearances and `/NeedAppearances`

E8c adds a typed form view to `laminapdf-model`, read tolerantly (ADR 0003). `doc.form()` walks `/AcroForm /Fields` with `Limits::max_tree_depth` and a cycle check, and reconciles it with the pages' `/Annots`: a widget no field reaches, and a field whose widget is on no page, are both kept and reported (04 §3.1). Each terminal field has its `FieldId`, fully qualified name, type, flags, value, default value, `/TU`, `/MaxLen`, options and widgets, with `/FT`, `/Ff`, `/V`, `/DV`, `/DA` and `/Q` inherited up `/Parent` (§12.7.4). Two fields with the same fully qualified name are one field and share a value.

E8d draws appearances, reusing E4e's `/DA` parser and text layout. A text field's value is laid out with its `/DA` font and size, auto-size for size 0, `/Q` alignment, comb cells, multiline wrapping, password bullets and `/MK /R` rotation, inside `/Tx BMC … EMC` (§12.7.4.3). A character the field's font can't draw is drawn in a fallback font the caller gives `SetFieldValue` by its bytes (`fonts`), subset, embedded and added to `/DR`, as FreeText's fallbacks are; without one, the fill fails with `annotation-glyph-missing` (§10, question 12). A document with no appearance generator refuses a text, combo or list fill with `appearance-unavailable`, as FreeText does, so no fill leaves a widget showing an old value. Check boxes and radio buttons only switch states and need no generator, unless a widget has no appearance yet.

A check box or radio button keeps its existing on-state appearances and gains one, a ZapfDingbats glyph from `/MK /CA`, only when it has none. A combo box draws like a text field; a list box draws its visible rows with the selected ones highlighted, from `/TI`. As for FreeText, the base WebAssembly module, which has no fonts, fails with `appearance-needs-text` ([E4 plan §2.2](e4-markup-and-drawing-annotations.md#22-where-appearances-are-generated)); the viewer opens documents in the extract module.

**`/NeedAppearances` true** (the round's first open question, §10). Opening a file never changes it. An op regenerates the appearances of the fields it changes and no others, and Lamina PDF never sets the flag (04 §3.5, item 8), which PDF 2.0 deprecates (§12.7.3). It doesn't clear a flag it finds either: fields it didn't touch may still need it in viewers that honour it. While the flag is true, Read mode shows the saved appearances, and the status bar's form note adds "Some answers may show only in Fill & Sign"; Fill & Sign's form controls show every value.

### 2.3 Behaviour, actions and submit

E8f implements ADR 0021 §2. Its recogniser reads each field's `/AA` `K`, `F`, `V` and `C` scripts as tokens, within a new `Limits::max_script_length`. It matches each script against the closed list of helper calls and stores the match as a typed `FieldBehaviour`: `NumberFormat`, `PercentFormat`, `DateFormat`, `TimeFormat`, `SpecialFormat`, `RangeValidate` or `SimpleCalculate`. Native code then applies it: keystroke filtering, the formatted appearance through E8d, range and date checks, and calculation in `/CO` order (§12.7.3).

**Calculations join the user's transaction** (E8f's open question). A change runs every `/CO` calculation once, in order, and adds a `SetFieldValue` for each calculated field whose value changes to the same transaction, with the same origin. Undo then reverts the change and its totals together, as P02 draws. `fill_form` and `import_form_data` calculate after the last value, once; `FillOptions::calculate` turns it off. A calculated field locked by FieldMDP is skipped and listed in the report, so filling an unlocked field never becomes a conflict through a total. The planner still refuses a `SetFieldValue` on a locked field from any source (§5).

**Formatting changes the appearance, never `/V`.** `/V` keeps `1234.5` and the appearance shows `$1,234.50`, as Acrobat does (04 §3.3). A value that fails validation is never committed: the form control keeps the typed text and `/V` keeps its last valid value (P02).

**Scripts that aren't helper calls are reported.** `doc.form_scripts()` lists every script in a form with its location, size and state (`handled`, `not handled`, `xfa`), ADR 0021 §3's inventory. A field whose behaviour depends on a script that isn't handled is filled without formatting or calculation, and P02's notice names it.

**Actions** (E8i). `form_actions(field)` reads a field's `/A` and `/AA` actions through `/Next` chains, within `Limits`, as typed actions. The reader's action runner, used when a button is pressed or a trigger fires, does only these:

| Action (§12.6.4, §12.7.6) | What the runner does |
| --- | --- |
| GoTo, Named (next, previous, first, last page) | Moves the view |
| URI | Asks first, naming the address, then opens it |
| ResetForm | Asks first, then `reset_fields` over the action's fields (`/Fields`, with the Include/Exclude flag resolved) |
| Hide | `SetFieldHidden` for each field target; an annotation target gets E4's `UpdateAnnot` flags (class `Annotation`) |
| SubmitForm | The submit dialog (below) |
| ImportData | Not run: it names a file path on the sender's computer. The reader offers Import form data… instead |
| JavaScript | Not run (ADR 0021 §1); the reader says so where the user clicked |
| Launch, GoToR, GoToE, every other | Not run; the reader names the action |

**Submit** (E8i) is off by default and confirmed per send ([full-editor §5](../full-editor.md#5-requirements-that-hold-for-every-round), req. 6). `submit_request(action)` builds the request without sending it: the URL, the format from the action's flags (HTML form encoding, FDF or XFDF; §12.7.6.2) and the body, and the empty required fields that block it. A SubmitPDF action, which would send the whole document, is refused (`submit-pdf-refused`): ADR 0011's first guarantee is that no request carries the document's bytes. The dialog names the button, the host, the format and each field sent, and says what isn't sent. Only `https:` addresses are allowed. A `mailto:` address opens the mail app with the address and no data, and offers Export form data… for the attachment.

### 2.4 XFA

E8h's `doc.xfa()` returns `None` or the form's XFA part: its packets (`template`, `datasets`, `config` and the rest) as bytes, and its kind. ISO 32000-2 deprecates XFA (the `XFA` entry, §12.7.3), and Lamina PDF builds no XFA layout engine (04 §3.4). The kind decides what the reader can offer:

- **Dynamic:** any of three signals. The catalog's `NeedsRendering` is true (ISO 32000-2 §7.7.2, deprecated with XFA), the `config` packet asks for dynamic rendering (`dynamicRender` is `required`), or the AcroForm has no terminal field while the template has fields. The pages are placeholders, usually Acrobat's "Please wait…" page.
- **Static:** anything else with `/XFA`. The pages are real, and the AcroForm fields fill as usual.

Acrobat shows a static form's XFA values, not the AcroForm's, so answers filled into the fields alone don't show in Acrobat. On the first change to a static XFA form, the reader adds `RemoveXfa` to the same transaction and says so in the notice, with Undo. A signed file keeps its XFA, since removing it is a `Document` change, and the notice warns that Adobe apps may show the old answers instead. The library never removes XFA by itself: `fill_form` reports `xfa-not-updated` and leaves the choice to the caller.

The reader's words, under the Fill & Sign toolbar (refining P01's strip, which the design area records):

- **Dynamic:** "**This form needs Adobe Acrobat or Adobe Reader.** It's an XFA form, an older Adobe format that only Adobe's apps can show or fill. Open it in Adobe Acrobat Reader, or ask the sender for a standard PDF form." Fill & Sign has nothing to fill, and the Fields tab says so.
- **Static:** "**This form also has an XFA version, which only Adobe's apps use.** Lamina PDF fills the standard form fields and removes the XFA version when you change an answer, so every app shows the same answers."

## 3. Conformance the edits must keep

- **Tagged PDF and PDF/UA-1.** Filling changes no structure. Each form control takes its accessible name from `/TU`, then the field name, as P01 specifies. `FlattenForm` follows E5e: the drawing is an artifact, the `Form` element and its `OBJR` go, and a PDF/UA-1 claim raises `flatten-untagged` (§10, question 6).
- **PDF/A.** A value drawn with a font the file doesn't embed would break a PDF/A claim, so on a file that claims PDF/A, E8d draws with an embedded font even where `/DA` names a standard 14 font. PDF/A forbids XFA, scripts and a true `/NeedAppearances` (ISO 19005-2 §6.4.1, §6.6); this round adds none of them.
- **Earlier revisions keep old answers.** An incremental save leaves earlier values in the previous revision (§7.5.6). `SetFieldValue` with `Clear` and `FlattenForm` set `still_in_earlier_revisions`, and the reader's history note says that Save as optimised removes them.
- **Nothing in a form runs.** No script runs (ADR 0021); no action reaches the network without the submit dialog; script text is shown as plain text, never as markup.

## 4. Crate and package changes

- **`laminapdf-model`**: the form view (`Form`, `Field`, `FieldKind`, `FieldFlags`, `FieldValue`, `Widget`), the reconciliation report, `tab_order(page)` (E8c); `form_actions` (E8i); the recogniser and `FieldBehaviour` in a new `forms::behaviour` module, and `form_scripts` (E8f); `xfa()` and the kind detector (E8h).
- **`laminapdf-ops`**: `SetFieldValue` (E8c), `SetFieldHidden` (E8f), `RemoveXfa` (E8h) and `FlattenForm` (E8g), each in a new `forms/` module, with the field-level FieldMDP check (§5).
- **`laminapdf-build`**: field appearance builders in `annot/appearance/field/` (E8d).
- **`laminapdf-interchange`** (from E5d): form fields in XFDF and FDF, and the JSON format (E8g); the HTML form encoding for submit (E8i).
- **`laminapdf-cos`**: `Limits::max_script_length`, through ADR 0003's process (E8f).
- **The façade and bindings** (requirement 7, each in the PR that adds the op or function). `laminapdf::Document` gains `form`, `tab_order`, `form_scripts`, `xfa`, `fill_form`, `reset_fields`, `import_form_data`, `export_form_data`, `flatten_form` and `submit_request`. `@laminapdf/sdk`'s `Document` gains the same as flat methods, following E5's pattern: `formFields`, `tabOrder`, `formScripts`, `xfa`, `fillForm`, `resetFields`, `importFormData`, `exportFormData`, `flattenForm` and `submitRequest`, with the ops through `apply`. Each has rustdoc or TSDoc, a runnable example, tsify types and a frozen sample in `fixtures/ops/v1/`. E8g's example fills an invoice form from JSON.
- **`@laminapdf/viewer`**: a form layer (`layers/form-layer.ts`) of native controls over each widget, synced with zoom and rotation, beside the annotation layer (E8e).
- **`@laminapdf/ui`**: the `formFill` mode (Fill & Sign) with its toolbar, the Fields rail tab and the locked and certified states (E8e); the action runner and the submit dialog (E8i); formatted input, validation messages, calculated fields and the script notice (E8j); the Form data menu and its dialogs (E8k); the phone's form bar and choice sheet (E8l); the XFA notice (E8h).
- **The `Platform` interface** (`@laminapdf/ui`'s `src/platform/`): `submitForm` and its flag (§6), an ADR 0010 amendment in E8i.
- **Size.** Every Fill & Sign screen loads with the editor's lazy chunk; the reader's startup scripts gain nothing. The recogniser and the appearance builders join the extract module, which E8d and E8f measure against ADR 0016 §6's budgets.

## 5. Saving and the save constraint

Every save plans first ([E1 plan §7](e1-editing-foundation.md#7-saving-and-the-save-constraint)). The round's value ops are class `FormFill`; `RemoveXfa` is `Document` and `FlattenForm` is `Content`. DocMDP P1 permits no change, P2 permits form filling and signing, and P3 adds annotations (ISO 32000-2 §12.8.2.2). FieldMDP and a signature field's `/Lock` restrict changes to the fields they name (§12.8.2.4, §12.7.5.5).

E8c makes the planner's FieldMDP check field-level for form ops: `SetFieldValue`, `SetFieldHidden` and `FlattenForm` name their field, so a lock counts only against the fields it covers (`/All`, `/Include`, `/Exclude`, with ADR 0007's rules for names that can't be read). This replaces ADR 0007 §9's interim rule that any form change counts against every lock, and E8c records it in the ADR.

| Constraint | When | What Save does | What the user sees |
| --- | --- | --- | --- |
| `Free` | No signature, a sound base | An incremental update | Nothing; after clearing or flattening, the note that earlier versions keep the old answers |
| `AppendOnly`: approval signatures | Signed, not certified | An incremental update for every op; the reader keeps XFA (§2.4) | The status-bar note "Signed file. Saving adds your changes after the signature; the signature stays valid." |
| `AppendOnly` with a FieldMDP lock or `/Lock` | Some fields locked | An incremental update. An op on a locked field is a violation (`Restriction::FieldMdp`). Reset, import, fill and calculation skip locked fields and list them. Flattening a locked field is a violation, so the façade's "all" leaves locked fields out | A locked field draws read-only, its Fields-tab row says "Locked by R. Osei's signature", and a change opens the conflict dialog (P01). P03's toasts and dialogs name the skipped fields |
| `AppendOnly` with DocMDP P2 or P3 | Certified for form filling | `FormFill` ops save incrementally; `SetFieldHidden` (class `Annotation`) saves under P3 and is a violation under P2; `FlattenForm` and `RemoveXfa` are violations. A PDF 2.0 `/Lock` with `/P` counts as DocMDP | The note while filling; the flatten dialog is the conflict confirmation (P03, "Flatten anyway") |
| `AppendOnly` with DocMDP P1 | Certified, no changes | Every op is a violation | The banner from entering Fill & Sign, and the conflict dialog at the first change (P01's `certifiedNoChanges`) |
| Usage rights (UR3) | Extended by Adobe Reader rights | `FormFill` ops need `/Form [/FillIn]`; `SetFieldHidden` needs `/Annots [/Modify]`, checked through `Op::annot_right` as for every annotation change; `FlattenForm` and `RemoveXfa` are violations (§12.8.2.3) | The note, or the banner when the rights don't cover filling |
| `RequiresRewrite` | A repaired file | A full rewrite | A neutral strip saying why |
| `RequiresRewrite`: encrypted base | Password-protected | Save is refused (`encrypted-base`) until E11 | The warning strip |
| `Conflict` | Ops the constraint forbids | Save is refused; Apply anyway… invalidates the named signatures after a confirmation (`with_invalidate_signatures`) | The danger strip naming each signature, with Save a copy… |

Hiding a field starts as an annotation change. `SetFieldHidden` changes a widget's `/F` flags, and ISO 32000-2 §12.8.2.2 doesn't name widget flag changes among what P2 permits; ADR 0007 errs towards reporting a violation. So `SetFieldHidden` is class `Annotation`: allowed under P3 and with approval signatures, a violation under P1 and P2, and held to UR3's `/Annots [/Modify]`. It is promoted to `FormFill` only if pyHanko's difference analysis and an Acrobat check both accept a hide on a certified P2 form; E8f ran pyHanko's check. pyHanko 0.38.0 rates an incremental update that hides a field on a form certified at P2 `FORM_FILLING`, the same as filling it: `docmdp_ok` is true. (Checked 2026-11-18 on `fixtures/forms/scripts.pdf` certified at P2. `fields.pdf` can't be checked, because pyHanko refuses its two fields that share a name.) The Acrobat check waits for the owner's optional reference files, so `SetFieldHidden` stays `Annotation`.

Exports and `submit_request` change nothing and work on any document, signed or encrypted once opened.

## 6. Differences between the shells

Filling, behaviour and form data are the same on both shells. The differences are fonts, files and how a submit leaves the device.

| Area | Flag | Web | Desktop |
| --- | --- | --- | --- |
| Fonts for a value the field's font can't encode | `systemFonts` (E6h) | Bundled fonts | Bundled fonts, then fonts on the computer |
| Import and export form data | `canSaveAsDialog` | The file picker; a save dialog in Chromium, else a download | Native dialogs |
| Submit | `canSubmitWithoutCors` (new, E8i) | `fetch` from the page: works only when the server allows cross-origin requests (CORS), else P01's "Not sent" message | The main process sends the request, so CORS doesn't apply; the request is allowed only to the `https:` host confirmed in the dialog, for that one request |
| The request after sending | none | The network indicator lists it with its host | The same |
| Phone | none: breakpoints | P04's form bar and choice sheet | — |

`Platform.submitForm(request)` sends a request that `submit_request` built and the user confirmed, and returns the status. E8i adds it and the flag to ADR 0010's tables, with the web value `false`.

**Server features:** none. A submit goes from the device straight to the form's own address, never through Lamina PDF's servers, and only after the user confirms that one send. ADR 0011 §3's guarantees 3 and 5 say the app contacts no origin but a remote document's host; a confirmed submit is a new exception. The owner decided on 2026-11-17 to amend ADR 0011 for it, in a PR that lands before E8i merges (§10, question 7).

## 7. Todos

Each todo is one reviewable pull request on the [todo template](../../todos/TEMPLATE.md)'s scale, under ~1,500 changed lines with tests. The core todos flip the `core` and `pdf` columns, the interface todos `web`, and E8m `desktop`, as in E5.

| ID | Todo | Depends on |
| --- | --- | --- |
| [E8c](../../todos/done/E8c-acroform-field-model.md) | The form view, `SetFieldValue`, `tab_order`, the field-level FieldMDP check | E8b |
| [E8d](../../todos/done/E8d-field-appearances.md) | Appearances for every field type's value; the `/NeedAppearances` rule | E4e, E8c |
| [E8e](../../todos/ready/E8e-field-widgets-ui.md) | Fill & Sign: the form layer, the Fields tab, tab order, required and read-only, locked and certified states | E1e, E4f, E8d |
| [E8f](../../todos/done/E8f-declarative-behaviour.md) | The helper-call recogniser, native formats, validation and calculation, `SetFieldHidden`, the script inventory and its corpus run | E8a, E8d |
| [E8g](../../todos/ready/E8g-form-data-and-flatten.md) | Form fields in XFDF and FDF, the JSON format, `fill_form`, `reset_fields`, `FlattenForm` | E2e, E5d, E5e, E8f |
| [E8h](../../todos/ready/E8h-xfa-detection-and-notice.md) | `xfa()`, the static and dynamic kinds, `RemoveXfa`, the reader's XFA notice | E8c, E8e |
| [E8i](../../todos/ready/E8i-form-actions-and-submit.md) | `form_actions`, the action runner, `submit_request`, `Platform.submitForm` and the submit dialog | E8e, E8f, E8g |
| [E8j](../../todos/ready/E8j-validation-and-calculation-ui.md) | P02 in the reader: formatted input, validation messages, calculated fields, the script notice | E8e, E8f |
| [E8k](../../todos/ready/E8k-form-data-ui.md) | P03 in the reader: the Form data menu, import, export, reset and flatten dialogs | E8e, E8g |
| [E8l](../../todos/ready/E8l-phone-form-filling.md) | P04: the phone's form bar and choice sheet | E8e, E8j |
| [E8n](../../todos/ready/E8n-flat-form-tools.md) | Flat forms: Fill & Sign's Text, Check and Date tools as presets of the text box | E4j, E8e |
| [E8o](../../todos/ready/E8o-live-form-differential.md) | The live form differential: names, types and values against pypdf and pdf.js over the corpus (split from E8c) | E8c |
| [E8m](../../todos/ready/E8m-desktop-forms.md) | Desktop: the round in the engine, native dialogs, submit from the main process, WebdriverIO suites; the round's close | E1h, E8h, E8i, E8j, E8k, E8l, E8n |

Every todo also depends on this plan (E8b). E8c starts first; E8d and E8f follow it, and the interface todos start once E8e has the form layer. E8g waits on E5d, which is not yet started, so it may become the round's longest wait; E8f and E8j don't need it. E8i also waits on the ADR 0011 amendment PR (§6).

## 8. Fixtures

Lamina PDF's own scripts write every new fixture, licensed like the rest of Lamina PDF (`MIT OR Apache-2.0`), with provenance in `fixtures/README.md`.

- **`fixtures/forms/`** (new, `tools/fixtures/forms.py`, no dependencies; E8c adds it and later todos extend it):
  - `fields.pdf` (every field type and flag, inherited and merged fields, an orphan widget and a field with no widget, two fields sharing a name, radios with `/Opt` and RadiosInUnison)
  - `text-layout.pdf` (auto-size, comb, multiline, quadding, `/MK /R` rotated widgets, password, a `/DA` naming `/Helv` missing from `/DR`, non-Latin values)
  - `need-appearances.pdf`
  - `scripts.pdf` (each helper call of ADR 0021 §2 in both spellings, `/CO` order, one script of each kind that isn't handled, an oversized script)
  - `actions.pdf` (push buttons with GoTo, URI, Named, ResetForm, Hide, SubmitForm in each format, SubmitPDF, ImportData, JavaScript and Launch, and a `/Next` chain)
  - `tabs.pdf` (pages with `/Tabs /R`, `/C`, `/S`, `/A`, `/W` and none)
  - `tagged-form.pdf` (PDF/UA-1, `Form` elements)
  - `pdfa-form.pdf` (PDF/A-2b, `/Helv` in `/DA`)
  - `xfa-static.pdf` and `xfa-dynamic.pdf` (minimal XFA packets written by the script)
- **`fixtures/forms/data/`** (E8g): JSON, XFDF and FDF data for `fields.pdf`, a file with unknown names, malformed files with a known line, and an XFDF with a DTD that must be refused.
- **Signed files** (`fixtures/signed/make.py`, pyHanko, MIT): the existing approval, `certified-p1`, `-p2`, `-p3`, `locked` and `usage-rights` files, plus `form-certified-p2.pdf` and `form-locked-include.pdf` (a FieldMDP `/Include` lock over two of `fields.pdf`'s fields), which E8c adds.
- **Real forms:** a US federal form with helper-call scripts, such as the IRS W-9. US federal government works are in the public domain (17 U.S.C. §105). E8f picks the form after checking it has no XFA, and records its URL, date and hash.
- **The pinned corpus** (nightly, never copied): pdf.js's `acroform_calculation_order`, `annotation-text-widget`, `annotation-button-widget`, `annotation-choice-widget`, `textfields`, `fields_order`, `resetform`, `widget_hidden_print`, `xfa_filled_imm1344e` and `xfa_issue14315`, and PDFBox's `AcroFormForMerge*` files.
- **Acrobat references (optional):** forms filled and saved by Acrobat, and its renders, as extra comparisons if the owner makes them later. They never block a todo (§10, question 8).

## 9. Tests

Each op gets the tests its todo lists ([ADR 0018](../../decisions/0018-test-strategy.md)). "Round-trip" means Lamina PDF, pdf.js, PDFium and qpdf (`--check` clean) each reopen the saved file and report the change and nothing else.

| Op or function | Unit | Golden | Round-trip | Differential | Fuzz | e2e |
| --- | --- | --- | --- | --- | --- | --- |
| Form view, `tab_order` (E8c) | inheritance, reconciliation, shared names, each `/Tabs` value | — | — | field names, types and values vs pypdf and pdf.js over the corpus (E8o) | a `form_tree` target over corpus forms | — |
| `SetFieldValue` (E8c, E8d) | each kind and refusal; radios, `/Opt`, `/I` | `fixtures/golden/forms/` per field type | pdf.js and PDFium read each value back and draw it within E4g's threshold | renders vs pdf.js and PDFium (§10, question 8) | `ops_json` gains the op | — |
| Behaviour (E8f) | every helper call, each format string, `/CO` order, locked calculated fields | — | a calculated total survives a save | displayed values vs pdf.js's on the real forms | an `af_script` target for the recogniser | — |
| `SetFieldHidden` (E8f) | Hidden set and cleared on each widget | — | pdf.js and PDFium hide the widget | pyHanko's difference analysis on `form-certified-p2.pdf` (§5) | `ops_json` | — |
| Form data, `fill_form`, reset (E8g) | each format and mode; unknown names; DTD refused | the XFDF, FDF and JSON of `fields.pdf` | export, import into a copy, export again: the same file | XFDF values vs pdf.js's `getFieldObjects` after import | `fdf`, `xfdf` gain fields; a `form_json` target | — |
| `FlattenForm` (E8g) | intents, hidden widgets, signature fields kept, tagged pages | `fields.pdf` flattened | PDFium's render of the flattened page matches the filled one within E4g's threshold; `/AcroForm` keeps only signature fields | the corpus forms flattened and rendered | `save_roundtrip` gains the op | — |
| `xfa`, `RemoveXfa` (E8h) | both kinds, packets as arrays and streams | — | pdf.js shows the AcroForm after removal | kind vs pdf.js's `isPureXfa` on the corpus files | an `xfa_packets` target | — |
| Actions and submit (E8i) | each action, `/Next` chains, each submit format, SubmitPDF refused | the body of each format for `fields.pdf` | — | — | `form_actions` joins `form_tree` | Playwright: no request leaves before Send (network assertions) |
| Interface (E8e, E8h–E8l) | Vitest for the mode's state, the dialogs and the Fields tab | — | — | — | — | Playwright per command, tagged with the row's ID; axe on each screen |
| Desktop (E8m) | — | — | — | — | — | WebdriverIO on macOS, Windows and Linux |

Across every op:

- apply then undo leaves the document overlay empty and the next save byte-identical (ADR 0006 §3);
- redo re-applies the same bytes;
- the same state saves to the same bytes on native and wasm32.

`would_violate` on every signed fixture gives §5's verdict, asserted per op, because a misclassified op is a security bug. veraPDF reports nothing new on `tagged-form.pdf` and `pdfa-form.pdf` after each op. A new fuzz target's `fixtures/regressions/<target>` folder joins `laminapdf-model`'s regression guard list.

## 10. Decisions on the open questions

The plan's recommendations, unless a decision names the owner. The owner decided questions 7 and 8 on 2026-11-17.

1. **`/NeedAppearances` true: regenerate on open or on edit?** On edit, for the fields an op changes. Opening a file never changes it; Lamina PDF never sets the flag and never clears one it finds (§2.2). Regenerating every appearance on open would make every opened form dirty, and would modify signed files nobody meant to change.
2. **The JSON schema: our own, or an existing one?** Our own, kept small: `{ "$schema": "https://laminapdf.com/schemas/form-data/v1.json", "fields": { "<fully qualified name>": value } }`, where a value is a string, `true` or `false` for a check box with one on-state, an array of strings for a multi-select list, or `null` for empty. It is the same object `fillForm` takes and `exportFormData` writes, so an export fills a form unchanged. pdf-lib has no data format, only an API. Nutrient's Instant JSON is a vendor's format tied to its own annotation model; a converter can come later if users ask. E8g publishes the schema with the docs.
3. **Calculations: their own transaction, or the user's?** The user's (§2.3), so one undo restores the value and its totals, as P02 draws.
4. **Reset and import: ops, or transactions?** Transactions of `SetFieldValue` (§2.1). The `ResetFields` op that E8c named and P03's `ImportFormData` are dropped; the user sees one undo step either way.
5. **Should an invalid entry survive closing the file?** ([P02](../../design/paper/forms.md#p02-validation-formatting-and-calculated-fields)'s open question.) No. An invalid entry is the form control's state, never in the document overlay or `/V`, so closing asks as saving does ("1 answer isn't valid and won't be saved") and then drops it.
6. **Flattened answers in a tagged file.** As E5e: artifacts, and `flatten-untagged` on PDF/UA-1. Answers are real content, though, and dropping them from the tags loses them for screen readers. E8g records how many tagged forms in the corpus this affects. A later todo, owned by E15a's plan, may tag flattened values as text instead.
7. **Submit and ADR 0011's guarantees.** A confirmed submit contacts an origin that isn't the document's host, which guarantees 3 and 5 rule out. **Decided by the owner on 2026-11-17:** ADR 0011 is amended so that a submit happens only after the person confirms that one send, seeing exactly what is sent and where. Submit is off by default, and it never sends the document's bytes. The amendment PR lands before E8i merges.
8. **Acrobat reference files.** **Decided by the owner on 2026-11-17:** pdf.js and PDFium are the round's references for E8d's renders and E8f's displayed values. Acrobat reference files are optional extras if the owner makes them later, as [issue #524](https://github.com/laudebugs/laminapdf/issues/524) asked for comments; they never block E8d or any other todo.
9. **Static XFA forms on the first change.** Recommendation: the reader removes the XFA part in the same transaction, except in a signed file (§2.4); the library leaves it to the caller. Updating the `datasets` packet instead would keep the XFA form in step, but needs XFA's binding rules, which this round doesn't build.
10. **Flat forms' Text, Check and Date tools** (P01). Kept, in their own todo, [E8n](../../todos/ready/E8n-flat-form-tools.md): presets of Comment mode's text box (E4j), so a flat form can be filled without fields. They add annotations, not fields, so no inventory row tracks them, but P01 draws them in Fill & Sign's toolbar and a flat form is the commonest form people get.
11. **The plan's location:** `docs/plans/editor/`, as E1–E7 use.
12. **Where a fallback font comes from.** **Decided by the owner on 2026-11-18:** from the caller, by its bytes, as FreeText's fallbacks do. `SetFieldValue` takes optional `fonts`; Lamina PDF bundles no font, which would grow every WebAssembly module by 60 to 100 KB gzipped and still miss most scripts. On a file that claims PDF/A, the first of those fonts draws the whole value, embedded; without one, the save warns `pdfa-font-not-embedded`.

The round's todos then build the form view and appearances first, behaviour and form data on top, and the interface last, so filling works in the library before the first screen lands.
