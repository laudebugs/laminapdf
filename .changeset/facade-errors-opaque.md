---
"@laminapdf/sdk": minor
---

Breaking (Rust, while alpha): `laminapdf::Error` no longer exposes the errors of the crates below it. The `Cos`, `Ops`, `Save`, `Text` and `Build` variants are one opaque `Error::Core`, and `CosError`, `OpsError`, `SaveError`, `SinkError` and `TextError` are no longer re-exported. Match on `error_code()` instead; a `data-pending` error's ranges come from the new `Error::pending_ranges()`, which replaces `laminapdf_source::pending_ranges`. The JavaScript API and every error's code, message and JSON are unchanged.
