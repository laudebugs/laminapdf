---
"@laminapdf/viewer": patch
---

The viewer can now fill in a PDF form. `new FormLayer(viewer, options)`, added with `viewer.addLayer`, puts a native control over each form field's widget: a text box, a check box or radio button, or a list. `layer.setForm(fields, access)` gives it the fields to show, and `options.onCommit(field, value)` hands over each field's new value once its visit ends, as one undo step. A field can be read-only, locked by a signature (it asks `options.guard` before its first change) or guarded on a certified file (it asks before the first change too). The control being filled keeps what was typed when another field's edit redraws the page.

The document Web Worker reads the form for it, as `client.formFields()`, and the order Tab visits a page's fields in, as `client.tabOrder(index)`.
