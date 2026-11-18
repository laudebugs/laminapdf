---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

You can now read a form's fields and fill them in. `doc.formFields()` (`form` in Rust) lists every field with its name, kind, value, default value, options and flags, and its widgets with their page and box. It also lists widgets that no field reaches and fields that no page shows. `doc.tabOrder(index)` (`tab_order` in Rust) gives the order Tab visits a page's widgets in, as the page's `/Tabs` asks.

The new `setFieldValue` op, applied with `doc.apply`, gives a text field, check box, radio group, list or combo box a value, or clears it. It keeps each button's state and a list box's chosen options in step. A value that doesn't fit the field fails with `field-value-invalid` and says why, a signature field with `field-is-signature`, and an unknown field with `unknown-field`. The widgets keep the appearance they had until a later release draws the new value.

On a signed form, a signature's field lock now counts only against the fields it covers: filling a field outside the lock saves as an incremental update that keeps the signature valid.
