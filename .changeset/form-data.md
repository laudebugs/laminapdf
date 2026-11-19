---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

You can now fill a form from data, move its answers to and from other apps, and reset it. `doc.fillForm({ fields: { "customer.name": "Ada", paid: true } })` (`fill_form` in Rust) fills each field by its full name in one change, so a single undo takes every answer back, and the form's calculations run once after the last answer. A check box takes `true` or `false`, a list an array of its options, and `null` empties a field.

What can't be filled is listed, and the rest is filled. The result names the fields the form doesn't have (`unknown`), the fields a signature locks (`locked`), and the values a field refuses (`refused`, each with its code, such as `field-value-invalid` for text that is too long).

`doc.exportFormData("json")` (`export_form_data`) writes the form's answers in the same shape `fillForm` takes, so an export fills a copy of the form as it was; its schema ships in the package's docs as `schemas/form-data.schema.json`. `"xfdf"` and `"fdf"` write the formats Acrobat and most PDF apps import, and `doc.importFormData(bytes)` (`import_form_data`) reads all three. `doc.resetFields()` (`reset_fields`) gives every field its default value, or empties it, as a form's Reset button does.

XFDF and FDF files from `exportAnnotations` and other apps now carry form fields too. Comment import still leaves them out and lists them, so form data and comments stay separate.
