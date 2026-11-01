---
"@laminapdf/render": minor
---

A render `data-pending` thrown inside `@laminapdf/sdk`'s worker now reaches the page with its byte ranges: `LaminaError.ranges` lists them. Before, they arrived empty. `RenderError` has a new `toDiagnostic()`, which returns the plain error object every lamina surface uses, and the worker posts it. `toJSON()`, `reviveRenderError` and `SerializedRenderError` are gone. The viewer's worker now throws `RenderError`s for `cancelled`, `disposed` and a missing backend (`render-backend-unavailable`) instead of plain objects. Errors thrown by the hayro module now carry a `hint` and a `docsUrl`.
