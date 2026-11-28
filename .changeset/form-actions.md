---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Read what a form's buttons do and build the request a submit would make. `doc.formActions(id)` returns a field's `/A` and `/AA` actions through `/Next` as typed data (go to a page, a named page move, a link, a form reset, hide and show, a submit, and the actions the reader never runs: a script, a launch, a file import and an action on another file). `doc.submitRequest(id, index)` builds a `SubmitForm` action's request without sending it: the address, the format (HTML form encoding, FDF or XFDF), the body, every field in it and the required fields still empty. It refuses a send of the whole PDF (`submit-pdf-refused`) and an address that isn't `https:` (`submit-url-refused`); a `mailto:` address is a request that carries no data. In the Rust library, `Document::form_actions` and `Document::submit_request` do the same.
