# A field listed in /Fields through its widget was dropped

[Resolution](resolution.md). Issue [#751](https://github.com/laudebugs/laminapdf/issues/751).

## What the differential saw

The live form differential (E8o) found one field that Lamina PDF's form view (`Document::form`, E8c) left out over the pinned corpus: `veg` in pdf.js's `test/pdfs/opt_demo.pdf`. pdf.js and qpdf read `veg` as a radio group set to `/1`, which its `/Opt` exports as `Potato`. Lamina PDF listed no field of that name, and the differential excused the file with a `[[known]]` entry until the reader was fixed.

## The file's shape

The form's `/Fields` lists six objects. Four are fields; the last two, objects 10 and 11, are widget annotations with no `/T` and no `/FT`. Each widget's `/Parent` is object 26, the field: `/FT /Btn`, `/Ff 49152` (Radio and NoToggleToOff), `/T (veg)`, `/V /1`, `/Opt [(Carrot) (Potato)]`. Object 26 has no `/Kids`, so nothing in the tree from `/Fields` down leads to it.

ISO 32000-2 §12.7.4 says `/Fields` holds the document's root fields, so the file is malformed. qpdf (`qpdf --json --json-key=acroform`) and pdf.js follow each widget's `/Parent` and report `veg` once per widget. pypdf's `get_fields` stops at an entry without `/T` and lists nothing for it.

## How it reproduces

The pinned file is in the external corpus only (`cargo xtask corpus fetch`), so `fixtures/forms/widget-entries.pdf` copies its shape: two widgets in `/Fields`, their field a radio group with `/Opt` and no `/Kids`. On that fixture the old reader gave no `veg` and reported both widgets in `Form::untyped`, as if they were fields without a type.

## The cause

The form view walks `/Fields` down `/Kids` and reads each entry as a field. A widget listed there became a terminal field merged with itself: it inherited nothing, because the walk takes inherited values from the nodes above it in the tree, and the widget had no `/FT` of its own. The view therefore left it out of `Form::fields` and listed it in `Form::untyped`. The real field was never reached, so its name, type and value were lost. The untyped report named the widget's object, and the differential's `form-facts` example doesn't surface `untyped`, so nothing showed the loss.
