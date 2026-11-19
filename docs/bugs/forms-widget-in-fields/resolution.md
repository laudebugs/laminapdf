# Resolution: a widget listed in /Fields stands for its field

[Diagnosis](detail.md).

## The decision

The form view now reads a widget that `/Fields` lists as the field its `/Parent` names, as qpdf and pdf.js do. ISO 32000-2 §12.7.4 asks `/Fields` for the root fields, so the file is wrong, but readers are tolerant (ADR 0003) and the writer's intent is plain. The rule covers an entry that is a widget annotation with a `/Parent` and neither `/T` nor `/FT`, and only when that parent has `/T` or `/FT`, a field's own keys (Table 226). An entry with a name or type of its own still goes through the walk as the field it says it is, and a `/Parent` that names something else, such as the page tree, leaves the entry the untyped node it was.

## The fix

`roots` in `crates/laminapdf-model/src/forms/read.rs` builds the walk's starting nodes from `/Fields`. For a widget entry, it climbs the `/Parent` chain with `parent_chain`, which visits each object once, so a loop ends the climb, and it stops at `Limits::max_tree_depth`. The widget's parent becomes the node the walk starts from. It inherits from the fields above it on the chain, so a two-level chain gives `order.veg` with the root's `/FT`. A field is placed once however many of its widgets are listed, and a field `/Fields` also names directly keeps its first place. The listed widgets are kept by their field's object number. When the walk reads that field as a terminal field, it adds them after its `/Kids`, skipping any widget the walk has already reached.

The change leaves the reconciliation's rules alone, and whatever is still left out shows up in it:

- A chain that reaches no field type reports the parent field in `Form::untyped`, where the old reader reported the widget.
- A widget whose parent has field kids, which §12.7.4 doesn't allow, is not attached; a page that lists it makes it an orphan widget with that parent as its field.
- A widget with no `/Parent`, or one that is its own parent, stays the untyped node it was.

The live differential's `form-facts` example marks a field reached through such an entry as below an unnamed node (`unnamed_ancestor`), as the view now reads it. pypdf's `get_fields` skips an entry without `/T`, so the `unnamed-node` class excuses the field against pypdf, and its reason now says so. The `[[known]]` entry for `opt_demo.pdf` under #751 is gone.

## How it was checked

- `crates/laminapdf-model/tests/form_listed_widgets.rs` sweeps the shapes in eleven tests: one widget listed in place of its field, two widgets of one field, the field listed too (before and after its widget, with and without `/Kids`), a two-level chain with and without the tree reaching the field, three `/Parent` loops, a chain with no type, a widget with no parent, a widget whose parent has field kids, and the fixture. Under the old rule, ten of the eleven fail; the widget with no parent passes under both, as it should.
- `fixtures/forms/widget-entries.pdf` copies `opt_demo.pdf`'s shape, since the pinned file is in the external corpus only. `qpdf --json` reads `veg` with `/1` there as it does in `opt_demo.pdf`, and the form view now reads `veg` as a radio group at `/1`, which exports `Potato`, with both widgets and nothing to report.
- The model, ops, save and façade crates' tests pass, `form` reads `fields.pdf` and `reconcile.pdf` as before, and so `SetFieldValue`, the save planner's field names and the fill code see no change on them.
- `cargo xtask differential --live --forms` over the external corpus and `fixtures/forms/`: 2,068 files compared, 4,494 fields both sides list, 211 differences, none unexcused, and no stale entry. In `opt_demo.pdf`, `veg` now pairs with pdf.js's `veg`, whose `Potato` the `opt-export` class matches to `/1`, and the `unnamed-node` class excuses it against pypdf. The new fixture gives the same two rows.

## Left for later

Other walks of `/Fields` (`Document::field_names` and `field_by_name`, the disclosure report, the form scripts, the reference graph and the save planner's signature detection) read the tree on their own and still skip a widget entry. Giving the form one reading is [#768](https://github.com/laudebugs/laminapdf/issues/768).
