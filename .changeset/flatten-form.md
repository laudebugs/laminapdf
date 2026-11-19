---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

You can now flatten a filled form. `doc.flattenForm(undefined, "print")` (`flatten_form` in Rust) draws every field's answer into its page and removes the field, so the copy shows the same in every reader and the answers can no longer be changed. Pass field IDs from `formFields()` to flatten only those fields. One undo brings the fields back.

Signature fields are never flattened, and neither are fields a signature locks when you leave the IDs out. A push button with no picture of its own stays a field, and `warnings` lists it as `flatten-no-appearance`. A form's XFA part goes too, and `warnings` says so with `xfa-removed` when some fields are left. The `flattenForm` op does the same for fields you name, in every module.
