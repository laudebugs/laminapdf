# E9: form authoring

**Status:** Draft

This plan fixes the ops of the ninth editor round, how the save planner gates them, the tests and fixtures, and the ten pull requests that build the round. E8 lets people fill the forms they already have; E9 lets them make one. Form design mode draws, moves, resizes and configures every field type, signature fields included, with labels, options, behaviours and order. On a flat form, Lamina PDF can suggest the fields itself, and nothing is added until the user accepts a suggestion.

Authoring is harder to get right than it looks, for two reasons. A field Lamina PDF creates must fill the same way in pdf.js, PDFium and Acrobat, so every entry it writes has to be one those viewers already read. And every authoring change is a change to the document's structure, and no certification level permits one (ISO 32000-2 §12.8.2.2), so the round's save rules matter more than its drawing tools.

- **Round:** [full-editor §E9](../full-editor.md#e9-form-authoring), phase 5, milestone M8 Forms and signing
- **Feature-plan todo:** [E9a](../../todos/done/E9a-form-authoring-feature-plan.md)
- **Follows:** ADR [0021](../../decisions/0021-pdf-javascript-policy.md) (scripts never run; §4: authoring writes helper calls as data), [0005](../../decisions/0005-document-model.md) (the overlay and `LaminaPdfId`s), [0006](../../decisions/0006-op-log-and-undo.md) (ops and undo), [0007](../../decisions/0007-save-planner.md) (the save planner, DocMDP and FieldMDP), [0010](../../decisions/0010-platform-adapter.md) (capability flags), [0018](../../decisions/0018-test-strategy.md) (test tiers), [0026](../../decisions/0026-error-and-diagnostic-contract.md) (error codes), [0030](../../decisions/0030-extraction-output-and-model-plugins.md) (rules in the core, models as plug-ins)
- **Builds on:** [E8](e8-form-filling.md) (the form view, `SetFieldValue`, E8d's appearance builders, E8f's behaviours, E8e's form layer and Fields tab), [E4](e4-markup-and-drawing-annotations.md) (annotation tagging and the viewer's move and resize gestures), [E6](e6-adding-content.md) (one op per added object, snapping in Edit mode)
- **Design:** [P05 Form design mode](../../design/paper/forms.md#p05-form-design-mode), [P06 Tab order and accessible labels](../../design/paper/forms.md#p06-tab-order-and-accessible-labels), [P07 Automatic field detection review](../../design/paper/forms.md#p07-automatic-field-detection-review)

## 1. Where things stand

The library reads and fills every field type, but no op creates, changes or removes a field.

- **The form view.** `laminapdf-model`'s `forms` module (E8c) gives `doc.form()`: each `TerminalField` with its `FieldId`, fully qualified name, `/TU` (`tooltip`), `FieldKind`, `FieldFlags`, value, default value, `/DA`, `/Q`, `/MaxLen`, `/Opt` and widgets, inherited up `/Parent` (ISO 32000-2 §12.7.4), and the reconciliation report (orphan widgets, widgetless fields, shared names). `doc.tab_order(page)` reads `/Tabs` (§12.5.1, Table 31) and orders the page's widgets.
- **Behaviour.** E8f's recogniser reads a field's `/AA` `K`, `F`, `V` and `C` scripts as helper calls and stores them as `FieldBehaviour` (`NumberFormat`, `PercentFormat`, `DateFormat`, `TimeFormat`, `SpecialFormat`, `RangeValidate`, `SimpleCalculate`). Calculations run in `/CO` order (§12.7.3). Nothing writes a behaviour.
- **Ops.** `laminapdf-ops`' `forms/` module has `SetFieldValue` (class `FormFill`) and `SetFieldHidden` (class `Annotation`). `Op::form_field()` names the field each changes, and the planner holds that field to the locks that cover it (ADR 0007 §12). `widgets.rs` reports what page ops do to widgets (ADR 0007 §11).
- **Appearances.** `laminapdf-build/src/annot/appearance/field.rs` (E8d) draws a text, combo or list value inside `/Tx BMC … EMC`, the `/MK` border and background, and a check box's ZapfDingbats glyph when a widget has no on-state appearance. It draws no push button caption and no empty signature field.
- **Tagging.** `laminapdf-ops`' `annot/tags.rs` puts a new annotation into the structure tree as an `Annot` element with an `OBJR`, and `link/tags.rs` does the same for a `Link`; both use `parent_tree.rs`. No code writes a `Form` element.
- **The planner and authoring.** A class `Document` op names no field, so the planner judges it by class alone. DocMDP at any level forbids it, and a field lock allows it (`Rule::FieldMdp` refuses only `FormFill` by class). An op that renamed or deleted a locked field under an approval signature would therefore save without a violation today. §5 closes that gap before any authoring op ships.
- **Detection.** `laminapdf-extract`'s `tables/edges.rs` finds a page's ruling lines (stroked segments, thin filled rectangles), and its line and block builders give text with positions. ADR 0030's `LayoutModel` labels 12 kinds of region, none of them a form field.
- **The matrix.** E.01–E.04 and E.07–E.09 had `round = "E8"`, with E8's lowered targets ([E8 plan §2](e8-form-filling.md#2-the-operations)). This plan moves them to `round = "E9"` at `round-trip`. E.15 has `round = "E9"`; L.06 has `round = "E15"`.

## 2. The operations

The round implements eight rows of the [feature inventory](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and writes the first half of a ninth, L.06.

| ID | What it means | Ops (§2.1–§2.4) | Target | Todos (core and `pdf` · web · desktop) |
| --- | --- | --- | --- | --- |
| E.01 | Text fields (single and multi-line) | `CreateField`, `UpdateFieldDef`, `RenameField`, `MoveWidget`, `DeleteField` | round-trip | E9b, E9e · E9c · E9j |
| E.02 | Check boxes and radio buttons | the same, with `AddWidget` and `RemoveWidget` | round-trip | E9b, E9e · E9c · E9j |
| E.03 | Dropdowns and list boxes | the same, with `/Opt` in `UpdateFieldDef` | round-trip | E9b, E9e · E9c · E9j |
| E.04 | Push buttons | `CreateField` with a caption and a form action | round-trip | E9b, E9e · E9c · E9j |
| E.07 | Input validation | `SetFieldBehaviour` (validate) | round-trip | E9g · E9h · E9j |
| E.08 | Automatic calculations | `SetFieldBehaviour` (calculate), `SetCalculationOrder` | round-trip | E9g · E9h · E9j |
| E.09 | Tab order | `SetTabOrder` | round-trip | E9g · E9h · E9j |
| E.15 | Signature fields | `CreateField` with `Signature`, `/Lock` and `/SV` | round-trip | E9f · E9c · E9j |
| L.06 | Accessible form labels | `tooltip` (`/TU`) on `CreateField` and `UpdateFieldDef`; the `Form` element | write (row stays E15's) | E9b · E9h · E9j |

A row's column flips in the todo that completes the row for that column. E9b creates every kind of field, but a field that can't be renamed, moved or given options isn't authored yet, so E.01–E.04 flip their `core` and `pdf` columns in E9e. E8f already proved E.07 and E.08 at `round-trip` in core for filling; E9g adds the writing half and tags its tests with both rows. The desktop columns flip in E9j, as in E8.

**L.06 stays in round E15.** E9 writes `/TU` and, in a tagged file, a `Form` element for each field it creates. E15c completes the row with tagging tools for fields Lamina PDF didn't create, which is why its round is E15. E9's tests tag L.06 at `write` in core and `pdf`, ahead of its round, as E6 did for B.12.

### 2.1 Field ops

Ops name fields by `FieldId` and widgets by `AnnotId`, take geometry in page space (ADR 0027), and carry every value they write (ADR 0006 §1). Every op in this section is class `Document`, except `CreateField` with a signature spec (§2.2), and none needs a full rewrite (`requires_rewrite` false).

| Op | Payload | Todo |
| --- | --- | --- |
| `CreateField` | `{ name: String, kind: FieldSpec, widgets: Vec<WidgetSpec>, tooltip: Option<String>, flags: FieldFlags, appearance: TextStyle, default_value: Option<FieldValue>, value: Option<FieldValue>, tagged: bool, fonts: Vec<BlobId> }` | E9b (`Signature` in E9f) |
| `DeleteField` | `{ field: FieldId }`: removes the field, its widgets from each page's `/Annots`, its `/CO` entry, its `Form` element, and parents left without kids | E9b |
| `UpdateFieldDef` | `{ field: FieldId, tooltip, flags, appearance, options, max_len, default_value, caption, action, lock, seed }`, each optional; a present `Option` field set to `None` removes the entry | E9e (`lock`, `seed` in E9f) |
| `RenameField` | `{ field: FieldId, name: String }`: the new fully qualified name, moving the field under other parents when its prefix changes | E9e |
| `MoveWidget` | `{ widget: AnnotId, rect: RectPt }`: moves or resizes a widget on its page and redraws its appearances | E9e |
| `AddWidget`, `RemoveWidget` | `{ field: FieldId, widget: WidgetSpec }` and `{ widget: AnnotId }`: a radio option, or the same field shown again on another page | E9e |

The payload types:

- `FieldSpec` is `Text { max_len, comb }`, `CheckBox`, `RadioGroup`, `ComboBox { options, editable }`, `ListBox { options, multi_select }`, `PushButton { caption, action: Option<ButtonAction> }` or `Signature { lock: Option<LockSpec>, seed: Option<SeedValue> }`.
- `WidgetSpec` is `{ page: PageId, rect: RectPt, export: Option<String>, border: Option<Border>, background: Option<Colour>, rotation: u16, print: bool }`.
- `TextStyle` is `/DA`'s font (a standard 14 face or a `BlobId`), size (0 for auto) and colour, with `/Q`.

**Names follow §12.7.4.2.** `name` is a fully qualified name. Its periods separate partial names, so `CreateField` reuses each existing non-terminal field on the path and creates the missing ones. A name that an existing terminal field holds fails with `field-name-taken`, as P05 asks ("Another field is called repair_type"). An empty partial name, or a name under a terminal field, fails with `field-name-invalid`. Both are new codes; E9b adds them to `codes.json` with their `docs/errors` pages. P05's Duplicate gives the copy a new name (`name_2`, `name_3`, …), so it never shares a value by accident.

**What `CreateField` writes.** A field with one widget is written merged with it, as Acrobat writes single fields; a radio group is a parent with one kid widget per option. The op creates `/AcroForm` when the document has none, with `/Fields`, a default `/DA` (`/Helv 0 Tf 0 g`) and `/DR` holding `/Helv` (Helvetica) and `/ZaDb` (ZapfDingbats); it never sets `/NeedAppearances` (§12.7.3, [E8 plan §2.2](e8-form-filling.md#22-fields-appearances-and-needappearances)).

**Appearances.** A check box's or radio button's on-state is named for its export value when that is a plain ASCII name, else the widget's index, with the value in `/Opt` (§12.7.5.2.4). Every widget gets its appearances in the same op through E8d's builders, extended in E9b to draw a push button's `/MK /CA` caption and a signature field's empty box. A field created in a document opened in the base WebAssembly module fails with `appearance-needs-text`, as fills do.

**Identity needs no `/PieceInfo`.** E6b gives each added page object an identity in its form XObject's page-piece dictionary, because page content has no object of its own ([E6 plan §2.1](e6-adding-content.md#21-ops)). A field and its widgets are objects, so a `FieldId` and an `AnnotId` survive a save as E4's annotation IDs do. `CreateField`, like `AddAnnot`, reports what it made in `Applied`: the field in a new `created_fields`, its widgets in `created_annotations`. The overlay allocates object numbers deterministically, so a replayed log creates the same IDs.

**Push buttons take a form action.** `ButtonAction` is `ResetForm { fields, include }` (§12.7.6.3) or `SubmitForm { url, format, fields, include }` (§12.7.6.2): the two actions a form author needs. Writing them needs nothing from E8i; E8i's action runner is what runs them in the web reader. URI, GoTo, Named and JavaScript actions on a button arrive with E17b's action editor (ADR 0021 §4).

**Moving changes only geometry.** `MoveWidget` keeps a widget on its page. A move to another page is `RemoveWidget` and `AddWidget` in one transaction, so the planner sees both pages.

**Deleting leaves the old field in earlier revisions.** `DeleteField` and `RemoveWidget` set `still_in_earlier_revisions`, as every delete does (§7.5.6).

### 2.2 Signature fields

E9f adds `FieldSpec::Signature`: an unsigned signature field (§12.7.5.5) that E10 signs. The widget may be visible or have a zero-size `/Rect` for an invisible signature. Creating the first one sets `/SigFlags` bit 1, SignaturesExist (§12.7.3); bit 2, AppendOnly, is the signer's to set (E10). `SetFieldValue` already refuses a signature field (`field-is-signature`).

- **`lock`** writes the field's `/Lock` dictionary (§12.7.5.5): `/Action` `All`, `Include` or `Exclude` with `/Fields` by fully qualified name, and the PDF 2.0 `/P` that certifies when this field is signed. P05's "Lock these fields when signed" writes it. The lock binds nothing until E10 signs the field; E10f then writes the FieldMDP transform that the planner enforces.
- **`seed`** writes `/SV` (§12.7.5.5, the seed value dictionary): the filters and `/SubFilter`s allowed, `/Reasons`, `/MDP`, and `/Ff` for the entries that are required. E9 writes and reads them; E10j's signing flow follows them.
- **A new signature field is class `Signature`.** ISO 32000-2 Table 257 lets P2 and P3 allow signing, and a later signer needs a field to sign: pyHanko's difference analysis accepts a new signature field after certification (its `SigFieldCreationRule`), and so does Acrobat, at least for an invisible one. So `CreateField` with `Signature` reports class `Signature`, allowed under P2 and P3 and refused under P1, while every other `CreateField` is `Document`. E9f checks pyHanko's verdict on `certified-p2.pdf` for a visible and an invisible field. If the visible one fails, a visible signature field stays `Document` and only an invisible one is `Signature`, and E9f records the result in §10. E10's sign op, which can sign into a new field, is class `Signature` too.

### 2.3 Behaviour, calculation order and tab order

These ops are E9g's, each class `Document`, none needing a rewrite.

| Op | Payload | Todo |
| --- | --- | --- |
| `SetFieldBehaviour` | `{ field: FieldId, format: Option<FormatSpec>, validate: Option<RangeValidate>, calculate: Option<SimpleCalculate> }`; `FormatSpec` is `Number`, `Percent`, `Date`, `Time` or `Special` with E8f's parameters | E9g |
| `SetCalculationOrder` | `{ fields: Vec<FieldId> }`: the new `/CO`, which must list exactly the form's calculated fields | E9g |
| `SetTabOrder` | `{ page: PageId, order: Rows \| Columns \| Structure \| Custom(Vec<AnnotId>) }`; `Custom` lists every widget on the page once | E9g |

**Behaviours are written as helper calls.** ADR 0021 §4 lets authoring write the helper calls as text: the user picks "Number, 2 decimals, $", and `SetFieldBehaviour` writes the JavaScript actions (§12.6.4) on the field's `/AA` triggers (§12.6.3): `F` and `K` for a format, `V` for a validation, `C` for a calculation. The writer builds each call from typed parameters only, in Acrobat's spelling, and a property test proves that E8f's recogniser reads back exactly the behaviour written. Nothing runs. A trigger that already holds a script that isn't a helper call is replaced only when the payload sets it, and the op's result lists each script it replaced, so the web reader can name each script it replaced. Setting `calculate` adds the field to the end of `/CO`; clearing it removes the field. P05's "one of the options" validation for a dropdown is no script: it is a combo box without the Edit flag.

**PDF/A forbids these scripts.** PDF/A-2 and -3 forbid JavaScript actions (ISO 19005-2 §6.6.1) and a widget's or field's `/AA` (§6.4.1, interactive forms; §6.6.2 covers the catalog's and pages' `/AA`). On a file that claims PDF/A, `SetFieldBehaviour` with any helper fails with `pdfa-field-actions`, and the inspector disables those sections and says why. §6.6.1 also forbids actions by type: Launch, Sound, Movie, ResetForm, ImportData, Hide, SetOCGState, Rendition, Trans, GoTo3DView and JavaScript. SubmitForm isn't among them. So E9b adds `pdfa-field-actions` with its `docs/errors` page and refuses a push button's ResetForm on a PDF/A file; a SubmitForm button is allowed unless §6.4.1 also bars the widget's `/A`, which E9b checks against the clause and veraPDF before it lands. E9g reuses the code.

**Tab order.** `Rows` and `Columns` set `/Tabs` to `/R` or `/C`; `Structure` sets `/S`, and needs a tagged file. `Custom` reorders the page's widgets within `/Annots`, keeping every other annotation where it was, and sets `/Tabs /A`, the PDF 2.0 annotations-array order, which Lamina PDF's reader also follows when it meets no `/Tabs`. A file that claims PDF/UA-1 needs `/S` on every page with annotations (ISO 14289-1 §7.18.3), so there `Custom` fails with `tab-order-needs-structure` until E15b can reorder the `Form` elements (§10, question 6). A `Custom` list that misses or repeats a widget fails with `tab-order-invalid`.

### 2.4 Accessible labels and tagging

`tooltip` writes `/TU`, the name a screen reader says and other viewers show as a tooltip (§12.7.4.1). `CreateField` tags a field when the file is tagged and `tagged` is set, which is the web reader's default (P06's "Also tag the field for screen readers"). It wraps each widget in a `Form` structure element holding an `OBJR` to it, and gives the widget its `/StructParent` (ISO 32000-2 §14.8.4; ISO 14289-1 §7.18.4). The element goes after the element that owns the nearest content above the widget, as `annot/tags.rs` places comments, with that module's parent-tree code. `DeleteField` removes the elements. On an untagged file that claims PDF/UA-1, the conformance check reports `annot-not-tagged`, E4's code, since a widget is an annotation.

P06's "Use nearby text" belongs to the web reader: it reads the closest text run left of or above the widget through `laminapdf-extract` and fills the label input, which the user confirms. It never writes `/TU` by itself.

### 2.5 Automatic field detection

E9d adds `suggest_fields(page)` to `laminapdf-extract`, rules only, as ADR 0030 §6 requires of core crates. It finds ruling lines with `tables/edges.rs`, boxes from closed rules, runs of underscores or dots in text, and the label text beside each, then suggests a `FieldSuggestion { kind, rect, label, reason }`. Suggested kinds are text (one line or several), date (a label such as "Date"), check box (a small square), signature (a label such as "Signature") and, for neighbouring boxes with one question, a radio group. Areas that existing widgets cover are skipped. A suggestion is data, never an op: accepting one in P07 makes a `CreateField` (§10, question 1).

## 3. Conformance the edits must keep

- **Fields fill in every viewer.** A field Lamina PDF creates fills in pdf.js, PDFium and Acrobat Reader: it writes only entries those viewers read, and every widget has its appearances. E9b records a manual Acrobat Reader check per field kind; pdf.js and PDFium are checked in CI (§9).
- **Tagged PDF and PDF/UA-1.** New fields get `Form` elements in tagged files (§2.4). A field without `/TU` on a file that claims PDF/UA-1 raises a new warning, `field-label-missing`, from the save's conformance check, so P06's "1 field has no accessible label" and the library agree. Pages a field is added to get `/Tabs /S` in a tagged file when they have no tab order, as E4's annotation ops do.
- **PDF/A.** No authoring op writes a script, XFA or a true `/NeedAppearances` on a PDF/A file (ISO 19005-2 §6.4.1, §6.6). A new field's appearance uses an embedded font there: the first of the op's `fonts`, else the save warns `pdfa-font-not-embedded`, as fills do ([E8 plan §10](e8-form-filling.md#10-decisions-on-the-open-questions), question 12).
- **Nothing runs.** Behaviours are helper calls written from typed values, and authoring runs no script (ADR 0021 §1, §4).

## 4. Crate and package changes

- **`laminapdf-model`**: `FieldSpec`, `WidgetSpec`, `TextStyle`, `ButtonAction`, `LockSpec`, `SeedValue` and the name rules in `forms/` (E9b, E9f); `TerminalField::lock` and `TerminalField::seed` read back (E9f).
- **`laminapdf-ops`**: the ops of §2.1–§2.3 in `forms/`, a new `forms/tags.rs` for `Form` elements over the shared parent-tree code, `Applied::created_fields`, `Op::authored_fields()` and `Op::form_right()` for the planner (§5, E9k), and the helper-call writer beside E8f's recogniser (E9b, E9e–E9g).
- **`laminapdf-build`**: push button captions and empty signature boxes in `annot/appearance/field.rs` (E9b, E9f).
- **`laminapdf-save`**: the field-level checks of authoring ops and the `/Form` usage rights (E9k), recorded as ADR 0007 §13.
- **`laminapdf-extract`**: `suggest_fields` and `FieldSuggestion` in a new `forms` module, reusing `tables/edges.rs` (E9d).
- **The façade and bindings** (requirement 7, each in the PR that adds the op). Every op goes through `apply` on `laminapdf::Document` and as an edit in `@laminapdf/sdk`, with rustdoc or TSDoc, a runnable example, tsify types and a frozen sample in `fixtures/ops/v1/`. `laminapdf::Document` gains `suggest_fields`, and the SDK's `Document` gains `suggestFields`. E9b's example builds a sign-up form on an existing page; E9g's adds a total.
- **`@laminapdf/viewer`**: design style for widgets (dashed edge, tint, name) and tab-stop badges in the form layer E8e adds, with E4's `annotation-gesture.ts` for move and resize (E9c, E9h).
- **`@laminapdf/ui`**: the `formDesign` mode, its toolbar, the inspector and the Fields tab's design view (E9c); the behaviour sections, the Calculation order dialog and the tab-order view (E9h); the suggestions view (E9i). Snapping to a grid and to other fields' edges is shared with E6e: whichever lands first writes it.
- **Size.** Form design loads with the editor's lazy chunk; the web reader's startup scripts gain nothing. The ops join every WebAssembly module, the detector the extract module; E9b and E9d measure their growth against ADR 0016 §6's budgets.

## 5. Saving and the save constraint

Every authoring op is class `Document`, except a new signature field, which is class `Signature` (§2.2). DocMDP permits no `Document` change at any level: P1 allows no change, P2 form filling, page templates and signing, P3 also annotations (ISO 32000-2 §12.8.2.2). Approval signatures alone restrict only the fields their FieldMDP transforms and `/Lock` dictionaries lock (§12.8.2.4, §12.7.5.5).

**Authoring ops name their fields.** Today a class `Document` op is judged by class, and a lock allows it (§1). E9k, which lands before any authoring op, adds `Op::authored_fields()`: the field an op changes, when it exists, and every fully qualified name the op gives or takes away. A rename names both the old and the new name, and a create names the new one. The planner then holds each authored field as ADR 0007 §12 holds a filled one:

- every lock that covers one of its names, or a field below it, is `Restriction::FieldMdp`, by §11's coverage rules; an `/Include` lock also covers a field above one it lists, since renaming or deleting that field renames or deletes the listed one. Recommendation: `/All` and `/Exclude` cover a field created after signing too. ISO 32000-2 §12.8.2.4 locks the fields as they were at signing, so this reading is stricter than the standard, as ADR 0007 §11's is for page ops; a new field beside a locked form looks to a reader like part of what was signed;
- a change to a signed signature field or one of its widgets, and creating, renaming or deleting a field that shares its name or sits above or below it, is `Restriction::SignedWidget`. Recommendation: this goes beyond ISO 32000-2 §12.8, which keeps an approval signature valid after such a change, since a renamed parent or a new child changes what a reader sees as signed. A field whose name can't be read counts too;
- `SetCalculationOrder` and `SetTabOrder` name no field and keep the class rule, since neither changes a field's definition or value.

E9k records this as ADR 0007 §13, with `would_violate(&op)` giving the same verdict before the op applies. It also teaches the planner UR3's `/Form` `/Add` and `/Delete` rights, since today it reads only `/FillIn`, and tests every rule with `test_support::Declared` stand-ins, as E2r did before the page ops shipped.

| Constraint | When | What Save does | What the user sees |
| --- | --- | --- | --- |
| `Free` | No signature, a sound base | An incremental update | Nothing; after a delete, the note that earlier versions keep the old field |
| `AppendOnly`: approval signatures | Signed, not certified | An incremental update for every op that touches no locked or signed field; the signatures stay valid under §12.8 | The status-bar note "Signed file. Saving adds your changes after the signature." (§10, question 2) |
| `AppendOnly` with a FieldMDP lock or `/Lock` | Some fields locked | An incremental update. Creating, changing, renaming, moving or deleting a field a lock covers, by any name it has or gets, is a violation (`Restriction::FieldMdp`) | Locked fields draw with a lock in design style; changing one opens P05's conflict dialog |
| `AppendOnly` with a signed signature field | Any signature | Changing, moving or deleting the signed field or its widgets is a violation (`Restriction::SignedWidget`); unsigned signature fields are ordinary fields | The same dialog, naming the signer |
| `AppendOnly` with DocMDP P1, P2 or P3 | Certified | Every authoring op is a violation, except that P2 and P3 allow a new signature field (§2.2); the session becomes `Conflict` at the first change. A PDF 2.0 `/Lock` with `/P` counts as DocMDP | Entering Form design changes nothing; the first change opens the conflict dialog ("the certification will no longer be valid"); P1 shows the banner on entering, as in P01 |
| Usage rights (UR3) | Extended by Adobe Reader rights | `CreateField` needs `/Form [/Add]` and `DeleteField` `/Form [/Delete]` (§12.8.2.3, Table 260), checked through `Op::form_right()`; every other authoring op is a violation (Table 260 grants no right to change a field) | The note, or the banner when the rights don't cover the change |
| `RequiresRewrite` | A repaired file | A full rewrite | A neutral strip saying why |
| `RequiresRewrite`: encrypted base | Password-protected | Save is refused (`encrypted-base`) until E11 | The warning strip |
| `Conflict` | Ops the constraint forbids | Save is refused; Apply anyway… invalidates the named signatures after a confirmation (`with_invalidate_signatures`, ADR 0007 §3) | The danger strip naming each signature, with Save a copy… |

This replaces [full-editor §E9](../full-editor.md#e9-form-authoring)'s "blocked unless signatures are removed", which P05 already marked as superseded. Lamina PDF saves authoring on a signed file after the override, and the save invalidates exactly the signatures it names.

`suggest_fields` changes nothing and works on any document.

## 6. Differences between the shells

Form authoring is the same on both shells, and no server feature is involved: detection runs on the device, and the status bar says so (P07).

| Area | Flag | Web | Desktop |
| --- | --- | --- | --- |
| Fonts for a field's appearance | `systemFonts` (E6h) | Bundled fonts and font files the user adds | Also fonts on the computer, read by the engine and passed in as bytes |
| Handles | none: pointer type | 8 px handles in 24 px hit areas; 44 px on a tablet (P05) | The same |
| Phone | none: breakpoints | Not offered: "Form design needs a larger screen" | |

## 7. Todos

Each todo is one reviewable pull request under ~1,500 changed lines with tests, on the [todo template](../../todos/TEMPLATE.md). The core todos flip the `core` and `pdf` columns, the interface todos `web`, and E9j `desktop`, as in E8. The round started with E9b–E9d. This plan splits them:

- E9k takes the planner's checks of authoring ops, so they land before any op;
- E9e takes E9b's configuration ops, and E9f its signature fields;
- E9g takes behaviour, calculation order and tab order;
- E9h takes P06 and P05's behaviour sections out of E9c;
- E9i takes P07's interface out of E9d;
- E9j is new, for the desktop.

| ID | Todo | Depends on |
| --- | --- | --- |
| [E9k](../../todos/done/E9k-authoring-save-checks.md) | The planner's checks of authoring ops: `Op::authored_fields()`, `Op::form_right()`, UR3's `/Form` rights, ADR 0007 §13 | E9a |
| [E9b](../../todos/ready/E9b-field-creation-ops.md) | `CreateField` and `DeleteField` for every field type but signatures, names, push button captions, `Form` elements, `pdfa-field-actions` | E8d, E9k |
| [E9e](../../todos/ready/E9e-field-configuration-ops.md) | `UpdateFieldDef`, `RenameField`, `MoveWidget`, `AddWidget`, `RemoveWidget`; flips E.01–E.04 core and `pdf` | E9b |
| [E9f](../../todos/ready/E9f-signature-fields.md) | Signature fields: `FieldSpec::Signature`, `/Lock`, `/SV`, `/SigFlags`; flips E.15 core and `pdf` | E9e |
| [E9g](../../todos/ready/E9g-behaviour-and-order-ops.md) | `SetFieldBehaviour` and the helper-call writer, `SetCalculationOrder`, `SetTabOrder`; E.07–E.09 core and `pdf` | E8f, E9b |
| [E9c](../../todos/ready/E9c-form-design-mode.md) | P05: the `formDesign` mode, field tools, move and resize, the inspector, signature field settings; flips E.01–E.04 and E.15 `web` | E8e, E9e, E9f |
| [E9h](../../todos/ready/E9h-behaviour-and-tab-order-ui.md) | P05's Format, Validation and Calculation sections and Calculation order dialog; P06's tab-order view and labels; flips E.07–E.09 `web` | E9c, E9g |
| [E9d](../../todos/ready/E9d-automatic-field-detection.md) | `suggest_fields` in `laminapdf-extract`, the labelled set and its precision and recall | E7c, E8g, E9a |
| [E9i](../../todos/ready/E9i-field-detection-review-ui.md) | P07: the suggestions view, Accept, Reject and Group as one radio group | E9c, E9d |
| [E9j](../../todos/ready/E9j-desktop-form-design.md) | Desktop: form design in the engine, system fonts, WebdriverIO suites, the Acrobat checks; the round's close | E8m, E9h, E9i |

E9k starts first, so no authoring op can ship without its checks, and E9b follows it. E9e and E9g then run side by side, E9f follows E9e, and E9d can start once E8g can flatten its labelled forms, since it creates nothing. The interface todos wait on E8e's form layer. E8e hasn't started, so it may be the round's longest wait.

## 8. Fixtures

Lamina PDF's own scripts write every new fixture, licensed like the rest of Lamina PDF (`MIT OR Apache-2.0`), with provenance in `fixtures/README.md`.

- **`fixtures/forms/`** (`tools/fixtures/forms.py`):
  - `blank-page.pdf` (a page with printed labels and no form, for creating fields; E9b)
  - `nested-names.pdf` (non-terminal fields three deep, a name with a terminal parent, merged and separate widgets; E9b)
  - `tagged-form.pdf` (PDF/UA-1, `Form` elements, `/Tabs /S`), which E8 planned and E9b adds if E8 hasn't
  - `behaviours.pdf` (one field per helper, an unhandled script on a trigger, a `/CO` of three; E9g)
- **`fixtures/forms/flat/`** (E9d): the labelled set. `tools/fixtures/flat_forms.py` flattens fillable forms with `FlattenForm` (E8g) and writes each field's kind and rectangle to a `.labels.json` beside the flat file, so the labels come from the original fields. The forms are the script's own, plus US federal forms with no XFA, which are in the public domain (17 U.S.C. §105): the SBA Form 413 already in `fixtures/forms/`, and at least four more that E9d picks and records with URL, date and hash.
- **Signed files** (`fixtures/signed/make.py`, pyHanko, MIT): the existing `approval.pdf`, `certified-p1`, `-p2`, `-p3`, `locked.pdf`, `form-locked-include.pdf` and `usage-rights.pdf`, plus `usage-rights-form-add.pdf` (UR3 granting `/Form [/Add /FillIn]`) and `usage-rights-form-delete.pdf` (`/Form [/Delete /FillIn]`), each also granting `/Signature [/Modify]`, and `form-locked-exclude.pdf` (an `/Exclude` lock), which E9k adds, and `signature-field-locked.pdf` (an unsigned field with `/Lock` and `/SV`), which E9f adds.
- **The pinned corpus** (nightly, never copied): the E8 plan's form files, reused to create, rename and delete fields in forms Lamina PDF didn't make.

## 9. Tests

Each op gets the tests its todo lists ([ADR 0018](../../decisions/0018-test-strategy.md)). "Round-trip" means Lamina PDF, pdf.js, PDFium and qpdf (`--check` clean) each reopen the saved file and report the change and nothing else.

| Op or function | Unit | Golden | Round-trip | Differential | Fuzz | e2e |
| --- | --- | --- | --- | --- | --- | --- |
| `CreateField`, `DeleteField` (E9b) | each kind; names, clashes, nested parents; a new `/AcroForm`; tagging | `fixtures/golden/forms/authoring/` per kind | pdf.js and PDFium list each new field with its type and fill it through their form APIs; the filled value reopens in Lamina PDF | names and types of created fields vs pypdf and pdf.js (E8o's differential gains the authored files) | `ops_json` gains the ops | — |
| `UpdateFieldDef`, `RenameField`, `MoveWidget`, `AddWidget`, `RemoveWidget` (E9e) | each entry set and removed; renames across parents; radio options | per op on `nested-names.pdf` | pdf.js and PDFium read the changed definition | the same differential | `ops_json` | — |
| Signature fields (E9f) | `/Lock`, `/SV`, `/SigFlags`, invisible fields | `signature-field-locked.pdf` | pyHanko signs the created field and validates it | pyHanko reads the `/Lock` and `/SV` written | `ops_json` | — |
| `SetFieldBehaviour` (E9g) | write then recognise gives the same behaviour, for every helper and parameter | the `/AA` of `behaviours.pdf` | pdf.js formats, validates and calculates the written fields as Lamina PDF does | displayed values vs pdf.js on `behaviours.pdf` | an `af_write` target: any behaviour written is recognised back | — |
| `SetCalculationOrder`, `SetTabOrder` (E9g) | permutations, refusals, each `/Tabs` value, PDF/UA-1 refusal | — | `tab_order` and `/CO` read back | — | `ops_json` | — |
| `suggest_fields` (E9d) | rules on synthetic pages | — | — | precision and recall on `fixtures/forms/flat/` (§10, question 1) | a `field_suggest` target over corpus pages | — |
| Interface (E9c, E9h, E9i) | Vitest for the mode's state, the inspector and the tab-order list | — | — | — | — | Playwright: design a form with every field type, save, fill it in pdf.js; axe on each screen |
| Desktop (E9j) | — | — | — | — | — | WebdriverIO on macOS, Windows and Linux |

Across every op:

- apply then undo leaves the document overlay empty and the next save byte-identical (ADR 0006 §3);
- redo re-applies the same bytes;
- the same state saves to the same bytes on native and wasm32.

`would_violate` on every signed fixture gives §5's verdict, asserted per op and per lock coverage, because a misclassified op is a security bug. veraPDF reports nothing new on `tagged-form.pdf` and `pdfa-form.pdf` after each op. A new fuzz target's `fixtures/regressions/<target>` folder joins `laminapdf-model`'s regression guard list.

## 10. Decisions on the open questions

The plan's recommendations, unless a decision names the owner.

1. **Automatic field detection: heuristic only, or an optional ML model?** Heuristic only, in `laminapdf-extract`. ADR 0030 §6 keeps models out of core crates, and its `LayoutModel` slot labels no form fields, so a model would need its own trait, weights with a permissive licence, and a plug-in shipped beside the app. The heuristics cover the common flat form: ruled lines, boxes and underscores beside labels. E9d's targets on the labelled set are a precision of at least 0.85 and a recall of at least 0.70, counting a suggestion as right when its kind matches and its box overlaps the original field's by an intersection over union of 0.5 or more. Precision ranks first because every wrong suggestion costs the user a Reject. A model comes back only as an ADR 0030 amendment, if E9d measures the rules below those targets on forms users bring.
2. **Authoring under approval signatures.** `AppendOnly`, as ISO 32000-2 §12.8 and ADR 0007's class rules allow, with locked and signed fields held to §5. Other validators may still flag a new field after a signature as a change. E9b (with E9k's checks in place) runs pyHanko's difference analysis on `approval.pdf` after `CreateField` and records the verdict here, as E8f did for `SetFieldHidden`. If pyHanko flags it, the status note adds "Some apps may report that the file changed after signing."
3. **Signed files: blocked, or saved after the override?** Saved after the override (§5), as P05 draws and the todos already said. full-editor §E9 is updated to match in this PR.
4. **A date field.** A text field with a date format (`AFDate_FormatEx`), shown as its own tool because people look for one (P05). PDF has no date field type.
5. **Which actions a push button gets in E9.** ResetForm and SubmitForm only (§2.1). Every other action, JavaScript included, arrives with E17b's action editor (ADR 0021 §4).
6. **A custom tab order in a tagged file.** On a file that claims PDF/UA-1, only `Structure` order is offered, since ISO 14289-1 §7.18.3 requires it, and a custom order there waits for E15b's structure-tree editing, which can move the `Form` elements. **Owner: the E15a plan, round E15.** A tagged file without the claim may take a custom order (`/Tabs /A`), and P06 says that screen readers follow the structure order instead.
7. **Who flips L.06.** E15c, in round E15, as its row says (§2). E9 writes `/TU` and `Form` elements for the fields it creates and tags those tests at `write`.
8. **Where the round's desktop columns flip.** In a todo of their own, E9j, as E8m does for E8, instead of in E9c, so each interface todo stays one reviewable PR.
9. **The plan's location:** `docs/plans/editor/`, as E1–E8 use.
10. **A new signature field on a certified file.** Allowed under P2 and P3 as class `Signature` (§2.2), since Table 257 allows signing there, pyHanko accepts the new field, and Acrobat accepts at least an invisible one; E9f confirms it for visible fields and narrows it to invisible ones if pyHanko refuses.

The round then builds the field ops and their save rules first, configuration, signatures and behaviour beside each other, and the interface last, so a form can be authored in the library before the first screen lands.
