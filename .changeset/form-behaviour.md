---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Forms now format, check and add up their values as they do in Acrobat, without running any JavaScript. Most form scripts only call Acrobat's built-in helpers. Lamina PDF reads those calls as data and does their work itself: a number field shows `$1,234.50` while its value stays `1234.5`, a date field shows its date in the form's pattern, and a total updates as soon as a field it adds up changes.

`setFieldValue` now refuses text that breaks a field's format or range, with `field-value-invalid` and the reason `format` or `range`. The same edit updates every total that depends on the changed value, so one undo reverts the value and its totals. A total that a signature locks keeps its value, and the edit lists it in `skippedCalculations`. The new `setFieldHidden` edit hides a field or shows it again, as a form's Hide button does.

`doc.fieldBehaviour(id)` says what a field's helpers do, `doc.validateField(id, text)` checks a value before you set it, and `doc.calculate()` brings stale totals up to date. `doc.formScripts()` lists every script in a file, never its text, each marked `handled`, `not handled` or `xfa`. The new `maxScriptLength` limit (64 KiB) caps the bytes of one script Lamina PDF reads. A longer script counts as not handled.
